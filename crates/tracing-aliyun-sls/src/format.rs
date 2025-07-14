use crate::time::SystemTime;
use bitflags::bitflags;
use std::{fmt, fmt::Display};

/// Formatter for logging tracing events.
pub struct Format<T = SystemTime> {
    pub(crate) timer: T,
    pub(crate) display_target: bool,
    pub(crate) display_level: bool,
    pub(crate) display_thread_id: bool,
    pub(crate) display_thread_name: bool,
    pub(crate) display_filename: bool,
    pub(crate) display_line_number: bool,
}

pub(super) struct RecordSpanConfig {
    pub(super) kind: RecordSpan,
    pub(super) timing: bool,
}

bitflags! {
    /// Configures what points in the span lifecycle are logged as events.
    pub struct RecordSpan: u8 {
        /// one event when span is created
        const NEW = 1;
        /// one event per enter of a span
        const ENTER = 1 << 1;
        /// one event per exit of a span
        const EXIT = 1 << 2;
        /// one event when the span is dropped
        const CLOSE = 1 << 3;


        /// spans are ignored (this is the default)
        const NONE = 0;
        /// one event per enter/exit of a span
        const ACTIVE = Self::NEW.bits() | Self::ENTER.bits();
        /// events at all points (new, enter, exit, drop)
        const FULL = Self::NEW.bits() | Self::ENTER.bits() | Self::EXIT.bits() | Self::CLOSE.bits();
    }
}

impl Default for Format<SystemTime> {
    fn default() -> Self {
        Format {
            timer: SystemTime,
            display_target: true,
            display_level: true,
            display_thread_id: false,
            display_thread_name: false,
            display_filename: false,
            display_line_number: false,
        }
    }
}

impl<T> Format<T> {
    /// Use the given [`timer`] for log message timestamps.
    ///
    /// See [`time` module] for the provided timer implementations.
    ///
    /// [`timer`]: tracing_subscriber::fmt::time::FormatTime
    /// [`time` module]: mod@tracing_subscriber::fmt::time
    pub fn with_timer<T2>(self, timer: T2) -> Format<T2> {
        Format {
            timer,
            display_target: self.display_target,
            display_level: self.display_level,
            display_thread_id: self.display_thread_id,
            display_thread_name: self.display_thread_name,
            display_filename: self.display_filename,
            display_line_number: self.display_line_number,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Format<()> {
        Format {
            ..self.with_timer(())
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Format<T> {
        Format {
            display_target,
            ..self
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(self, display_level: bool) -> Format<T> {
        Format {
            display_level,
            ..self
        }
    }

    /// Sets whether or not the [thread ID] of the current thread is displayed
    /// when formatting events.
    ///
    /// [thread ID]: std::thread::ThreadId
    pub fn with_thread_ids(self, display_thread_id: bool) -> Format<T> {
        Format {
            display_thread_id,
            ..self
        }
    }

    /// Sets whether or not the [name] of the current thread is displayed
    /// when formatting events.
    ///
    /// [name]: std::thread#naming-threads
    pub fn with_thread_names(self, display_thread_name: bool) -> Format<T> {
        Format {
            display_thread_name,
            ..self
        }
    }

    /// Sets whether or not an event's [source code file path][file] is
    /// displayed.
    ///
    /// [file]: tracing::Metadata::file
    pub fn with_file(self, display_filename: bool) -> Format<T> {
        Format {
            display_filename,
            ..self
        }
    }

    /// Sets whether or not an event's [source code line number][line] is
    /// displayed.
    ///
    /// [line]: tracing::Metadata::line
    pub fn with_line_number(self, display_line_number: bool) -> Format<T> {
        Format {
            display_line_number,
            ..self
        }
    }

    /// Sets whether or not the source code location from which an event
    /// originated is displayed.
    ///
    /// This is equivalent to calling [`Format::with_file`] and
    /// [`Format::with_line_number`] with the same value.
    pub fn with_source_location(self, display_location: bool) -> Self {
        self.with_line_number(display_location)
            .with_file(display_location)
    }
}

impl Default for RecordSpanConfig {
    fn default() -> Self {
        Self {
            kind: RecordSpan::NONE,
            timing: false,
        }
    }
}

impl RecordSpanConfig {
    pub(super) fn without_time(self) -> Self {
        Self {
            kind: self.kind,
            timing: false,
        }
    }
    pub(super) fn with_kind(self, kind: RecordSpan) -> Self {
        Self {
            kind,
            timing: self.timing,
        }
    }
    pub(super) fn trace_new(&self) -> bool {
        self.kind.contains(RecordSpan::NEW)
    }
    pub(super) fn trace_enter(&self) -> bool {
        self.kind.contains(RecordSpan::ENTER)
    }
    pub(super) fn trace_exit(&self) -> bool {
        self.kind.contains(RecordSpan::EXIT)
    }
    pub(super) fn trace_close(&self) -> bool {
        self.kind.contains(RecordSpan::CLOSE)
    }
}

pub(super) struct TimingDisplay(pub(super) u64);
impl Display for TimingDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut t = self.0 as f64;
        for unit in ["ns", "µs", "ms", "s"].iter() {
            if t < 10.0 {
                return write!(f, "{:.2}{}", t, unit);
            } else if t < 100.0 {
                return write!(f, "{:.1}{}", t, unit);
            } else if t < 1000.0 {
                return write!(f, "{:.0}{}", t, unit);
            }
            t /= 1000.0;
        }
        write!(f, "{:.0}s", t * 1000.0)
    }
}
