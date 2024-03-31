use crate::client::SlsClient;
use crate::proto::{KeyValue, Log, LogGroup};
use chrono::Utc;
use std::collections::HashMap;
use std::fmt::Debug;
use tokio::{select, sync::mpsc};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Record},
    Event, Id, Metadata, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// A layer that collects logs and sends them to Aliyun SLS.
pub struct SlsLayer {
    pub(crate) max_level: tracing::Level,
    pub(crate) sender: mpsc::Sender<(Vec<KeyValue<'static>>, Log<'static>)>,
}

impl<S> Layer<S> for SlsLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _: Context<'_, S>) -> bool {
        metadata.level() <= &self.max_level
    }
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found, this is a bug");

        let mut tags: Vec<KeyValue<'static>> = Vec::with_capacity(16);
        tags.push(KeyValue::new("name", span.name()));
        let metadata = attrs.metadata();
        tags.push(KeyValue::new("target", metadata.target()));
        if let Some(file) = metadata.file() {
            tags.push(KeyValue::new("file", file));
        }
        if let Some(line) = metadata.line() {
            tags.push(KeyValue::new("line", line.to_string()));
        }

        attrs.record(&mut KeyValueVisitor { kvs: &mut tags });

        span.extensions_mut().insert(tags);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("unknown span");
        let mut exts = span.extensions_mut();
        let tags = exts.get_mut::<Vec<KeyValue>>().expect("no tags found");
        values.record(&mut KeyValueVisitor { kvs: tags });
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let time = Utc::now();

        let mut tags = Vec::with_capacity(16);
        for span in ctx
            .lookup_current()
            .into_iter()
            .flat_map(|span| span.scope().from_root())
        {
            let exts = span.extensions();
            let span_tags = exts.get::<Vec<KeyValue>>().expect("missing fields");
            tags.extend_from_slice(span_tags);
        }

        let mut contents = Vec::with_capacity(16);

        let metadata = event.metadata();
        contents.push(KeyValue::new("level", metadata.level().as_str()));
        contents.push(KeyValue::new("name", metadata.name()));
        contents.push(KeyValue::new("target", metadata.target().to_string()));
        if let Some(file) = metadata.file() {
            contents.push(KeyValue::new("file", file));
        }
        if let Some(line) = metadata.line() {
            contents.push(KeyValue::new("line", line.to_string()));
        }
        event.record(&mut KeyValueVisitor { kvs: &mut contents });

        let log = Log {
            time: time.timestamp() as u32,
            time_ns: Some(time.timestamp_subsec_nanos()),
            contents,
        };
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let _ = sender.send((tags, log)).await;
        });
    }
}

struct KeyValueVisitor<'a> {
    kvs: &'a mut Vec<KeyValue<'static>>,
}

impl<'a> Visit for KeyValueVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.kvs
            .push(KeyValue::new(field.name(), value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.kvs
            .push(KeyValue::new(field.name(), format!("{:?}", value)));
    }
}

pub struct SlsDispatcher {
    pub(crate) receiver: mpsc::Receiver<(Vec<KeyValue<'static>>, Log<'static>)>,
    pub(crate) client: SlsClient,
    pub(crate) buffer: HashMap<Vec<KeyValue<'static>>, Vec<Log<'static>>>,
    pub(crate) drain_timeout: std::time::Duration,
    pub(crate) shutdown: mpsc::Receiver<()>,
}

// MAX_SINGLE_SIZE is the maximum size of a single log group, 10MB
const MAX_SINGLE_SIZE: usize = 10 * 1024 * 1024;

impl SlsDispatcher {
    pub async fn run(&mut self) {
        loop {
            select! {
                _ = self.shutdown.recv() => {
                    break;
                },
                _ = tokio::time::sleep(self.drain_timeout) => {
                    if self.buffer.is_empty() {
                        continue;
                    }
                    let tags = self.buffer.iter().max_by_key(|(_, logs)| logs.len()).unwrap().0.clone();
                    let logs = self.buffer.remove(&tags).unwrap();
                    let _ = self.client.put_log(&LogGroup {
                        logs,
                        reserved: None,
                        topic: None,
                        source: None,
                        log_tags: tags,
                    }).await;
                },
                e = self.receiver.recv() => {
                    if e.is_none() {
                        break;
                    }
                    let (tags, log) = e.unwrap();
                    let logs = self.buffer.entry(tags.clone()).or_default();
                    let size_before = LogGroup::estimate_size(logs, &tags);
                    assert!(size_before <= MAX_SINGLE_SIZE, "log group size exceeds limit");
                    logs.push(log);
                    let size_after = LogGroup::estimate_size(logs, &tags);
                    if size_after > MAX_SINGLE_SIZE {
                        let (tags_removed, mut logs) = self.buffer.remove_entry(&tags).unwrap();
                        // pop the last log
                        let last_log = logs.pop().unwrap();
                        let _ = self.client.put_log(&LogGroup {
                            logs,
                            reserved: None,
                            topic: None,
                            source: None,
                            log_tags: tags_removed,
                        }).await;
                        // put the last log back
                        let new_logs = vec![last_log];
                        let size = LogGroup::estimate_size(&new_logs, &tags);
                        if size > MAX_SINGLE_SIZE {
                            eprintln!("single log exceeds log group size limit ({size}/{MAX_SINGLE_SIZE}), dropping log")
                        } else {
                            self.buffer.insert(tags, new_logs);
                        }
                    }
                }
            }
        }

        for (tags, logs) in self.buffer.drain() {
            let _ = self
                .client
                .put_log(&LogGroup {
                    logs,
                    reserved: None,
                    topic: None,
                    source: None,
                    log_tags: tags,
                })
                .await;
        }
    }
}

/// A guard that will send a shutdown signal to the dispatcher when dropped.
pub struct WorkGuard {
    pub(crate) shutdown: Option<mpsc::Sender<()>>,
}

impl Drop for WorkGuard {
    fn drop(&mut self) {
        let shutdown = self.shutdown.take().unwrap();
        tokio::spawn(async move {
            let _ = shutdown.send(()).await;
        });
    }
}
