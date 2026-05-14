# CLI rules

Rules for the `iii` CLI surface and how it's documented.

## `iii noun verb` naming convention

Commands and subcommands follow the pattern `iii <noun> <verb>` — the noun (what you're acting on)
precedes the verb (what you're doing).

Reviewing a CLI command involves **two independent checks**. Don't conflate them — they have
different fixes.

1. **Convention check** — does the command form match `iii <noun> <verb>`? If the second word is not
   a noun (e.g., `iii build worker`, `iii deploy <noun>`) or the order is reversed, flag it as a
   *convention violation*. Real English verbs like `reinstall`, `rebuild`, `recheck` satisfy the
   convention; do not flag them on convention grounds just because they start with `re-`.

2. **Recognized-list check** — even when the convention is satisfied, the second word must be a
   known subcommand of the noun. A novel-but-conformant command (e.g., `iii worker reinstall`)
   should be flagged as *unrecognized*, with the suggested replacement when one exists. Do not say
   "doesn't follow the noun-verb convention" for these — the convention is fine; the command isn't
   in the catalog.

### Recognized `iii worker` subcommands

The canonical set, as of writing:

- `iii worker init`
- `iii worker add` (also `iii worker add --force` for the reinstall path)
- `iii worker remove`
- `iii worker list`
- `iii worker start`
- `iii worker stop`
- `iii worker restart`
- `iii worker status`
- `iii worker logs`
- `iii worker exec`
- `iii worker update`
- `iii worker verify`
- `iii worker sync`
- `iii worker clear`

### Known unrecognized `iii worker` commands and their replacements

- `iii worker reinstall` → use `iii worker add --force`. (Convention is fine; `reinstall` is a valid
  verb. It's just not a separate subcommand — the reinstall flow lives behind `add --force`.)

**Recognized exemption — `iii trigger`:** The syntax `iii trigger <function-path> [argA="value" argB=5 ...]` is the canonical way to invoke any registered function from the CLI (e.g., `iii trigger sandbox::run`, `iii trigger state::set`, `iii trigger iii::durable::publish`). The `function-path` follows the worker-namespaced `noun::verb` scheme.

When you encounter another command that doesn't follow `iii noun verb`, flag it on *convention*
grounds — either the command name should change, or the doc should clarify the noun. When the
command follows the convention but isn't recognized, flag it on *recognized-list* grounds and link
to the replacement if one exists.

## `iii worker` CLI is iii-level tooling

All `iii worker` subcommands (see the recognized list above) plus the `iii.lock` lockfile and worker
image build/publish flow are part of iii itself, analogous to `npm`/`cargo`. They are documented in
the iii docs (primarily `using-iii/workers.mdx`), not Worker Docs.

`using-iii/cli.mdx` should reference `using-iii/workers.mdx` for `iii worker` subcommand details
rather than duplicate them.

## `using-iii/cli.mdx` scope

The CLI page covers:

- Engine flags (`--config`, `--use-default-config`, `--version`).
- Cross-cutting CLI verbs that aren't tied to a noun (`iii trigger ...` for invoking functions, if
  that survives).
- A pointer to `using-iii/workers.mdx` for `iii worker` subcommands.

It does **not** enumerate every `iii worker` subcommand — those are on the noun's primary page
(`using-iii/workers.mdx`).
