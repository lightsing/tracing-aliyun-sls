use aliyun_sls::{LogGroupMetadata, MayStaticKey};
use std::fmt::Debug;
use tracing::field::{Field, Visit};
use tracing_subscriber::field::MakeVisitor;

/// The default [`MakeVisitor`] implementation to record [`Attributes`]
///
/// [`Attributes`]: tracing::span::Attributes
#[derive(Debug)]
pub struct DefaultTags {
    // reserve the ability to add fields to this without causing a breaking
    // change in the future.
    _private: (),
}

/// The [visitor] produced by [`DefaultTags`]'s [`MakeVisitor`] implementation.
///
/// [visitor]: Visit
/// [`MakeVisitor`]: MakeVisitor
#[derive(Debug)]
pub struct DefaultTagsVisitor<'a> {
    meta: &'a mut LogGroupMetadata,
}

impl DefaultTags {
    /// Returns a new default [`MakeVisitor`] implementation.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DefaultTags {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MakeVisitor<&'a mut LogGroupMetadata> for DefaultTags {
    type Visitor = DefaultTagsVisitor<'a>;

    #[inline]
    fn make_visitor(&self, target: &'a mut LogGroupMetadata) -> Self::Visitor {
        DefaultTagsVisitor::new(target)
    }
}

impl<'a> DefaultTagsVisitor<'a> {
    /// Returns a new default visitor that formats to the provided `meta`.
    ///
    /// # Arguments
    /// - `meta`: the [`LogGroupMetadata`] to format to.
    pub fn new(meta: &'a mut LogGroupMetadata) -> Self {
        Self { meta }
    }
}

impl Visit for DefaultTagsVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.meta
            .add_tag(MayStaticKey::from_static(field.name()), value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.meta.add_tag(
            MayStaticKey::from_static(field.name()),
            format!("{value:?}"),
        );
    }
}
