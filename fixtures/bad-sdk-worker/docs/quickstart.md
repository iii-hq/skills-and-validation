```rust
use iii_sdk::{register_worker, InitOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let iii = register_worker("ws://localhost:49134", InitOptions::default());
    let _ = iii;
    Ok(())
}
```
