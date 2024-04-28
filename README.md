# tracing-aliyun-sls
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
![MIT licensed][license-badge]
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Flightsing%2Ftracing-aliyun-sls.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Flightsing%2Ftracing-aliyun-sls?ref=badge_shield)

[crates-badge]: https://img.shields.io/crates/v/tracing-aliyun-sls.svg
[crates-url]: https://crates.io/crates/tracing-aliyun-sls
[docs-badge]: https://docs.rs/tracing-aliyun-sls/badge.svg
[docs-url]: https://docs.rs/tracing-aliyun-sls
[license-badge]: https://img.shields.io/badge/license-MIT%20OR%20Apache2.0-blue.svg

Enable tracing integration with [Aliyun SLS](https://help.aliyun.com/zh/sls), also support `log` crate.

## Feature Flags


- `lz4`: enable lz4 compression for logs.
- `deflate`: enable deflate compression for logs.
- `log-comp`: enable the `Logger` for `log` crate.
- `derive-key`: enable the ability to derive the shard key (128 bits hex) from any string using BLAKE3.
- `rustls`: By default, it enables `reqwest/default-tls`, 
  if you want to use `rustls` as the TLS backend, enable this feature also disable default features.

Note: `lz4` and `deflate` cannot be enabled at the same time.

## Example

To use with `tracing`:
```rust
use tracing_aliyun_sls::SlsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
   let (layer, _guard) = SlsLayer::builder()
        .access_key("access_key")
        .access_secret("access_secret")
        .endpoint("cn-hangzhou.log.aliyuncs.com")
        .project("project")
        .logstore("logstore")
        .shard_key("shard_key") // Optional if you want to use `KeyHash` mode
        .max_level(tracing::Level::INFO) // Optional, default is `tracing::Level::TRACE`
        .drain_timeout(std::time::Duration::from_secs(10)) // Optional, default is 5 seconds
        .build_layer();
    
    tracing_subscriber::registry()
        .with(layer)
        .init();
}
```

If you want to use it with `log`, enable the `log-comp` feature:
```rust
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
```

## License
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Flightsing%2Ftracing-aliyun-sls.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2Flightsing%2Ftracing-aliyun-sls?ref=badge_large)