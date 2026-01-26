#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
  echo "codex 0.77.0"
  exit 0
fi

if [[ "${1:-}" == "help" ]]; then
  shift
  case "$*" in
    "")
      # `codex help` prints root help.
      cat <<'EOF'
codex 0.77.0

Usage: codex [OPTIONS] <COMMAND>

Commands:
  zed      Zed command
  exec     Execute commands
  alpha    Alpha command
  help     Print this message or the help of the given subcommand(s)

Options:
  -q, --quiet        Suppress output
  --json             Emit JSON
  -v                 Verbose output
EOF
      ;;
    "help")
      cat <<'EOF'
Usage: codex help [COMMAND]...

Print this message or the help of the given subcommand(s)
EOF
      ;;
    "exec")
      cat <<'EOF'
Usage: codex exec <COMMAND>

Commands:
  start    Start execution
  resume   Resume execution

Options:
  --beta            Beta option (long only)
  -a, --alpha       Alpha option
  -c               Short-only option
EOF
      ;;
    "exec start")
      cat <<'EOF'
Usage: codex exec start [OPTIONS] <PROMPT>

Arguments:
  <PROMPT>
          First line of prompt description
          Second line of prompt description (wrapped)

Options:
  --zulu            Zulu mode (long only)
  -b                Short-only option
  -a, --alpha       Alpha option
  -d, --delta PATH  Delta path (takes value)
EOF
      ;;
    "exec resume")
      cat <<'EOF'
Usage: codex exec resume [OPTIONS]

Options:
  --json            Emit JSON
  -q, --quiet       Suppress output
EOF
      ;;
    "alpha")
      cat <<'EOF'
Usage: codex alpha [OPTIONS]

Options:
  -x, --xray        Xray mode
EOF
      ;;
    "zed")
      cat <<'EOF'
Usage: codex zed [OPTIONS]

Options:
  --zebra           Zebra mode
EOF
      ;;
    "sandbox")
      cat <<'EOF'
Usage: codex sandbox [OPTIONS]

Options:
  --linux-only      Linux only
EOF
      ;;
    *)
      echo "fake_codex: unsupported help path: $*" >&2
      exit 2
      ;;
  esac
  exit 0
fi

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'EOF'
codex 0.77.0

Usage: codex [OPTIONS] <COMMAND>

Commands:
  zed      Zed command with a wrapped description
           that should not be interpreted as a new command token
  exec     Execute commands
  alpha    Alpha command
  help     Print this message or the help of the given subcommand(s)

Options:
  -q, --quiet        Suppress output
  --json             Emit JSON
  -v                 Verbose output
EOF
  exit 0
fi

want_help=false
if [[ "${@: -1}" == "--help" || "${@: -1}" == "-h" ]]; then
  want_help=true
  set -- "${@:1:$#-1}"
fi

if [[ "$want_help" != "true" ]]; then
  echo "fake_codex: only --version and --help are implemented" >&2
  exit 2
fi

case "$*" in
  "help")
    cat <<'EOF'
Usage: codex help [COMMAND]...

Print this message or the help of the given subcommand(s)
EOF
    ;;
  "exec")
    cat <<'EOF'
Usage: codex exec <COMMAND>

Commands:
  start    Start execution
  resume   Resume execution

Options:
  --beta            Beta option (long only)
  -a, --alpha       Alpha option
  -c               Short-only option
EOF
    ;;
  "exec start")
    cat <<'EOF'
Usage: codex exec start [OPTIONS] <PROMPT>

Options:
  --zulu            Zulu mode (long only)
  -b                Short-only option
  -a, --alpha       Alpha option
  -d, --delta PATH  Delta path (takes value)
EOF
    ;;
  "exec resume")
    cat <<'EOF'
Usage: codex exec resume [OPTIONS]

Options:
  --json            Emit JSON
  -q, --quiet       Suppress output
EOF
    ;;
  "alpha")
    cat <<'EOF'
Usage: codex alpha [OPTIONS]

Options:
  -x, --xray        Xray mode
EOF
    ;;
  "zed")
    cat <<'EOF'
Usage: codex zed [OPTIONS]

Options:
  --zebra           Zebra mode
EOF
    ;;
  "sandbox")
    cat <<'EOF'
Usage: codex sandbox [OPTIONS]

Options:
  --linux-only      Linux only
EOF
    ;;
  *)
    echo "fake_codex: unsupported help path: $*" >&2
    exit 2
    ;;
esac
