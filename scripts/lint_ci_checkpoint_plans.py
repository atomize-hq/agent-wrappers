#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path


_JSON_FENCE_RE = re.compile(r"```json\n(.*?)\n```", re.DOTALL)


@dataclass(frozen=True)
class LintError:
    path: Path
    message: str

    def format(self) -> str:
        return f"{self.path}: {self.message}"


def _extract_json_block(markdown: str, *, path: Path) -> dict:
    match = _JSON_FENCE_RE.search(markdown)
    if not match:
        raise ValueError("missing ```json fenced block")
    try:
        return json.loads(match.group(1))
    except json.JSONDecodeError as e:
        raise ValueError(f"invalid JSON in fenced block: {e}") from e


def _load_tasks_json(tasks_path: Path) -> list[dict]:
    raw = json.loads(tasks_path.read_text(encoding="utf-8"))
    if isinstance(raw, list):
        tasks = raw
    elif isinstance(raw, dict) and isinstance(raw.get("tasks"), list):
        tasks = raw["tasks"]
    else:
        raise ValueError("expected tasks.json to be a JSON array or an object with a top-level 'tasks' array")

    normalized: list[dict] = []
    for i, t in enumerate(tasks):
        if not isinstance(t, dict):
            raise ValueError(f"task at index {i} is not an object")
        normalized.append(t)
    return normalized


def _as_str_list(value: object, *, field: str) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(x, str) for x in value):
        raise ValueError(f"field '{field}' must be an array of strings")
    return list(value)


