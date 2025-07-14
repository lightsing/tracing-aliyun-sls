//! A reporter for batching and sending logs to the SLS service.
use crate::{Log, LogGroupMetadata, SlsClient};
use async_channel::{Receiver, Sender};
use futures_util::{FutureExt, join, select};
use std::{
    collections::HashMap,
    future::pending,
    pin::Pin,
    sync::{Arc, Mutex, atomic, atomic::AtomicBool},
};

type Item = (Arc<LogGroupMetadata>, Log);
pub(crate) type Producer = Sender<Item>;
type Consumer = Receiver<Item>;

const LOG_VEC_DEFAULT_CAPACITY: usize = 1024;
const VEC_POOL_DEFAULT_CAPACITY: usize = 1024;
const LOG_GROUP_DEFAULT_CAPACITY: usize = 1024;

/// Trait for creating a drain timer future.
pub trait DrainTimer: Send + Sync + 'static {
    /// Create a drain timer future.
    fn drain_timer(&self) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>;
}

/// A reporter for batching and sending logs to the SLS service.
#[derive(Clone)]
pub struct Reporter {
    state: Arc<State>,
    pub(crate) producer: Arc<Producer>,
    consumer: Arc<Mutex<Option<Consumer>>>,
    client: SlsClient,
}

/// Reporting is a handle to the reporting process, allowing configuration and starting the reporting.
pub struct Reporting {
    state: Arc<State>,
    consumer: Consumer,
    client: SlsClient,

    log_vec_capacity: usize,
    log_group_capacity: usize,
    vec_pool_capacity: usize,

    drain_timer: Box<dyn DrainTimer>,
    shutdown_signal: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
}

struct LogConsumer {
    consumer: Consumer,
    client: SlsClient,
    vec_pool: Vec<Vec<Log>>,
    log_group: HashMap<Arc<LogGroupMetadata>, Vec<Log>>,

    log_vec_capacity: usize,
    log_group_capacity: usize,
    vec_pool_capacity: usize,
}

struct State {
    is_reporting: AtomicBool,
    is_closing: AtomicBool,
}

impl Reporter {
    /// Create a new reporter with the given SLS client.
    pub fn from_client(client: SlsClient) -> Self {
        let (producer, consumer) = async_channel::unbounded();
        Self {
            state: Arc::new(State::default()),
            producer: Arc::new(producer),
            consumer: Arc::new(Mutex::new(Some(consumer))),
            client,
        }
    }

    /// Create the reporting future with a given drain timer.
    ///
    /// If the reporter is already in reporting state, it returns `None`.
    pub async fn reporting(&self, drain_timer: impl DrainTimer) -> Option<Reporting> {
        if self.state.set_reporting() {
            return None;
        }

        let consumer = self.consumer.lock().unwrap().take()?;

        Some(Reporting {
            state: self.state.clone(),
            consumer,
            client: self.client.clone(),

            log_vec_capacity: LOG_VEC_DEFAULT_CAPACITY,
            log_group_capacity: LOG_GROUP_DEFAULT_CAPACITY,
            vec_pool_capacity: VEC_POOL_DEFAULT_CAPACITY,

            drain_timer: Box::new(drain_timer),
            shutdown_signal: Box::pin(pending()),
        })
    }

    /// Report a log to the reporter.
    pub fn report(&self, metadata: Arc<LogGroupMetadata>, log: Log) {
        if !self.state.is_closing() {
            if let Err(e) = self.producer.send_blocking((metadata, log)) {
                tracing::error!("reporter send error: {e}");
            }
        }
    }
}

impl Reporting {
    /// Quit when shutdown_signal received.
    ///
    /// Accept a `shutdown_signal` argument as a graceful shutdown signal.
    pub fn with_graceful_shutdown(
        mut self,
        shutdown_signal: impl Future<Output = ()> + Send + Sync + 'static,
    ) -> Self {
        self.shutdown_signal = Box::pin(shutdown_signal);
        self
    }

    /// Set the initial batching log vector capacity.
    ///
    /// Default is `1024`.
    pub fn with_log_vec_capacity(mut self, capacity: usize) -> Self {
        self.log_vec_capacity = capacity;
        self
    }

    /// Set the initial log group capacity.
    ///
    /// Default is `1024`.
    pub fn with_log_group_capacity(mut self, capacity: usize) -> Self {
        self.log_group_capacity = capacity;
        self
    }

    /// Set the initial internal vector pool capacity.
    ///
    /// The vector pool is used to reuse log vectors to reduce allocations.
    pub fn with_vec_pool_capacity(mut self, capacity: usize) -> Self {
        self.vec_pool_capacity = capacity;
        self
    }

    /// Start the reporting process.
    pub async fn start(self) {
        let (shutdown_tx, shutdown_rx) = async_channel::bounded::<()>(1);

        let Reporting {
            state,
            consumer,
            client,
            drain_timer,
            shutdown_signal,
            log_vec_capacity,
            log_group_capacity,
            vec_pool_capacity,
        } = self;

        let mut vec_pool = Vec::with_capacity(vec_pool_capacity);
        vec_pool.resize_with(vec_pool_capacity, || Vec::with_capacity(log_vec_capacity));

        let log_group = HashMap::with_capacity(log_group_capacity);

        let mut consumer = LogConsumer {
            consumer,
            client,
            vec_pool,
            log_group,

            log_vec_capacity,
            log_group_capacity,
            vec_pool_capacity,
        };

        let work_fut = async move {
            let mut drain_fut = drain_timer.drain_timer().fuse();
            loop {
                select! {
                    _ = consumer.consume().fuse() => continue,
                    _ = drain_fut => {
                        consumer.drain().await;
                        drain_fut = drain_timer.drain_timer().fuse();
                    },
                    _ = shutdown_rx.recv().fuse() => {
                        break
                    },
                }
            }
            consumer.drain().await;
            state.is_closing.store(true, atomic::Ordering::Relaxed);
        };

        let shutdown_fut = async move {
            shutdown_signal.await;
            shutdown_tx.send_blocking(()).ok();
        };

        join!(work_fut, shutdown_fut);
    }
}

impl LogConsumer {
    async fn consume(&mut self) {
        let Ok((meta, log)) = self.consumer.recv().await else {
            return;
        };

        self.log_group
            .entry(meta)
            .or_insert_with(|| {
                self.vec_pool
                    .pop()
                    .unwrap_or_else(|| Vec::with_capacity(self.log_vec_capacity))
            })
            .push(log);
    }

    async fn drain(&mut self) {
        for (meta, mut log) in self.log_group.drain() {
            self.client.put_log(&meta, &log).await;
            log.clear();
            log.shrink_to(self.log_vec_capacity);
            self.vec_pool.push(log);
        }
        self.log_group.shrink_to(self.log_group_capacity);
        self.vec_pool.truncate(self.vec_pool_capacity);
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_reporting: AtomicBool::new(false),
            is_closing: AtomicBool::new(false),
        }
    }
}

impl State {
    fn set_reporting(&self) -> bool {
        self.is_reporting.swap(true, atomic::Ordering::Relaxed)
    }

    fn is_closing(&self) -> bool {
        self.is_closing.load(atomic::Ordering::Relaxed)
    }
}

impl<F, Fut> DrainTimer for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + Sync + 'static,
{
    fn drain_timer(&self) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        Box::pin(self())
    }
}
