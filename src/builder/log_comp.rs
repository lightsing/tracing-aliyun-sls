use crate::proto::{KeyValue, Log};
use crate::{client, layer, SlsTracingBuilder};
use chrono::Utc;
use log::{Metadata, Record, SetLoggerError};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing_log::AsLog;

/// A logger that sends logs to Aliyun SLS.
#[cfg_attr(docsrs, doc(cfg(feature = "log-comp")))]
pub struct Logger {
    max_level: log::Level,
    sender: mpsc::Sender<(Vec<KeyValue<'static>>, Log<'static>)>,
    shutdown: Option<mpsc::Sender<()>>,
}

impl Logger {
    /// Try to initialize the logger.
    #[cfg_attr(docsrs, doc(cfg(feature = "log-comp")))]
    pub fn try_init(self) -> Result<(), SetLoggerError> {
        let max_level = self.max_level;
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(max_level.to_level_filter());
        Ok(())
    }

    /// Initialize the logger.
    #[cfg_attr(docsrs, doc(cfg(feature = "log-comp")))]
    pub fn init(self) {
        self.try_init().unwrap();
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.max_level >= metadata.level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let time = Utc::now();

        let mut contents = Vec::with_capacity(6);

        contents.push(KeyValue::new("level", record.level().as_str()));
        contents.push(KeyValue::new("target", record.target().to_string()));
        if let Some(file) = record.file_static() {
            contents.push(KeyValue::new("file", file));
        } else if let Some(file) = record.file() {
            contents.push(KeyValue::new("file", file.to_string()));
        }
        if let Some(line) = record.line() {
            contents.push(KeyValue::new("line", line.to_string()));
        }
        if let Some(module_path) = record.module_path() {
            contents.push(KeyValue::new("module_path", module_path.to_string()));
        }
        contents.push(KeyValue::new("message", format!("{}", record.args())));

        let log = Log {
            time: time.timestamp() as u32,
            time_ns: Some(time.timestamp_subsec_nanos()),
            contents,
        };

        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _ = sender.send((vec![], log)).await;
        });
    }

    fn flush(&self) {}
}

impl Drop for Logger {
    fn drop(&mut self) {
        let shutdown = self.shutdown.take().unwrap();
        tokio::spawn(async move {
            let _ = shutdown.send(()).await;
        });
    }
}

impl SlsTracingBuilder<'_, String, &'_ str, &'_ str, &'_ str, &'_ str> {
    /// Build the logger.
    #[cfg_attr(docsrs, doc(cfg(feature = "log-comp")))]
    pub fn build_logger(self) -> Logger {
        let (sender, receiver) = mpsc::channel(1024);
        let (shutdown, shutdown_rx) = mpsc::channel(1);
        let mut dispatcher = layer::SlsDispatcher {
            receiver,
            client: client::SlsClient::new(
                self.access_key,
                self.access_secret,
                self.endpoint,
                self.project,
                self.logstore,
                self.shard_key,
                #[cfg(feature = "deflate")]
                self.compression_level,
            )
            .unwrap(),
            buffer: HashMap::new(),
            drain_timeout: self.drain_timeout,
            shutdown: shutdown_rx,
        };
        tokio::spawn(async move { dispatcher.run().await });
        Logger {
            max_level: self.max_level.as_log(),
            sender,
            shutdown: Some(shutdown),
        }
    }
}
