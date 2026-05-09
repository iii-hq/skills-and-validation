#!/usr/bin/env bash
# install-hook.sh — symlink the bundled pre-commit hook into the current
# repo's .git/hooks/pre-commit.
#
# Run from the root of the consumer repo:
#   ~/.local/share/skill-check/current/scripts/install-hook.sh

set -euo pipefail

# Find the directory containing this script (resolves through symlinks).
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
hook_src="$script_dir/pre-commit-hook.sh"

if [ ! -f "$hook_src" ]; then
  echo "ERROR: pre-commit-hook.sh not found next to install-hook.sh" >&2
  exit 1
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "ERROR: not inside a git repository" >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"

# Refuse to install into the skills-and-validation source repo itself —
# detected by the action.yml's first line declaring `name: skills-and-validation`.
if [ -f "$repo_root/action.yml" ] && \
   [ "$(head -n 1 "$repo_root/action.yml")" = "name: skills-and-validation" ]; then
  echo "ERROR: refusing to install the pre-commit hook into the skills-and-validation repository itself." >&2
  echo >&2
  echo "cd into the consumer repo (the one whose workers you want to validate) and run this script from there:" >&2
  echo "  cd /path/to/your/consumer-repo" >&2
  echo "  ~/.local/share/skill-check/current/scripts/install-hook.sh" >&2
  exit 1
fi

git_dir="$(git rev-parse --git-dir)"
hook_dst="$git_dir/hooks/pre-commit"

if [ -e "$hook_dst" ] && [ ! -L "$hook_dst" ]; then
  echo "ERROR: $hook_dst already exists and is not a symlink." >&2
  echo "Move or remove it before installing the skill-check hook." >&2
  exit 1
fi

mkdir -p "$git_dir/hooks"
ln -sf "$hook_src" "$hook_dst"
chmod +x "$hook_src"

echo "Installed: $hook_dst -> $hook_src"
echo
echo "On every commit, the hook will:"
echo "  1. Re-render any staged worker (iii-skill-render --write)"
echo "  2. Re-stage rendered README.md, skill.md, skills/*.md"
echo "  3. Run verify-rendered + verify --layers structure,vale"
echo "  4. Block the commit on remaining violations"
echo
echo "Skip with: git commit --no-verify"
