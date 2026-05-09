First, build from source:

```bash
cargo build --release
```

Verify the install:

```bash
broken-worker --help
broken-worker --manifest | jq
```
