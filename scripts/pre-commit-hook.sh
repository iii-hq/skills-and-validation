#!/usr/bin/env bash
# pre-commit-hook.sh — run iii-skill-render --write on staged worker dirs,
# re-stage the rendered files, and run iii-skill-check verify-rendered +
# verify --layers structure,vale.
#
# Installed via: scripts/install-hook.sh in the consumer repo.
# Skips the AI layer (slow + costs API tokens); CI runs ai on every PR.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

# Locate the binaries: prefer SKV_BIN on PATH (installed via install.sh),
# fall back to the install dir's `current` symlink.
if command -v iii-skill-render >/dev/null 2>&1 && command -v iii-skill-check >/dev/null 2>&1; then
  RENDER=$(command -v iii-skill-render)
  CHECK=$(command -v iii-skill-check)
else
  CURRENT="${SKV_DIR:-$HOME/.local/share/skill-check}/current"
  if [ ! -x "$CURRENT/bin/iii-skill-render" ] || [ ! -x "$CURRENT/bin/iii-skill-check" ]; then
    cat >&2 <<EOF
iii-skill-render / iii-skill-check not found.
Install with:
  curl -fsSL https://raw.githubusercontent.com/iii-hq/skills-and-validation/latest/scripts/install.sh | bash
EOF
    exit 1
  fi
  RENDER="$CURRENT/bin/iii-skill-render"
  CHECK="$CURRENT/bin/iii-skill-check"
fi

# Find which staged top-level dirs contain a worker manifest + docs/.
# Portable loop (no `mapfile` — works on bash 3.2 / macOS).
staged_workers=""
prev_dir=""
while IFS= read -r path; do
  dir="${path%%/*}"
  [ "$dir" = "$prev_dir" ] && continue
  prev_dir="$dir"
  if [ -f "$dir/iii.worker.yaml" ] && [ -d "$dir/docs" ]; then
    staged_workers+="$dir"$'\n'
  fi
done < <(git diff --cached --name-only | sort -u)

if [ -z "$staged_workers" ]; then
  exit 0
fi

fail=0
while IFS= read -r worker; do
  [ -z "$worker" ] && continue
  echo "iii-skill-render: $worker"
  "$RENDER" "$worker" --write

  # Re-stage anything the render touched (README.md, skill.md, skills/*.md,
  # plus deletions from stale-leaf cleanup).
  git add "$worker/README.md" "$worker/skill.md" 2>/dev/null || true
  git add -A "$worker/skills/" 2>/dev/null || true

  echo "iii-skill-check verify-rendered: $worker"
  if ! "$CHECK" verify-rendered "$worker"; then
    fail=1
  fi

  echo "iii-skill-check verify (structure + vale): $worker"
  if ! "$CHECK" verify "$worker" --layers structure,vale; then
    fail=1
  fi
done <<< "$staged_workers"

if [ "$fail" -ne 0 ]; then
  echo
  echo "iii-skill-check failed. Re-run on the affected worker:"
  echo "  iii-skill-render <worker> --write       # re-render"
  echo "  iii-skill-check verify <worker>         # see all layer violations"
  echo "Bypass with --no-verify if you really need to."
  exit 1
fi
