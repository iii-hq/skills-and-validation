# Configuration rules

Rules for configuration file naming and conventions.

## Config file names

`config.yaml` is the canonical filename for runtime configuration. It appears in two places that don't conflict because they live at different paths:

- **Engine config:** `~/.iii/config.yaml` — engine-wide settings (workers list, ports, telemetry).
- **Worker runtime config:** `<worker>/config.yaml` — per-worker runtime settings, rendered verbatim into the worker README under `## Configuration`.

The engine reads its own `config.yaml`; each worker reads its own. They share neither state nor schema.

The worker manifest is `iii.worker.yaml` — separate from the worker's runtime `config.yaml`. The manifest carries identity (`name`, `language`, `deploy`, `bin`); the runtime config carries values the worker reads at boot.

When source content references `iii-config.yaml`, normalize to `config.yaml` in any stub. Note the rename in the decisions log if the source page is being absorbed.

## Config reference is auto-generated (planned)

The configuration reference (the per-field schema for `config.yaml`) is intended to be auto-generated from a commented YAML source file colocated with the engine, then transcluded into `using-iii/engine.mdx`. A pre-Mintlify implementation (parser + React component, commit `0f925fd2` in iii-mono) was dropped during the Mintlify migration.

Until restored:
- Don't hand-author per-field schema content in the iii docs.
- The "Engine configuration" stub on `using-iii/engine.mdx` is a placeholder for the eventual generated content.

