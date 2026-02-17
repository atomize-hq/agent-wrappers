#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail=0

check_no_tracked_under() {
  local path="$1"
  if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
    echo "ERROR: repo hygiene: path is tracked but should not be: $path"
    fail=1
  fi
}

check_no_tracked_glob() {
  local pattern="$1"
  local hits
  hits="$(git ls-files | rg -n "$pattern" || true)"
  if [[ -n "$hits" ]]; then
    echo "ERROR: repo hygiene: tracked files match pattern: $pattern"
    echo "$hits"
    fail=1
  fi
}

# Directories that should never be committed.
check_no_tracked_under "wt"
check_no_tracked_under "target"
check_no_tracked_under "_download"
check_no_tracked_under "_extract"
check_no_tracked_under ".codex-bins"
check_no_tracked_under "cli_manifests/codex/raw_help"

# Common artifact types that should not be committed.
check_no_tracked_glob "\\.log$"
check_no_tracked_glob "universal-agent-api-planning-pack_.*\\.zip$"

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "repo hygiene: OK"
