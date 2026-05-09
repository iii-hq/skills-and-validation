```rust
use iii_sdk::{register_worker, InitOptions, TriggerRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let worker = register_worker("ws://localhost:49134", InitOptions::default());

    let result = worker
        .trigger(TriggerRequest {
            function_id: "textstats::analyze".into(),
            payload: json!({ "text": "hello world\nlooks small" }),
            action: None,
            timeout_ms: Some(5_000),
        })
        .await?;

    println!("{result:#?}");
    Ok(())
}
```

```typescript
import { registerWorker } from 'iii-sdk'

const worker = registerWorker('ws://localhost:49134')

const result = await worker.trigger({
  function_id: 'textstats::analyze',
  payload: { text: 'hello world\nlooks small' },
})

console.log(result)
```

```python
from iii import register_worker

worker = register_worker("ws://localhost:49134")

result = worker.trigger({
    "function_id": "textstats::analyze",
    "payload": {"text": "hello world\nlooks small"},
})

print(result)
```

The example calls `textstats::analyze`. Other entry points: `textstats::diff` and `textstats::summarize`.
