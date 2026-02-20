#!/usr/bin/env bash
set -euo pipefail

echo "## Agent API Codex stream_exec parity smoke (linux)"
rustc --version
cargo --version

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../../../../" && pwd)"
cd "$ROOT"

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

FAKEBIN="$TMP/fakebin"
mkdir -p "$FAKEBIN"

cat >"$FAKEBIN/codex" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

last_message=""
schema=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-last-message) last_message="${2:-}"; shift 2 ;;
    --output-schema) schema="${2:-}"; shift 2 ;;
    *) shift ;;
  esac
done

# Consume stdin so callers can write prompts without blocking.
cat >/dev/null || true

printf '%s\n' '{"type":"thread.started","thread_id":"thread-1"}'
printf '%s\n' '{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}'
printf '%s\n' '{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"agent_message","content":{"text":"hello from fake codex"}}'
printf '%s\n' '{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}'

if [[ -n "$last_message" ]]; then
  mkdir -p "$(dirname "$last_message")"
  printf '%s' 'hello from fake codex' >"$last_message"
fi

if [[ -n "$schema" ]]; then
  mkdir -p "$(dirname "$schema")"
  printf '%s\n' '{}' >"$schema"
fi

if [[ -n "${CODEX_WRAPPER_SMOKE_DUMP_ENV:-}" ]]; then
  mkdir -p "$(dirname "$CODEX_WRAPPER_SMOKE_DUMP_ENV")"
  env | LC_ALL=C sort >"$CODEX_WRAPPER_SMOKE_DUMP_ENV"
fi
EOF

chmod +x "$FAKEBIN/codex"

export CODEX_HOME="$TMP/codex-home"
mkdir -p "$CODEX_HOME"

# Cover both spawn strategies:
# - wrapper picks up `CODEX_BINARY`
# - direct spawn uses `codex` from `PATH`
export CODEX_BINARY="$FAKEBIN/codex"
export PATH="$FAKEBIN:$PATH"

echo "Running required tests (fixture/fake-binary only)"
cargo test -p agent_api --all-features
cargo test -p agent_api --features codex

if [[ "${RUN_WORKSPACE_ALL:-0}" == "1" ]]; then
  echo "RUN_WORKSPACE_ALL=1: running broader workspace tests (may be slow)"
  cargo test --workspace --all-targets --all-features
fi

if [[ "${RUN_PREFLIGHT:-0}" == "1" ]]; then
  echo "RUN_PREFLIGHT=1: running repo gate (Linux-only)"
  make preflight
fi

echo "OK"