def lint_ci_checkpoint_plan(plan_path: Path) -> list[LintError]:
    errors: list[LintError] = []
    feature_dir = plan_path.parent
    feature_slug = feature_dir.name

    try:
        doc = plan_path.read_text(encoding="utf-8")
        plan = _extract_json_block(doc, path=plan_path)
    except Exception as e:  # noqa: BLE001
        return [LintError(plan_path, str(e))]

    if not isinstance(plan, dict):
        return [LintError(plan_path, "top-level JSON must be an object")]

    def require(field: str, typ: type) -> object:
        if field not in plan:
            raise ValueError(f"missing required field '{field}'")
        v = plan[field]
        if not isinstance(v, typ):
            raise ValueError(f"field '{field}' must be {typ.__name__}")
        return v

    try:
        version = require("version", int)
        if version != 1:
            raise ValueError("field 'version' must be 1")

        feature = require("feature", str)
        if feature != feature_slug:
            raise ValueError(f"field 'feature' must match feature dir name ('{feature_slug}')")

        min_triads = require("min_triads_per_checkpoint", int)
        max_triads = require("max_triads_per_checkpoint", int)
        if min_triads <= 0 or max_triads <= 0 or min_triads > max_triads:
            raise ValueError("min/max triads per checkpoint must be positive and min <= max")

        slices = _as_str_list(require("slices", list), field="slices")
        if not slices:
            raise ValueError("'slices' must be non-empty")
        if len(set(slices)) != len(slices):
            raise ValueError("'slices' must not contain duplicates")

        checkpoints = require("checkpoints", list)
        if not isinstance(checkpoints, list) or any(not isinstance(cp, dict) for cp in checkpoints):
            raise ValueError("'checkpoints' must be an array of objects")
        if not checkpoints:
            raise ValueError("'checkpoints' must be non-empty")

        tasks_json_wiring = require("tasks_json_wiring", dict)
        checkpoint_tasks_wiring = tasks_json_wiring.get("checkpoint_tasks")
        if not isinstance(checkpoint_tasks_wiring, list) or any(not isinstance(x, dict) for x in checkpoint_tasks_wiring):
            raise ValueError("tasks_json_wiring.checkpoint_tasks must be an array of objects")

        # --- Slice partition validation ---
        seen: set[str] = set()
        slice_order = {s: i for i, s in enumerate(slices)}
        last_max_index = -1
        checkpoint_task_ids: set[str] = set()
        for cp in checkpoints:
            cp_id = cp.get("id")
            if not isinstance(cp_id, str) or not cp_id:
                raise ValueError("each checkpoint must have non-empty string field 'id'")

            group = cp.get("slice_group")
            if not isinstance(group, list) or any(not isinstance(s, str) for s in group) or not group:
                raise ValueError(f"checkpoint '{cp_id}': slice_group must be a non-empty array of strings")

            ending_slice = cp.get("ending_slice")
            if not isinstance(ending_slice, str) or not ending_slice:
                raise ValueError(f"checkpoint '{cp_id}': ending_slice must be a non-empty string")

            if group[-1] != ending_slice:
                raise ValueError(f"checkpoint '{cp_id}': ending_slice must equal last element of slice_group")

            checkpoint_task_id = cp.get("checkpoint_task_id")
            if not isinstance(checkpoint_task_id, str) or not checkpoint_task_id:
                raise ValueError(f"checkpoint '{cp_id}': checkpoint_task_id must be a non-empty string")
            checkpoint_task_ids.add(checkpoint_task_id)

            for s in group:
                if s not in slice_order:
                    raise ValueError(f"checkpoint '{cp_id}': slice '{s}' is not listed in top-level slices")
                if s in seen:
                    raise ValueError(f"slice '{s}' appears in multiple checkpoint groups (overlap)")
                seen.add(s)

            # Ensure groups don't scramble slice order.
            idxs = [slice_order[s] for s in group]
            if idxs != sorted(idxs):
                raise ValueError(f"checkpoint '{cp_id}': slice_group must preserve the order in 'slices'")
            if max(idxs) < last_max_index:
                raise ValueError(f"checkpoint '{cp_id}': checkpoint groups must be in slice order (no backward groups)")
            last_max_index = max(idxs)

            group_len = len(group)
            total_len = len(slices)
            if not (min_triads <= group_len <= max_triads):
                if not (total_len < min_triads and group_len == total_len):
                    raise ValueError(
                        f"checkpoint '{cp_id}': slice_group length {group_len} outside bounds "
                        f"[{min_triads}, {max_triads}] (no exception applies)"
                    )

        if seen != set(slices):
            missing = [s for s in slices if s not in seen]
            extra = [s for s in seen if s not in set(slices)]
            if missing:
                raise ValueError(f"checkpoint groups are missing slices: {missing}")
            if extra:
                raise ValueError(f"checkpoint groups contain unknown slices: {extra}")

        # --- tasks.json wiring validation ---
        tasks_path = feature_dir / "tasks.json"
        if not tasks_path.exists():
            raise ValueError("tasks.json not found next to ci_checkpoint_plan.md")
        tasks = _load_tasks_json(tasks_path)
        by_id: dict[str, dict] = {}
        for t in tasks:
            tid = t.get("id")
            if not isinstance(tid, str) or not tid:
                raise ValueError("all tasks must have a non-empty string 'id'")
            if tid in by_id:
                raise ValueError(f"duplicate task id '{tid}' in tasks.json")
            by_id[tid] = t

        wiring_ids: set[str] = set()
        for w in checkpoint_tasks_wiring:
            tid = w.get("id")
            if not isinstance(tid, str) or not tid:
                raise ValueError("tasks_json_wiring.checkpoint_tasks[].id must be a non-empty string")
            wiring_ids.add(tid)

            dep = w.get("depends_on_integration_task")
            if not isinstance(dep, str) or not dep:
                raise ValueError(f"checkpoint wiring '{tid}': depends_on_integration_task must be a non-empty string")

            blocks = w.get("blocks_next_slice_start")
            if blocks is not None and (not isinstance(blocks, str) or not blocks):
                raise ValueError(f"checkpoint wiring '{tid}': blocks_next_slice_start must be null or a non-empty string")

            if tid not in by_id:
                raise ValueError(f"checkpoint wiring '{tid}': task id not found in tasks.json")
            if dep not in by_id:
                raise ValueError(f"checkpoint wiring '{tid}': depends_on integration task '{dep}' not found in tasks.json")

            depends_on = by_id[tid].get("depends_on")
            if not isinstance(depends_on, list) or any(not isinstance(x, str) for x in depends_on):
                raise ValueError(f"checkpoint task '{tid}': depends_on must be an array of strings")
            if dep not in depends_on:
                raise ValueError(f"checkpoint task '{tid}': must depend_on '{dep}'")

            if blocks is not None:
                if blocks not in by_id:
                    raise ValueError(f"checkpoint wiring '{tid}': blocks_next_slice_start task '{blocks}' not found in tasks.json")
                blocks_depends = by_id[blocks].get("depends_on")
                if not isinstance(blocks_depends, list) or any(not isinstance(x, str) for x in blocks_depends):
                    raise ValueError(f"blocked task '{blocks}': depends_on must be an array of strings")
                if tid not in blocks_depends:
                    raise ValueError(f"blocked task '{blocks}': must depend_on checkpoint task '{tid}'")

        if checkpoint_task_ids != wiring_ids:
            missing = sorted(checkpoint_task_ids - wiring_ids)
            extra = sorted(wiring_ids - checkpoint_task_ids)
            if missing:
                raise ValueError(f"tasks_json_wiring.checkpoint_tasks missing checkpoint_task_id(s): {missing}")
            if extra:
                raise ValueError(f"tasks_json_wiring.checkpoint_tasks has extra task id(s): {extra}")

    except Exception as e:  # noqa: BLE001
        errors.append(LintError(plan_path, str(e)))

    return errors


def main(argv: list[str]) -> int:
    repo_root = Path(__file__).resolve().parent.parent
    next_dir = repo_root / "docs" / "project_management" / "next"
    if not next_dir.exists():
        print("lint-ci-checkpoint-plans: SKIP (docs/project_management/next/ missing)")
        return 0

    plans = sorted(next_dir.glob("*/ci_checkpoint_plan.md"))
    if not plans:
        print("lint-ci-checkpoint-plans: OK (no ci_checkpoint_plan.md files found)")
        return 0

    all_errors: list[LintError] = []
    for plan_path in plans:
        all_errors.extend(lint_ci_checkpoint_plan(plan_path))

    if all_errors:
        print("lint-ci-checkpoint-plans: FAIL")
        for err in all_errors:
            print(f"  - {err.format()}")
        return 1

    print(f"lint-ci-checkpoint-plans: OK ({len(plans)} plan(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

