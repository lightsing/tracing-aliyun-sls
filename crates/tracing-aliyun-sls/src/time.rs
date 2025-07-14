use aliyun_sls::Log;
use std::time::Instant;

/// A type that can measure and record the current time.
///
/// This trait is used by `Format` to include a timestamp with each `Event` when it is logged.
///
/// Notable default implementations of this trait are `RecordTime` and `()`. The former prints the
/// current time as reported by `std::time::SystemTime`, and the latter does not print the current
/// time at all. `RecordTime` is also automatically implemented for any function pointer with the
/// appropriate signature.
pub trait RecordTime {
    /// Measure and write out the current time.
    ///
    /// When `format_time` is called, implementors should get the current time using their desired
    /// mechanism, and write it out to the given `fmt::Write`. Implementors must insert a trailing
    /// space themselves if they wish to separate the time from subsequent log message text.
    fn record_time(&self, log: &mut Log);
}

/// Returns a new `SystemTime` timestamp provider.
///
/// This can then be configured further to determine how timestamps should be
/// configured.
///
/// This is equivalent to calling
/// ```rust
/// # fn timer() -> tracing_aliyun_sls::time::SystemTime {
/// tracing_aliyun_sls::time::SystemTime::default()
/// # }
/// ```
pub fn time() -> SystemTime {
    SystemTime
}

/// Returns a new `Uptime` timestamp provider.
///
/// With this timer, timestamps will be formatted with the amount of time
/// elapsed since the timestamp provider was constructed.
///
/// This can then be configured further to determine how timestamps should be
/// configured.
///
/// This is equivalent to calling
/// ```rust
/// # fn timer() -> tracing_aliyun_sls::time::Uptime {
/// tracing_aliyun_sls::time::Uptime::default()
/// # }
/// ```
pub fn uptime() -> Uptime {
    Uptime::default()
}

impl<F> RecordTime for &F
where
    F: RecordTime,
{
    fn record_time(&self, log: &mut Log) {
        (*self).record_time(log)
    }
}

impl RecordTime for () {
    fn record_time(&self, _: &mut Log) {}
}

impl RecordTime for fn(&mut Log) {
    fn record_time(&self, log: &mut Log) {
        (*self)(log)
    }
}

/// Retrieve and print the current wall-clock time.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct SystemTime;

/// Retrieve and print the relative elapsed wall-clock time since an epoch.
///
/// The `Default` implementation for `Uptime` makes the epoch the current time.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Uptime {
    epoch: Instant,
}

impl Default for Uptime {
    fn default() -> Self {
        Uptime {
            epoch: Instant::now(),
        }
    }
}

impl From<Instant> for Uptime {
    fn from(epoch: Instant) -> Self {
        Uptime { epoch }
    }
}

impl RecordTime for SystemTime {
    fn record_time(&self, log: &mut Log) {
        let now = std::time::SystemTime::now();
        if let Ok(duration) = now.duration_since(std::time::UNIX_EPOCH) {
            log.modify_timestamp(duration.as_secs() as u32);
            log.modify_subsec_nanosecond(duration.subsec_nanos());
        }
    }
}

impl RecordTime for Uptime {
    fn record_time(&self, log: &mut Log) {
        let elapsed = self.epoch.elapsed();
        log.modify_timestamp(elapsed.as_secs() as u32);
        log.modify_subsec_nanosecond(elapsed.subsec_nanos());
    }
}
