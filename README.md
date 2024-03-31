# tracing-aliyun-sls

Enable tracing integration with Aliyun SLS.

## Example

```rust
use tracing_aliyun_sls::SlsTracingBuilder;

fn main() {
    let (layer, _guard) = SlsTracingBuilder::new(
        "ACCESS_KEY_ID",
        "ACCESS_KEY_SECRET",
        "ENDPOINT",
        "PROJECT",
        "LOGSTORE",
    )
        .max_level(tracing::Level::INFO)
        .build();
    
    tracing_subscriber::registry()
        .with(layer)
        .init();
}
```