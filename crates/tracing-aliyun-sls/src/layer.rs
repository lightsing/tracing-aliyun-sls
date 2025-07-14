use crate::{
    event,
    event::RecordEvent,
    format,
    format::{RecordSpan, TimingDisplay},
    tags,
    time::{RecordTime, SystemTime},
};
use aliyun_sls::reporter::Reporter;
use aliyun_sls::{Log, LogGroupMetadata, MayStaticKey};
use compact_str::{CompactString, ToCompactString, format_compact};
use std::sync::Arc;
use std::{marker::PhantomData, time::Instant};
use tracing::{
    Event, Id, Metadata, Subscriber,
    span::{Attributes, Record},
};
use tracing_subscriber::registry::SpanRef;
use tracing_subscriber::{
    field::MakeVisitor,
    layer::{self, Context},
    registry::LookupSpan,
};

/// A [`Layer`] that logs to a [`Reporter`].
///
/// [`Layer`]: tracing_subscriber::layer::Layer
pub struct Layer<S, FT = SystemTime, T = tags::DefaultTags, E = event::DefaultEvent> {
    reporter: Reporter,
    format: format::Format<FT>,
    record_tags: T,
    record_event: E,
    record_span: format::RecordSpanConfig,
    instance_id: Option<CompactString>,
    log_internal_errors: bool,
    _inner: PhantomData<fn(S)>,
}

impl<S> Layer<S> {
    /// Returns a new [`Layer`] with the default configuration.
    pub fn new(reporter: Reporter) -> Self {
        Self {
            reporter,
            format: format::Format::default(),
            record_tags: tags::DefaultTags::default(),
            record_event: event::DefaultEvent::default(),
            record_span: format::RecordSpanConfig::default(),
            log_internal_errors: true,
            instance_id: None,
            _inner: PhantomData,
        }
    }
}

impl<S, FT, T, E> Layer<S, FT, T, E> {
    /// Sets the tags recorder for the layer.
    pub fn record_tags<T2>(self, record_tags: T2) -> Layer<S, FT, T2, E>
    where
        T2: for<'a> MakeVisitor<&'a mut LogGroupMetadata> + 'static,
    {
        Layer {
            reporter: self.reporter,
            format: self.format,
            record_tags,
            record_event: self.record_event,
            record_span: self.record_span,
            instance_id: self.instance_id,
            log_internal_errors: self.log_internal_errors,
            _inner: PhantomData,
        }
    }

    /// Sets the event recorder for the layer.
    pub fn record_event<E2>(self, record_event: E2) -> Layer<S, FT, T, E2>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        E2: RecordEvent<S> + 'static,
    {
        Layer {
            reporter: self.reporter,
            format: self.format,
            record_tags: self.record_tags,
            record_event,
            record_span: self.record_span,
            instance_id: self.instance_id,
            log_internal_errors: self.log_internal_errors,
            _inner: PhantomData,
        }
    }

    /// Use the given [`timer`] for span and event timestamps.
    ///
    /// See the [`time` module] for the provided timer implementations.
    //
    /// [`timer`]: tracing_subscriber::fmt::time::FormatTime
    /// [`time` module]: mod@tracing_subscriber::fmt::time
    pub fn with_timer<FT2>(self, timer: FT2) -> Layer<S, FT2, T, E> {
        Layer {
            reporter: self.reporter,
            format: self.format.with_timer(timer),
            record_tags: self.record_tags,
            record_event: self.record_event,
            record_span: self.record_span,
            instance_id: self.instance_id,
            log_internal_errors: self.log_internal_errors,
            _inner: self._inner,
        }
    }

    /// Do not emit timestamps with spans and event.
    pub fn without_time(self) -> Layer<S, (), T, E> {
        Layer {
            reporter: self.reporter,
            format: self.format.without_time(),
            record_tags: self.record_tags,
            record_event: self.record_event,
            record_span: self.record_span.without_time(),
            instance_id: self.instance_id,
            log_internal_errors: self.log_internal_errors,
            _inner: self._inner,
        }
    }

