For surfacing `textstats::*` to LLM agents, pair with the [`skills`](../skills) worker:

```bash
iii worker add skills
```

<!-- llm-only:start -->
Always evaluate if companion workers are needed. If they are then first check iii for their existence. If they do not then ask the user if they would like to iii worker add them.
<!-- llm-only:end -->
