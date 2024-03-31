//! A tracing layer that sends logs to Aliyun SLS.
//!
//! ## Feature Flags
//!
//! Note: `lz4` and `deflate` cannot be enabled at the same time.
//!
//! - `lz4`: enable lz4 compression for logs.
//! - `deflate`: enable deflate compression for logs.
//! - `log-comp`: enable the `Logger` for `log` crate.
//! - `derive-key`: enable the ability to derive the shard key (128bit hex) from any string using BLAKE3.
//!
//! ## Examples
//!
//! ### Tracing
//! ```rust
//! use tracing_aliyun_sls::SlsLayer;
//! use tracing_subscriber::layer::SubscriberExt;
//! use tracing_subscriber::util::SubscriberInitExt;
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() {
//!    let (layer, _guard) = SlsLayer::builder()
//!         .access_key("access_key")
//!         .access_secret("access_secret")
//!         .endpoint("cn-hangzhou.log.aliyuncs.com")
//!         .project("project")
//!         .logstore("logstore")
//!         .shard_key("shard_key") // Optional if you want to use `KeyHash` mode
//!         .max_level(tracing::Level::INFO) // Optional, default is `tracing::Level::TRACE`
//!         .drain_timeout(std::time::Duration::from_secs(10)) // Optional, default is 5 seconds
//!         .build_layer();
//!
//!     tracing_subscriber::registry()
//!         .with(layer)
//!         .init();
//! }
//! ```
//!
//! ### Log
//!
//! If you want to use the `Logger` for `log` crate, you need to enable the `log-comp` feature.
//!
#![cfg_attr(
    feature = "log-comp",
    doc = r#"```rust
use tracing_aliyun_sls::SlsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main(flavor = "current_thread")]
async fn main() {
   SlsLayer::builder()
        .access_key("access_key")
        .access_secret("access_secret")
        .endpoint("cn-hangzhou.log.aliyuncs.com")
        .project("project")
        .logstore("logstore")
        .shard_key("shard_key") // Optional if you want to use `KeyHash` mode
        .max_level(tracing::Level::INFO) // Optional, default is `tracing::Level::TRACE`
        .drain_timeout(std::time::Duration::from_secs(10)) // Optional, default is 5 seconds
        .build_logger()
        .init();
}
```"#
)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(all(feature = "lz4", feature = "deflate"), not(docsrs)))]
compile_error!("`lz4` and `deflate` cannot be enabled at the same time");

mod builder;
mod client;
mod layer;
mod proto;

pub use crate::builder::SlsTracingBuilder;
pub use crate::layer::{SlsLayer, WorkGuard};

#[cfg(feature = "log-comp")]
pub use crate::builder::log_comp::Logger;