    /// Configures how synthesized events are emitted at points in the [span
    /// lifecycle][lifecycle].
    ///
    /// The following options are available:
    ///
    /// - `RecordSpan::NONE`: No events will be synthesized when spans are
    ///    created, entered, exited, or closed. Data from spans will still be
    ///    included as the context for formatted events. This is the default.
    /// - `RecordSpan::NEW`: An event will be synthesized when spans are created.
    /// - `RecordSpan::ENTER`: An event will be synthesized when spans are entered.
    /// - `RecordSpan::EXIT`: An event will be synthesized when spans are exited.
    /// - `RecordSpan::CLOSE`: An event will be synthesized when a span closes. If
    ///    [timestamps are enabled][time] for this formatter, the generated
    ///    event will contain fields with the span's _busy time_ (the total
    ///    time for which it was entered) and _idle time_ (the total time that
    ///    the span existed but was not entered).
    /// - `RecordSpan::ACTIVE`: Events will be synthesized when spans are entered
    ///    or exited.
    /// - `RecordSpan::FULL`: Events will be synthesized whenever a span is
    ///    created, entered, exited, or closed. If timestamps are enabled, the
    ///    close event will contain the span's busy and idle time, as
    ///    described above.
    ///
    /// Note that the generated events will only be part of the log output by
    /// this formatter; they will not be recorded by other `Subscriber`s or by
    /// `Layer`s added to this subscriber.
    ///
    /// [lifecycle]: https://docs.rs/tracing/latest/tracing/span/index.html#the-span-lifecycle
    /// [time]: Layer::without_time()
    pub fn with_span_events(self, kind: RecordSpan) -> Self {
        Layer {
            record_span: self.record_span.with_kind(kind),
            ..self
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Layer<S, FT, T, E> {
        Layer {
            format: self.format.with_target(display_target),
            ..self
        }
    }

    /// Sets whether or not an event's [source code file path][file] is
    /// displayed.
    ///
    /// [file]: tracing::Metadata::file
    pub fn with_file(self, display_filename: bool) -> Layer<S, FT, T, E> {
        Layer {
            format: self.format.with_file(display_filename),
            ..self
        }
    }

    /// Sets whether or not an event's [source code line number][line] is
    /// displayed.
    ///
    /// [line]: tracing::Metadata::line
    pub fn with_line_number(self, display_line_number: bool) -> Layer<S, FT, T, E> {
        Layer {
            format: self.format.with_line_number(display_line_number),
            ..self
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(self, display_level: bool) -> Layer<S, FT, T, E> {
        Layer {
            format: self.format.with_level(display_level),
            ..self
        }
    }

    /// Sets whether or not the [thread ID] of the current thread is displayed
    /// when formatting events.
    ///
    /// [thread ID]: std::thread::ThreadId
    pub fn with_thread_ids(self, display_thread_ids: bool) -> Layer<S, FT, T, E> {
        Layer {
            format: self.format.with_thread_ids(display_thread_ids),
            ..self
        }
    }

    /// Sets whether or not the [name] of the current thread is displayed
    /// when formatting events.
    ///
    /// [name]: std::thread#naming-threads
    pub fn with_thread_names(self, display_thread_names: bool) -> Layer<S, FT, T, E> {
        Layer {
            format: self.format.with_thread_names(display_thread_names),
            ..self
        }
    }

    /// Sets the instance ID for the layer.
    pub fn with_instance_id(self, instance_id: impl Into<CompactString>) -> Layer<S, FT, T, E> {
        Layer {
            instance_id: Some(instance_id.into()),
            ..self
        }
    }
}

impl<S, FT, T, E> layer::Layer<S> for Layer<S, FT, T, E>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    FT: RecordTime + 'static,
    T: for<'a> MakeVisitor<&'a mut LogGroupMetadata> + 'static,
    E: RecordEvent<S> + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut metadata = self.create_metadata(attrs.metadata());
        attrs
            .values()
            .record(&mut self.record_tags.make_visitor(&mut metadata));

        let span = ctx.span(id).expect("Span not found, this is a bug");

        metadata.add_tag(
            MayStaticKey::from_static("span-id"),
            span.id().into_non_zero_u64().to_compact_string(),
        );

        if let Some(parent) = span.parent() {
            metadata.add_tag(
                MayStaticKey::from_static("parent-span-id"),
                parent.id().into_non_zero_u64().to_compact_string(),
            );
        }

        let mut extensions = span.extensions_mut();

        if self.record_span.timing
            && self.record_span.trace_close()
            && extensions.get_mut::<Timings>().is_none()
        {
            extensions.insert(Timings::new());
        }

        let metadata = Arc::new(metadata);

        if self.record_span.trace_new() {
            let mut log = Log::default();
            self.format.timer.record_time(&mut log);
            log.insert(MayStaticKey::from_static("span"), "new");
            self.reporter.report(metadata.clone(), log);
        }

        extensions.insert(metadata);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut metadata =
            if let Some(metadata) = span.extensions_mut().remove::<Arc<LogGroupMetadata>>() {
                Arc::into_inner(metadata).unwrap_or_else(|| self.create_metadata(span.metadata()))
            } else {
                self.create_metadata(span.metadata())
            };

        values.record(&mut self.record_tags.make_visitor(&mut metadata));
        span.extensions_mut().insert(Arc::new(metadata));
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = match ctx.lookup_current() {
            Some(span) => self.get_or_create_metadata(&span, event.metadata()),
            None => Arc::new(self.create_metadata(event.metadata())),
        };

        let mut log = Log::default();
        self.record_event
            .record_event(event, &ctx, &self.format, &mut log);

        self.reporter.report(metadata, log);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if self.record_span.trace_enter()
            || self.record_span.trace_close() && self.record_span.timing
        {
            let span = ctx.span(id).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();
            if let Some(timings) = extensions.get_mut::<Timings>() {
                let now = Instant::now();
                timings.idle += (now - timings.last).as_nanos() as u64;
                timings.last = now;
            }

            if self.record_span.trace_enter() {
                let metadata = self.get_or_create_metadata(&span, span.metadata());
                let mut log = Log::default();
                self.format.timer.record_time(&mut log);
                log.insert(MayStaticKey::from_static("span"), "enter");
                self.reporter.report(metadata, log);
            }
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if self.record_span.trace_exit()
            || self.record_span.trace_close() && self.record_span.timing
        {
            let span = ctx.span(id).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();
            if let Some(timings) = extensions.get_mut::<Timings>() {
                let now = Instant::now();
                timings.busy += (now - timings.last).as_nanos() as u64;
                timings.last = now;
            }

            if self.record_span.trace_exit() {
                let metadata = self.get_or_create_metadata(&span, span.metadata());
                let mut log = Log::default();
                self.format.timer.record_time(&mut log);
                log.insert(MayStaticKey::from_static("span"), "exit");
                self.reporter.report(metadata, log);
            }
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        if self.record_span.trace_close() {
            let metadata = self.get_or_create_metadata(&span, span.metadata());

            let mut log = Log::default();
            self.format.timer.record_time(&mut log);
            log.insert(MayStaticKey::from_static("span"), "close");
            if let Some(timing) = span.extensions().get::<Timings>() {
                let Timings {
                    busy,
                    mut idle,
                    last,
                } = *timing;
                idle += (Instant::now() - last).as_nanos() as u64;

                log.insert(
                    MayStaticKey::from_static("time.busy"),
                    format_compact!("{}", TimingDisplay(idle)),
                );
                log.insert(
                    MayStaticKey::from_static("time.idle"),
                    format_compact!("{}", TimingDisplay(busy)),
                );
            };

            self.reporter.report(metadata, log);
        }
    }
}

impl<S, FT, T, E> Layer<S, FT, T, E>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn get_or_create_metadata(
        &self,
        span: &SpanRef<S>,
        metadata: &Metadata,
    ) -> Arc<LogGroupMetadata> {
        if let Some(metadata) = span.extensions().get::<Arc<LogGroupMetadata>>() {
            metadata.clone()
        } else {
            let metadata = Arc::new(self.create_metadata(metadata));
            span.extensions_mut().insert(metadata.clone());
            metadata
        }
    }

    fn create_metadata(&self, metadata: &Metadata) -> LogGroupMetadata {
        let mut log_meta = LogGroupMetadata::default()
            .with_topic(metadata.name())
            .with_source(metadata.target());

        if let Some(instance_id) = &self.instance_id {
            log_meta.add_tag(
                MayStaticKey::from_static("instance_id"),
                instance_id.clone(),
            );
        }
        log_meta.add_tag(
            MayStaticKey::from_static("level"),
            metadata.level().as_str(),
        );
        if let Some(module_path) = metadata.module_path() {
            log_meta.add_tag(MayStaticKey::from_static("module_path"), module_path);
        }
        if let Some(file) = metadata.file() {
            log_meta.add_tag(MayStaticKey::from_static("file"), file);
        }
        if let Some(line) = metadata.line() {
            log_meta.add_tag(MayStaticKey::from_static("line"), line.to_compact_string());
        }
        log_meta
    }
}

/// Returns a new [aliyun sls layer] that can be [composed] with other layers to
/// construct a [`Subscriber`].
///
/// This is a shorthand for the equivalent [`Layer::new`] function.
///
/// [aliyun sls layer]: Layer
/// [composed]: tracing_subscriber::layer
/// [`Layer::default`]: Layer::new
pub fn layer<S>(reporter: Reporter) -> Layer<S> {
    Layer::new(reporter)
}

struct Timings {
    idle: u64,
    busy: u64,
    last: Instant,
}

impl Timings {
    fn new() -> Self {
        Self {
            idle: 0,
            busy: 0,
            last: Instant::now(),
        }
    }
}
