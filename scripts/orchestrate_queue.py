#!/usr/bin/env python3
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import json
import re
import subprocess
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


def _utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: Any) -> None:
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def _which(cmd: str) -> bool:
    from shutil import which

    return which(cmd) is not None


def _run(cmd: list[str], *, cwd: Path, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=str(cwd), text=True, capture_output=True, check=check)


def _git_current_branch(repo_root: Path) -> str:
    return _run(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=repo_root).stdout.strip()


def _git_has_changes(repo_root: Path) -> bool:
    out = _run(["git", "status", "--porcelain=v1"], cwd=repo_root).stdout
    return bool(out.strip())


def _git_commit_paths(repo_root: Path, paths: list[Path], message: str) -> None:
    rels = [str(p.relative_to(repo_root)) for p in paths]
    _run(["git", "add", "--"] + rels, cwd=repo_root, check=True)
    if not _git_has_changes(repo_root):
        return
    _run(["git", "commit", "-m", message], cwd=repo_root, check=True)


def _file_watch_once(directory: Path, *, timeout_s: int) -> None:
    if _which("fswatch"):
        try:
            subprocess.run(
                ["fswatch", "-1", str(directory)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=timeout_s,
                check=False,
            )
            return
        except subprocess.TimeoutExpired:
            return
    time.sleep(timeout_s)


def _tail_lines(path: Path, n: int) -> str:
    try:
        data = path.read_text(encoding="utf-8", errors="replace").splitlines()
        return "\n".join(data[-n:]) + ("\n" if data else "")
    except Exception:
        return ""


def _looks_like_path(s: str) -> bool:
    s = s.strip()
    return ("/" in s or s.endswith(".md") or s.endswith(".txt")) and not s.startswith("#")


def _load_kickoff_text(repo_root: Path, kickoff_ref: str) -> str:
    ref = kickoff_ref.strip()
    if not ref:
        return ""
    if _looks_like_path(ref):
        p = (repo_root / ref).resolve()
        if p.exists():
            return p.read_text(encoding="utf-8")
    return ref


def _extract_required_commands(kickoff_prompt: str) -> list[str]:
    """
    Best-effort: parse "## Commands (required)" section bullets and return shell commands.
    """
    lines = kickoff_prompt.splitlines()
    commands: list[str] = []
    in_section = False
    for raw in lines:
        s = raw.strip()
        if not in_section:
            if s.lower() == "## commands (required)" or s.lower() == "commands (required)":
                in_section = True
            continue
        if in_section:
            if s.startswith("## ") and s.lower() != "## commands (required)":
                break
            m = re.match(r"^-\s+(.*)$", s)
            if not m:
                continue
            item = m.group(1).strip()
            if item.startswith("`") and item.endswith("`") and len(item) >= 2:
                item = item[1:-1]
            if item:
                commands.append(item)
    return commands


@dataclass(frozen=True)
class Task:
    id: str
    index: int
    order: int
    status: str
    depends_on: list[str]
    kickoff_ref: str
    type: str
    worktree: str
    workstream_id: str


def _queue_tasks(payload: Any) -> list[dict[str, Any]]:
    if isinstance(payload, dict):
        tasks = payload.get("tasks", [])
        return [t for t in tasks if isinstance(t, dict)]
    if isinstance(payload, list):
        return [t for t in payload if isinstance(t, dict)]
    raise TypeError("Queue JSON must be an array of tasks or an object with a 'tasks' array.")


def _update_task(payload: Any, task_id: str, updates: dict[str, Any]) -> None:
    for t in _queue_tasks(payload):
        if str(t.get("id", "")).strip() == task_id:
            t.update(updates)
            return
    raise KeyError(f"Task not found: {task_id}")


def _derive_workstream_id(t: dict[str, Any]) -> str:
    ws = str(t.get("workstream_id") or "").strip()
    if ws:
        return ws
    typ = str(t.get("type") or "").strip().lower()
    if typ == "code":
        return "WS-CODE"
    if typ == "test":
        return "WS-TEST"
    if typ == "integration":
        return "WS-INT"
    return "WS-DEFAULT"


def _normalize_status(s: str) -> str:
    v = (s or "").strip().lower()
    if v in {"todo", "pending"}:
        return "pending"
    if v in {"in_progress", "in-progress"}:
        return "in_progress"
    if v in {"done", "completed", "complete"}:
        return "completed"
    if v in {"blocked"}:
        return "blocked"
    if v in {"deferred"}:
        return "deferred"
    return v or "pending"


def _load_tasks(payload: Any, repo_root: Path) -> list[Task]:
    tasks_raw = _queue_tasks(payload)
    out: list[Task] = []
    for idx, t in enumerate(tasks_raw):
        task_id = str(t.get("id", "")).strip()
        if not task_id:
            continue
        order = t.get("order")
        try:
            order_int = int(order) if order is not None else (idx + 1) * 10
        except Exception:
            order_int = (idx + 1) * 10
        depends_on = [str(d) for d in (t.get("depends_on") or []) if str(d).strip()]
        kickoff_ref = str(t.get("kickoff_prompt") or "").strip()
        typ = str(t.get("type") or "").strip()
        worktree = str(t.get("worktree") or "").strip()
        workstream_id = _derive_workstream_id(t)
        out.append(
            Task(
                id=task_id,
                index=idx,
                order=order_int,
                status=_normalize_status(str(t.get("status") or "pending")),
                depends_on=depends_on,
                kickoff_ref=kickoff_ref,
                type=typ,
                worktree=worktree,
                workstream_id=workstream_id,
            )
        )
    return out


def _runnable_tasks(tasks: list[Task], done: set[str]) -> list[Task]:
    runnable: list[Task] = []
    for t in tasks:
        if t.status != "pending":
            continue
        if all(dep in done for dep in t.depends_on):
            runnable.append(t)
    return sorted(runnable, key=lambda x: x.order)


def _active_workstreams(payload: Any) -> set[str]:
    active: set[str] = set()
    for t in _queue_tasks(payload):
        if _normalize_status(str(t.get("status") or "")) == "in_progress":
            active.add(_derive_workstream_id(t))
    return active


def _role_label(task_type: str) -> str:
    v = (task_type or "").strip().lower()
    if v == "code":
        return "Code"
    if v == "test":
        return "Test"
    if v == "integration":
        return "Integration"
    return "Agent"


def _append_session_log(session_log: Path, text: str) -> None:
    session_log.parent.mkdir(parents=True, exist_ok=True)
    existing = session_log.read_text(encoding="utf-8") if session_log.exists() else ""
    if existing and not existing.endswith("\n"):
        existing += "\n"
    session_log.write_text(existing + text, encoding="utf-8")


def _session_log_start(*, session_log: Path, task_id: str, role: str, base_branch: str, kickoff_ref: str, worktree: str) -> None:
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    entry = "\n".join(
        [
            f"## [{ts}] {role} Agent – {task_id} – START",
            f"- Orchestrator: set `{task_id}` → `in_progress` in `tasks.json`",
            f"- Base branch: `{base_branch}`",
            f"- Kickoff prompt: `{kickoff_ref}`",
            f"- Worktree: `{worktree}`" if worktree else "- Worktree: N/A",
            "- Blockers: none",
            "",
        ]
    )
    _append_session_log(session_log, entry)


def _session_log_end(
    *,
    session_log: Path,
    task_id: str,
    role: str,
    worktree: str,
    last_message_path: Path,
    extra: list[str] | None = None,
) -> None:
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    snippet = ""
    if last_message_path.exists():
        msg = last_message_path.read_text(encoding="utf-8", errors="replace").strip()
        msg_lines = msg.splitlines()
        snippet_lines = msg_lines[:40]
        snippet = "\n".join(snippet_lines).strip()
    blockers_line = "- Blockers: none"
    if last_message_path.exists():
        msg = last_message_path.read_text(encoding="utf-8", errors="replace")
        m = re.search(r"(?im)^-\\s*\\*\\*Blocker[s]?\\*\\*:\\s*(.*)$", msg)
        if m:
            tail = m.group(1).strip()
            if tail and tail.lower() not in {"none", "<none>"}:
                blockers_line = f"- Blockers: {tail}"

    lines: list[str] = [
        f"## [{ts}] {role} Agent – {task_id} – END",
        f"- Worktree: `{worktree}`" if worktree else "- Worktree: N/A",
        f"- Worker output: `{last_message_path}`",
    ]
    if extra:
        lines.extend(extra)
    if snippet:
        lines.extend(["- Worker summary (first ~40 lines):", "```text", snippet, "```"])
    lines.extend([blockers_line, ""])
    _append_session_log(session_log, "\n".join(lines))


def _ensure_worktree(*, repo_root: Path, base_branch: str, worktree: str) -> tuple[Path, str]:
    worktree_path = (repo_root / worktree).resolve()
    branch = Path(worktree).name
    if worktree_path.exists():
        gitfile = worktree_path / ".git"
        if not gitfile.exists():
            raise RuntimeError(f"Worktree path exists but is not a git worktree: {worktree_path}")
        return worktree_path, branch

    # Branch may already exist from prior runs.
    branch_exists = _run(["git", "show-ref", "--verify", "--quiet", f"refs/heads/{branch}"], cwd=repo_root, check=False).returncode == 0
    if branch_exists:
        _run(["git", "worktree", "add", str(worktree_path), branch], cwd=repo_root, check=True)
    else:
        _run(["git", "worktree", "add", "-b", branch, str(worktree_path), base_branch], cwd=repo_root, check=True)
    return worktree_path, branch


def _remove_worktree(repo_root: Path, worktree: str) -> None:
    worktree_path = (repo_root / worktree).resolve()
    if not worktree_path.exists():
        return
    _run(["git", "worktree", "remove", "--force", str(worktree_path)], cwd=repo_root, check=False)


def _fast_forward_merge(repo_root: Path, *, base_branch: str, integration_branch: str) -> None:
    _run(["git", "checkout", base_branch], cwd=repo_root, check=True)
    _run(["git", "merge", "--ff-only", integration_branch], cwd=repo_root, check=True)


def _make_prompt_text(
    *,
    repo_root: Path,
    task_id: str,
    worktree_path: Path | None,
    base_branch: str,
    kickoff_ref: str,
    kickoff_text: str,
) -> str:
    wt = str(worktree_path) if worktree_path else str(repo_root)
    return "\n".join(
        [
            f"You are a coding agent executing exactly one task: {task_id}.",
            f"Base repo: {repo_root}",
            f"Task worktree: {wt}",
            f"Base branch: {base_branch}",
            "",
            "Hard rules:",
            "- Do not proceed to any other task IDs.",
            "- Do NOT edit feature docs, task tracking, or session logs:",
            "  - docs/project_management/next/**/tasks.json",
            "  - docs/project_management/next/**/session_log.md",
            "- Do NOT create/remove git worktrees; the orchestrator handles that.",
            "- Do NOT update task statuses; the orchestrator handles that.",
            "- Do NOT run `git checkout` / `git pull` or otherwise switch branches; the orchestrator already prepared the worktree on the task branch.",
            "- Work only in the provided worktree (git repo cwd).",
            "- Run the required commands listed under 'Commands (required)' in the kickoff prompt.",
            "- End with a concise report including: files changed, branch/worktree, commits, commands run + pass/fail, and any blockers.",
            "",
            f"Kickoff prompt path: {kickoff_ref}",
            "",
            "Kickoff prompt (verbatim):",
            kickoff_text.strip(),
            "",
        ]
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Orchestrate this repo's triad tasks.json with low-poll monitoring.")
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--queue", required=True, help="Path to docs/project_management/.../tasks.json")
    parser.add_argument("--run-root", default=".runs", help="Repo-relative run state root")
    parser.add_argument("--max-workers", type=int, default=2, help="Global max parallel workers")
    parser.add_argument("--per-workstream", type=int, default=1, help="Max concurrent tasks per workstream")
    parser.add_argument("--watch-timeout-s", type=int, default=600, help="Fallback wait window (default 10m)")
    parser.add_argument("--stop-on-blocked", action="store_true", help="Stop the run when any task blocks")
    parser.add_argument("--dry-run", action="store_true", help="Print what would run, do not spawn workers")
    parser.add_argument("--only-task-ids", default="", help="Comma-separated allowlist of task IDs to run")
    parser.add_argument("--id-regex", default="", help="Regex allowlist for task IDs to run (applied after only-task-ids)")
    parser.add_argument(
        "--codex-cmd",
        default="codex exec --dangerously-bypass-approvals-and-sandbox",
        help="Codex command prefix (default enables git + .git writes in worktrees)",
    )
    args = parser.parse_args(argv)

    repo_root = Path(args.repo_root).resolve()
    queue_path = Path(args.queue).resolve()
    run_root = (repo_root / args.run_root).resolve()

    spawn_script = (repo_root / "scripts" / "spawn_worker.py").resolve()
    if not args.dry_run:
        run_root.mkdir(parents=True, exist_ok=True)
        if not spawn_script.exists():
            print(f"Missing spawn script: {spawn_script}", file=sys.stderr)
            return 2

    base_branch = _git_current_branch(repo_root)

    only_ids: set[str] = {x.strip() for x in args.only_task_ids.split(",") if x.strip()} if args.only_task_ids else set()
    id_re = re.compile(args.id_regex) if args.id_regex else None

    session_log = queue_path.parent / "session_log.md"
    track_paths = [queue_path]
    if session_log.exists():
        track_paths.append(session_log)

    running: dict[str, subprocess.Popen[bytes]] = {}
    task_to_worktree: dict[str, str] = {}
    task_base_sha: dict[str, str] = {}

    while True:
        payload = _read_json(queue_path)
        tasks_all = _load_tasks(payload, repo_root)
        tasks = [
            t
            for t in tasks_all
            if (not only_ids or t.id in only_ids)
            and (id_re is None or id_re.search(t.id) is not None)
        ]

        done = {t.id for t in tasks_all if t.status == "completed"}
        blocked = [t for t in tasks_all if t.status == "blocked" and (not only_ids or t.id in only_ids)]
        if blocked and args.stop_on_blocked:
            print(f"STOP: {len(blocked)} blocked task(s).")
            return 1

        runnable = _runnable_tasks(tasks, done)

        # Dry-run is a pure scheduling preview: do not mutate queue state, logs, worktrees, or
        # run-root artifacts.
        if args.dry_run:
            active_streams = _active_workstreams(payload)
            planned: list[Task] = []
            for t in runnable:
                if len(planned) >= int(args.max_workers):
                    break
                if int(args.per_workstream) > 0 and t.workstream_id in active_streams:
                    continue
                planned.append(t)
                active_streams.add(t.workstream_id)

            if not planned:
                print("DRY RUN: no runnable tasks (within scope).")
                return 0

            for t in planned:
                planned_cwd = repo_root / t.worktree if t.worktree and t.worktree.strip().upper() != "N/A" else repo_root
                print(f"DRY RUN: would spawn {t.id} ({t.workstream_id}) in {planned_cwd}")
            return 0

        if not runnable and not running:
            print("DONE: no runnable tasks and no running workers (within scope).")
            return 0

        active_streams = _active_workstreams(payload)

        spawned_any = False
        for t in runnable:
            if t.id in running:
                continue
            if len(running) >= int(args.max_workers):
                break
            if int(args.per_workstream) > 0 and t.workstream_id in active_streams:
                continue

            run_dir = run_root / t.id
            prompt_path = run_dir / "prompt.md"
            done_path = run_dir / f"{t.id}.done"
            last_message_path = run_dir / "last_message.md"

            run_dir.mkdir(parents=True, exist_ok=True)
            if done_path.exists():
                done_path.unlink()

            kickoff_text = _load_kickoff_text(repo_root, t.kickoff_ref)
            worktree_path: Path | None = None
            worktree_branch: str | None = None

            # Docs START (orchestration branch).
            _update_task(payload, t.id, {"status": "in_progress", "started_at": _utc_now()})
            if session_log.exists():
                _session_log_start(
                    session_log=session_log,
                    task_id=t.id,
                    role=_role_label(t.type),
                    base_branch=base_branch,
                    kickoff_ref=t.kickoff_ref,
                    worktree=t.worktree if t.worktree and t.worktree.strip().upper() != "N/A" else "",
                )
            _write_json(queue_path, payload)
            if not args.dry_run:
                _git_commit_paths(repo_root, track_paths, f"docs: start {t.id}")

            # Create worktree AFTER the docs start commit so task branches include it. This is
            # required for integration tasks to be fast-forward mergeable back into the base branch.
            if t.worktree and t.worktree.strip().upper() != "N/A":
                worktree_path, worktree_branch = _ensure_worktree(
                    repo_root=repo_root,
                    base_branch=base_branch,
                    worktree=t.worktree,
                )
                task_to_worktree[t.id] = t.worktree
                if worktree_branch:
                    sha = _run(["git", "rev-parse", worktree_branch], cwd=repo_root, check=True).stdout.strip()
                    task_base_sha[t.id] = sha
                    (run_dir / "base_sha.txt").write_text(sha + "\n", encoding="utf-8")

            prompt_text = _make_prompt_text(
                repo_root=repo_root,
                task_id=t.id,
                worktree_path=worktree_path,
                base_branch=base_branch,
                kickoff_ref=t.kickoff_ref,
                kickoff_text=kickoff_text,
            )
            prompt_path.write_text(prompt_text, encoding="utf-8")

            if args.dry_run:
                print(f"DRY RUN: would spawn {t.id} ({t.workstream_id}) in {worktree_path or repo_root}")
                active_streams.add(t.workstream_id)
                continue

            cmd = [
                sys.executable,
                str(spawn_script),
                "--repo-root",
                str(worktree_path or repo_root),
                "--task-id",
                t.id,
                "--run-dir",
                str(run_dir),
                "--codex-cmd",
                args.codex_cmd,
            ]
            proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            running[t.id] = proc
            spawned_any = True
            active_streams.add(t.workstream_id)

        if not spawned_any and running:
            _file_watch_once(run_root, timeout_s=int(args.watch_timeout_s))

        finished: list[str] = []
        for task_id, proc in list(running.items()):
            run_dir = run_root / task_id
            done_path = run_dir / f"{task_id}.done"
            log_path = run_dir / "worker.log"
            failure_path = run_dir / "failure.md"

            if done_path.exists():
                finished.append(task_id)
                continue

            if proc.poll() is not None and not done_path.exists():
                tail = _tail_lines(log_path, 200)
                failure_path.write_text(
                    f"# Worker exited without DONE\n\nfinished_at: {_utc_now()}\n\n## Last 200 log lines\n\n```text\n{tail}```\n",
                    encoding="utf-8",
                )
                payload = _read_json(queue_path)
                _update_task(
                    payload,
                    task_id,
                    {
                        "status": "blocked",
                        "blocked_at": _utc_now(),
                        "blockers": ["Worker exited without writing DONE sentinel."],
                        "unblock_steps": [f"Inspect {log_path}", f"Inspect {failure_path}", "Re-run task with revised prompt."],
                    },
                )
                _write_json(queue_path, payload)
                finished.append(task_id)

        for task_id in finished:
            proc = running.pop(task_id, None)
            if proc is not None:
                proc.poll()

            run_dir = run_root / task_id
            done_path = run_dir / f"{task_id}.done"
            log_path = run_dir / "worker.log"
            last_message_path = run_dir / "last_message.md"

            done_info: dict[str, str] = {}
            if done_path.exists():
                for line in done_path.read_text(encoding="utf-8", errors="replace").splitlines():
                    if "=" in line:
                        k, v = line.split("=", 1)
                        done_info[k.strip()] = v.strip()

            payload = _read_json(queue_path)
            tasks_all = _load_tasks(payload, repo_root)
            this = next((t for t in tasks_all if t.id == task_id), None)
            role = _role_label(this.type if this else "")
            wt = task_to_worktree.get(task_id, this.worktree if this else "")
            wt = wt if wt and wt.strip().upper() != "N/A" else ""

            status = (done_info.get("status") or "").strip().lower()
            if status != "success":
                _update_task(
                    payload,
                    task_id,
                    {
                        "status": "blocked",
                        "blocked_at": _utc_now(),
                        "blockers": [f"Worker status={status or 'unknown'} (see .runs)."],
                        "unblock_steps": [f"Inspect {log_path}", f"Inspect {done_path}", "Adjust prompt and rerun."],
                    },
                )
                _write_json(queue_path, payload)
                if session_log.exists():
                    _session_log_end(
                        session_log=session_log,
                        task_id=task_id,
                        role=role,
                        worktree=wt,
                        last_message_path=last_message_path,
                        extra=[f"- Orchestrator: marked task blocked (worker status={status or 'unknown'})"],
                    )
                _git_commit_paths(repo_root, track_paths, f"docs: finish {task_id} (blocked)")
                continue

            # Require a commit on the task branch when a worktree is involved. This prevents
            # "success" exits that still failed to commit due to sandboxed .git restrictions.
            if this and wt:
                branch = Path(wt or this.worktree).name
                base_sha = task_base_sha.get(task_id) or (run_dir / "base_sha.txt").read_text(encoding="utf-8").strip()
                branch_sha = _run(["git", "rev-parse", branch], cwd=repo_root, check=False).stdout.strip()
                if not branch_sha or branch_sha == base_sha:
                    _update_task(
                        payload,
                        task_id,
                        {
                            "status": "blocked",
                            "blocked_at": _utc_now(),
                            "blockers": [f"No commit produced on branch '{branch}' (likely commit failed)."],
                            "unblock_steps": [
                                f"Inspect {run_dir / 'last_message.md'}",
                                f"Inspect {run_dir / 'worker.log'}",
                                "Re-run the task with a less-restrictive codex sandbox.",
                            ],
                        },
                    )
                    _write_json(queue_path, payload)
                    if session_log.exists():
                        _session_log_end(
                            session_log=session_log,
                            task_id=task_id,
                            role=role,
                            worktree=wt,
                            last_message_path=last_message_path,
                            extra=["- Orchestrator: marked task blocked (no commit produced)"],
                        )
                    _git_commit_paths(repo_root, track_paths, f"docs: finish {task_id} (blocked)")
                    # Keep worktree for inspection.
                    continue

            # If this is an integration task, fast-forward merge integration branch to base branch.
            extra: list[str] = []
            if this and this.type.strip().lower() == "integration" and wt:
                integration_branch = Path(wt or this.worktree).name
                try:
                    _fast_forward_merge(repo_root, base_branch=base_branch, integration_branch=integration_branch)
                    extra.append(f"- Orchestrator: fast-forward merged `{integration_branch}` → `{base_branch}`")
                except subprocess.CalledProcessError as exc:
                    _update_task(
                        payload,
                        task_id,
                        {
                            "status": "blocked",
                            "blocked_at": _utc_now(),
                            "blockers": [f"Failed ff-merge {integration_branch} into {base_branch}: rc={exc.returncode}"],
                            "unblock_steps": ["Inspect git history", "Resolve merge/rebase, then rerun integration task."],
                        },
                    )
                    _write_json(queue_path, payload)
                    if session_log.exists():
                        _session_log_end(
                            session_log=session_log,
                            task_id=task_id,
                            role=role,
                            worktree=wt,
                            last_message_path=last_message_path,
                            extra=[f"- Orchestrator: marked task blocked (ff-merge failed: `{integration_branch}` → `{base_branch}`)"],
                        )
                    _git_commit_paths(repo_root, track_paths, f"docs: finish {task_id} (blocked)")
                    continue

            # Docs END (orchestration branch).
            _update_task(payload, task_id, {"status": "completed", "completed_at": _utc_now()})
            if session_log.exists():
                _session_log_end(
                    session_log=session_log,
                    task_id=task_id,
                    role=role,
                    worktree=wt,
                    last_message_path=last_message_path,
                    extra=extra,
                )
            _write_json(queue_path, payload)
            _git_commit_paths(repo_root, track_paths, f"docs: finish {task_id}")

            # Remove worktree after docs commit (code/test/integration tasks expect cleanup).
            if wt:
                _remove_worktree(repo_root, wt)


if __name__ == "__main__":
    raise SystemExit(main())
