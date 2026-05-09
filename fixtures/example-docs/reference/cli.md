---
title: "iii CLI"
description: "Command reference for the iii CLI."
owner: "platform"
type: "reference"
---

<!-- skill:exclude-sections-by-default -->

# iii CLI

Top-level commands for the `iii` binary.

## worker <!-- skill:include-section -->

Subcommands for managing workers.

- `iii worker new <name>` — scaffold a new worker.
- `iii worker add <name>` — register an installed worker with the engine.
- `iii worker start <name>` — start the worker.
- `iii worker stop <name>` — stop the worker.

## creds <!-- skill:include-section -->

Credential management subcommands.

- `iii creds new <worker>` — issue a new credential pair.
- `iii creds rotate <worker>` — promote the pending credential to active.
- `iii creds rollback <worker>` — restore the previous active credential.

## update

Bumps the engine binary in place. Excluded from the skill — covered separately in the install how-to.
