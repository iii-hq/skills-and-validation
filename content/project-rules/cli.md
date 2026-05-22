# CLI rules

Rules for the `iii` CLI surface and how it's documented.

## `iii noun verb` naming convention

Commands and subcommands follow the pattern `iii <noun> <verb>` — the noun (what you're acting on)
precedes the verb (what you're doing).

Reviewing a CLI command involves **two independent checks**. Don't conflate them — they have
different fixes and different severities.

1. **Convention check.** **Severity: warning.** Does the command form match `iii <noun> <verb>`?
   If the second word is not a noun (e.g., `iii build worker`, `iii deploy <noun>`) or the order is
   reversed, flag it as a *convention violation*. Real English verbs like `reinstall`, `rebuild`,
   `recheck` satisfy the convention; do not flag them on convention grounds just because they
   start with `re-`. Naming is a guideline rather than a hard requirement; the right form is often
   debatable, so this check surfaces as a warning rather than an error.

2. **Recognized-list check.** Even when the convention is satisfied, the second word must be a
   known subcommand of the noun. A novel-but-conformant command (e.g., `iii worker reinstall`)
   should be flagged as *unrecognized*, with the suggested replacement when one exists. Do not say
   "doesn't follow the noun-verb convention" for these — the convention is fine; the command isn't
   in the catalog. Unrecognized commands are an error (they mislead readers about what the CLI
   actually supports).

3. **"Re-" verbs are still verbs.** `reinstall`, `restart`, `reset`, `rebuild` all satisfy the
   noun-verb convention. Do not flag them on convention grounds.

### Recognized iii commands

Canonical lists from `iii <noun> --help`. Treat these as the source of truth; flag any other
noun-verb pair as *unrecognized* (not as a convention violation).

**`iii worker`** subcommands:
- `iii worker init`
- `iii worker add`
- `iii worker remove`
- `iii worker reinstall`
- `iii worker update`
- `iii worker clear`
- `iii worker start`
- `iii worker stop`
- `iii worker restart`
- `iii worker list`
- `iii worker sync`
- `iii worker verify`
- `iii worker status`
- `iii worker logs`
- `iii worker exec`
- `iii worker sandbox`

**`iii cloud`** subcommands: `login`, `logout`, `whoami`, `context`, `orgs`, `projects`, `envs`,
`deploy`, `deployments`, `versions`, `vars`, `domains`, `api-keys`, `registry`, `completions`,
`push`.

**`iii worker sandbox`** subcommands: `run`, `create`, `exec`, `list`, `stop`, `upload`, `download`.

**`iii project`** subcommands: `init`, `generate-docker`.

**Verbless top-level commands** (take args, not a verb): `iii trigger <function-path>`,
`iii console`, `iii update`. The `iii trigger` form is documented separately below.

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
