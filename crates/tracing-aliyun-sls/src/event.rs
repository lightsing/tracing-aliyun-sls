use crate::format::Format;
use aliyun_sls::{Log, MayStaticKey};
use compact_str::{CompactString, ToCompactString, format_compact};
use std::fmt;
use tracing::{Event, Subscriber, field::Field};
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime},
    layer::Context,
    registry::LookupSpan,
};

pub trait RecordEvent<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    /// Write a log message for `Event` in `Context` to the given [`Log`].
    fn record_event<T: FormatTime>(
        &self,
        event: &Event<'_>,
        ctx: &Context<'_, S>,
        format: &Format<T>,
        log: &mut Log,
    ) -> fmt::Result;
}

/// The default [`RecordEvent`] implementation to record [`Event`]
#[derive(Debug)]
pub struct DefaultEvent {
    // reserve the ability to add fields to this without causing a breaking
    // change in the future.
    _private: (),
}

impl DefaultEvent {
    /// Create a new `DefaultEvent`.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DefaultEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> RecordEvent<S> for DefaultEvent
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn record_event<T: FormatTime>(
        &self,
        event: &Event<'_>,
        _ctx: &Context<'_, S>,
        format: &Format<T>,
        log: &mut Log,
    ) -> fmt::Result {
        if format.display_level {
            log.insert(
                MayStaticKey::from_static("level"),
                event.metadata().level().as_str(),
            );
        }

        if format.display_timestamp {
            let mut timestamp = CompactString::const_new("");
            format.timer.format_time(&mut Writer::new(&mut timestamp))?;
            log.insert(MayStaticKey::from_static("timestamp"), timestamp);
        }

        let current_thread = std::thread::current();
        if format.display_thread_name {
            if let Some(name) = std::thread::current().name() {
                log.insert(MayStaticKey::from_static("thread_name"), name);
            } else if !format.display_thread_id {
                log.insert(
                    MayStaticKey::from_static("thread_name"),
                    format_compact!("{:0>2?} ", current_thread.id()),
                );
            }
        }
        if format.display_thread_id {
            log.insert(
                MayStaticKey::from_static("thread_id"),
                format_compact!("{:0>2?} ", std::thread::current().id()),
            );
        }

        if format.display_target {
            log.insert(
                MayStaticKey::from_static("target"),
                event.metadata().target(),
            );
        }

        if format.display_line_number {
            if let Some(line) = event.metadata().line() {
                log.insert(MayStaticKey::from_static("line"), line.to_compact_string());
            }
        }

        if format.display_filename {
            if let Some(file) = event.metadata().file() {
                log.insert(MayStaticKey::from_static("file"), file);
            }
        }

        event.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            log.insert(
                MayStaticKey::from_static(field.name()),
                format_compact!("{value:?}"),
            );
        });

        Ok(())
    }
}
