#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def _utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _which(cmd: str) -> str | None:
    from shutil import which

    return which(cmd)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Spawn a coding agent worker and write a DONE sentinel on exit.")
    parser.add_argument("--repo-root", required=True, help="Repo root (cwd for codex)")
    parser.add_argument("--task-id", required=True, help="Task id")
    parser.add_argument("--run-dir", required=True, help="Run directory (.runs/<TASK_ID>)")
    parser.add_argument("--prompt-file", default="prompt.md", help="Prompt filename inside run-dir")
    parser.add_argument("--log-file", default="worker.log", help="Log filename inside run-dir")
    parser.add_argument("--pid-file", default="worker.pid", help="PID filename inside run-dir")
    parser.add_argument("--done-file", default="", help="DONE filename inside run-dir (default: <TASK_ID>.done)")
    parser.add_argument("--last-message-file", default="last_message.md", help="Final message capture filename")
    parser.add_argument(
        "--codex-cmd",
        default="codex exec --full-auto",
        help="Codex command prefix (default: 'codex exec --full-auto')",
    )
    args = parser.parse_args(argv)

    repo_root = Path(args.repo_root).resolve()
    task_id = str(args.task_id).strip()
    run_dir = Path(args.run_dir).resolve()

    prompt_path = run_dir / args.prompt_file
    log_path = run_dir / args.log_file
    pid_path = run_dir / args.pid_file
    last_message_path = run_dir / args.last_message_file
    done_name = args.done_file.strip() or f"{task_id}.done"
    done_path = run_dir / done_name

    run_dir.mkdir(parents=True, exist_ok=True)
    if done_path.exists():
        done_path.unlink()

    _write_text(pid_path, f"{os.getpid()}\n")

    if not prompt_path.exists():
        _write_text(done_path, f"status=failed\ntask_id={task_id}\nfinished_at={_utc_now()}\nerror=missing_prompt\n")
        return 2

    if not repo_root.exists():
        _write_text(done_path, f"status=failed\ntask_id={task_id}\nfinished_at={_utc_now()}\nerror=missing_repo\n")
        return 2

    use_script = _which("script") is not None
    codex_prefix = args.codex_cmd.strip()

    inner = f'{codex_prefix} -o {shlex.quote(str(last_message_path))} - < {shlex.quote(str(prompt_path))}'

    if use_script:
        cmd = ["script", "-eq", "/dev/null", "bash", "-lc", inner]
    else:
        cmd = ["bash", "-lc", inner]

    exit_code: int = 0
    status = "success"
    error: str | None = None
    try:
        with log_path.open("wb") as log_handle:
            completed = subprocess.run(
                cmd,
                cwd=str(repo_root),
                stdout=log_handle,
                stderr=subprocess.STDOUT,
                check=False,
            )
            exit_code = int(completed.returncode)
            if exit_code != 0:
                status = "failed"
                error = "nonzero_exit"
    except Exception as exc:
        status = "failed"
        error = f"exception:{type(exc).__name__}"
        exit_code = 1

    lines = [
        f"status={status}",
        f"task_id={task_id}",
        f"finished_at={_utc_now()}",
        f"log_path={log_path}",
        f"last_message_path={last_message_path}",
        f"exit_code={exit_code}",
    ]
    if error:
        lines.append(f"error={error}")

    _write_text(done_path, "\n".join(lines) + "\n")
    return 0 if status == "success" else 1


if __name__ == "__main__":
    raise SystemExit(main())

