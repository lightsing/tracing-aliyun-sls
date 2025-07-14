//! A tracing layer that sends logs to Aliyun SLS.
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Formatters for logging [`Event`] to [`aliyun_sls::Log`] event.
pub mod event;
/// Formatters for logging tracing events.
pub mod format;
/// Tracing layer that sends logs to Aliyun SLS.
pub mod layer;
/// Formatters for logging metadata to [`aliyun_sls::LogGroupMetadata`] tags.
pub mod tags;
/// Time utilities for recording timestamps.
pub mod time;

pub use aliyun_sls::{SlsClient, reporter};
pub use layer::layer;
