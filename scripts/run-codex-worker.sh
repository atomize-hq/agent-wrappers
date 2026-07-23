#!/usr/bin/env bash
# Run a bounded Codex task through an explicit profile in an assigned worktree.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/run-codex-worker.sh --profile NAME --worktree DIR --packet FILE \
  [--sandbox read-only|workspace-write] [--output-last-message FILE] [--dry-run]

The packet must be outside the target worktree. A profile overlay named
$CODEX_HOME/NAME.config.toml must exist; the launcher never falls back to the
default Codex configuration.
EOF
}

profile=""
worktree=""
packet=""
sandbox="workspace-write"
output_last_message=""
dry_run=0

while (($#)); do
  case "$1" in
    --profile) profile=${2:?missing value for --profile}; shift 2 ;;
    --worktree) worktree=${2:?missing value for --worktree}; shift 2 ;;
    --packet) packet=${2:?missing value for --packet}; shift 2 ;;
    --sandbox) sandbox=${2:?missing value for --sandbox}; shift 2 ;;
    --output-last-message) output_last_message=${2:?missing value for --output-last-message}; shift 2 ;;
    --dry-run) dry_run=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'error: unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$profile" && -n "$worktree" && -n "$packet" ]] || {
  printf '%s\n' 'error: --profile, --worktree, and --packet are required' >&2
  usage >&2
  exit 2
}
[[ "$profile" =~ ^[A-Za-z0-9_-]+$ ]] || {
  printf '%s\n' 'error: profile may contain only letters, digits, _ and -' >&2
  exit 2
}
case "$sandbox" in read-only|workspace-write) ;; *)
  printf '%s\n' 'error: --sandbox must be read-only or workspace-write' >&2
  exit 2 ;;
esac

worktree=$(cd "$worktree" && pwd -P)
packet=$(cd "$(dirname "$packet")" && pwd -P)/$(basename "$packet")
[[ -d "$worktree/.git" || -f "$worktree/.git" ]] || {
  printf 'error: worktree is not a Git worktree: %s\n' "$worktree" >&2
  exit 2
}
[[ -r "$packet" ]] || { printf 'error: packet is not readable: %s\n' "$packet" >&2; exit 2; }
case "$packet" in "$worktree"/*)
  printf '%s\n' 'error: packet must be outside the target worktree' >&2; exit 2 ;;
esac

codex_home=${CODEX_HOME:-"$HOME/.codex"}
profile_file="$codex_home/$profile.config.toml"
[[ -f "$profile_file" ]] || {
  printf 'error: required Codex profile overlay does not exist: %s\n' "$profile_file" >&2
  exit 2
}
command -v codex >/dev/null || { printf '%s\n' 'error: codex is not on PATH' >&2; exit 127; }

cmd=(codex exec --profile "$profile" --sandbox "$sandbox" --cd "$worktree")
if [[ -n "$output_last_message" ]]; then
  output_last_message=$(cd "$(dirname "$output_last_message")" && pwd -P)/$(basename "$output_last_message")
  cmd+=(--output-last-message "$output_last_message")
fi
cmd+=(-)

if ((dry_run)); then
  printf 'profile overlay: %s\n' "$profile_file"
  printf 'packet: %s\n' "$packet"
  printf 'command:'
  printf ' %q' "${cmd[@]}"
  printf '\n'
  exit 0
fi

"${cmd[@]}" < "$packet"
