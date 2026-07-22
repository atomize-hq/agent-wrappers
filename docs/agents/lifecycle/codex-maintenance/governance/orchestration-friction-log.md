# Orchestration friction log — codex maintenance

Factual process-improvement log maintained by the lead orchestration session.
Not a generated lifecycle artifact; must not widen the active maintenance
scope. Each entry records observed friction with evidence and a proposed
future fix.

## Entry 1 — maintenance-watch aborts the whole queue on one agent's fetch failure

- Workflow step / timestamp: establish-maintenance-truth, 2026-07-21T (local run).
- Observed issue: `cargo run -p xtask -- maintenance-watch --emit-json /tmp/uaa-maintenance-watch.json`
  exits 2 without emitting any queue JSON because the `claude_code` release
  listing fetch fails (`curl` exit 56, HTTP 401). The `codex` lane (GitHub
  releases) resolves fine but its queue entry is lost.
- Evidence: watcher stderr `curl failed for
  https://storage.googleapis.com/storage/v1/b/claude-code-dist-86c565f3-f756-42ad-8dfa-d59b1c096819/o?prefix=claude-code-releases/
  with exit 56: ... 401`; direct curl of that URL returns HTTP 401
  ("Anonymous caller does not have storage.objects.list access").
  `build_watch_queue_with_resolver` (crates/xtask/src/agent_maintenance/watch.rs)
  propagates the first resolver error for any enrolled agent.
- Impact: the repository's authoritative release-detection surface cannot emit
  codex truth at all; operator must reproduce the codex resolver by hand to
  unblock an otherwise-healthy lane. Also breaks scheduled CI (see Entry 2).
- Workaround used: ran the watcher's exact codex fetch out-of-band
  (`https://api.github.com/repos/openai/codex/releases?per_page=100`, tag
  prefix `rust-v`, drafts/prereleases excluded, semver-sorted,
  `latest_stable_minus_one` selection) to obtain watcher-equivalent values:
  latest_stable `0.145.0`, target `0.144.6`, current_validated `0.125.0`.
- Likely root cause: (a) upstream GCS bucket for claude_code releases no
  longer allows anonymous `storage.objects.list`; (b) watcher design is
  all-or-nothing across agents with no per-agent error isolation and no
  `--agent` filter.
- Proposed fix / owner / priority: xtask `maintenance-watch` should isolate
  per-agent resolver failures (report them in the queue payload) and/or grow
  an `--agent` filter; claude_code `release_watch.upstream` needs a reachable
  source (owner: wrappers team / xtask). Priority: high.
- Blocking: worked around for this run; fix is follow-up only.

## Entry 2 — scheduled release-watch CI failing daily; packet PRs stopped 2026-06-01

- Workflow step / timestamp: establish-maintenance-truth, 2026-07-21.
- Observed issue: `.github/workflows/agent-maintenance-release-watch.yml` runs
  conclude `failure` daily (verified 2026-07-14 through 2026-07-21; the
  newest `agent-maintenance-open-pr.yml` success is 2026-06-01). No packet PRs
  have been opened since.
- Evidence: `gh run list --workflow=agent-maintenance-release-watch.yml`
  (8/8 recent runs failed); `gh run list --workflow=agent-maintenance-open-pr.yml`
  (last successes 2026-06-01). Consistent with Entry 1's 401 onset.
- Impact: ~7 weeks of missed automated maintenance detection across all
  enrolled agents; the codex lane silently fell from target 0.134.0 to a
  0.125.0 baseline vs 0.145.0 latest stable.
- Workaround used: none possible from this repo session; local truth derived
  manually (Entry 1).
- Likely root cause: same as Entry 1.
- Proposed fix / owner / priority: same as Entry 1, plus an alerting hook so a
  failing watch workflow pages someone instead of failing quietly. Priority:
  high.
- Blocking: follow-up only.

## Entry 3 — stale frozen request and five superseded open packet PRs

- Workflow step / timestamp: establish-maintenance-truth, 2026-07-21.
- Observed issue: the committed frozen request
  (`governance/maintenance-request.toml`) targets `0.129.0` (recorded
  2026-05-14), while watcher-equivalent truth today selects `0.144.6`. Open
  packet PRs exist for codex `0.130.0` (#117), `0.131.0` (#121), `0.132.0`
  (#126), `0.133.0` (#130), `0.134.0` (#135) — all unlanded and all stale
  relative to current truth. A local proof-run branch
  (`codex/live-codex-maintenance-0.129.0`) also exists as historical evidence.
- Evidence: request TOML `target_version = "0.129.0"`; `gh pr list` output;
  release fetch in Entry 1.
- Impact: control-plane ambiguity — five concurrent "current" packets for one
  agent, none matching truth; operator must adjudicate supersession manually.
- Workaround used: treated all open packet PRs and the local 0.129.0 branch as
  evidence only (superseded); regenerated the request via
  `prepare-agent-maintenance` per the documented stale-request path. No remote
  PRs/branches were closed, deleted, or modified.
- Likely root cause: the open-pr workflow opens a new packet PR per newly
  detected target but nothing closes/supersedes older unlanded packet PRs for
  the same agent.
- Proposed fix / owner / priority: supersession policy in the open-pr workflow
  (close or retitle older open packet PRs for the same agent when a newer
  target is detected), and a watcher check that flags multiple concurrent open
  packet PRs per agent (owner: wrappers team / CI). Priority: medium.
- Blocking: worked around for this run; fix is follow-up only.

## Entry 4 — documented executor write command omits the required `--run-id`

- Workflow step / timestamp: execute-canonical-lane, 2026-07-21.
- Observed issue: the runbook form `cargo run -p xtask -- execute-agent-maintenance
  --request ... --write` is refused: "--run-id is required with --write so the
  relay can validate against one prepared dry-run baseline" (exit 2). Neither
  `HANDOFF.md`, `OPS_PLAYBOOK.md`, nor the operator runbook mention `--run-id`
  or that the dry-run's `run_id` must be threaded into write mode.
- Evidence: refused invocation output above; successful invocation required
  `--run-id 20260721T204156Z` from the dry-run's `run_dir` banner.
- Impact: one wasted write attempt per operator; the coupling between dry-run
  baseline and write mode is discoverable only from the error string.
- Workaround used: reran with the dry-run's printed `run_id`.
- Likely root cause: executor grew a dry-run/write validation handshake but
  the generated packet docs' command templates were not updated.
- Proposed fix / owner / priority: render the `--run-id` requirement into the
  generated HANDOFF/OPS command blocks, or let `--write` default to the most
  recent prepared run for the same request sha (owner: xtask renderer).
  Priority: low.
- Blocking: no.

## Entry 5 — friction log location collides with the executor's dry-run/write baseline check

- Workflow step / timestamp: execute-canonical-lane, 2026-07-21.
- Observed issue: `execute-agent-maintenance --write` refused with "local
  execution-host preflight must not mutate the workspace; changed paths:
  docs/agents/lifecycle/codex-maintenance/governance/orchestration-friction-log.md"
  because this log was edited between the dry-run baseline and write mode. The
  operator-facing guidance requires continuous friction logging at a path
  inside the maintenance root, which the executor snapshots.
- Evidence: write refusal above (exit 2) naming only this file.
- Impact: every mid-lane friction entry invalidates the prepared run and
  forces a fresh dry-run/write cycle.
- Workaround used: append friction entries only immediately before a fresh
  dry-run, then run write with no intervening workspace changes.
- Likely root cause: the baseline check hashes the whole maintenance root,
  including operator-owned (non-generated) governance files.
- Proposed fix / owner / priority: exclude operator-owned, non-generated
  governance files (or this log by name) from the baseline mutation check, or
  document a standard out-of-root location for orchestration logs (owner:
  xtask executor). Priority: medium.
- Blocking: worked around.

## Entry 6 — required-target platform evidence has no working local lane

- Workflow step / timestamp: execute-canonical-lane (second write run), 2026-07-21.
- Observed issue: the packet requires a `x86_64-unknown-linux-musl` snapshot
  (RULES.json `union.required_target`), but every configured execution path
  failed: local AMD64 Podman under QEMU SIGSEGVs `rustc`; configured hosts
  `spenser-linux-codex` (192.168.50.132) and `clawOne` (20.127.187.160) are
  unreachable ("No route to host" / connect timeout, re-verified by the
  orchestrator). The historical CI lane
  (`codex-cli-update-snapshot.yml`) is workflow_dispatch-only and marked
  "historical manual only"; dispatching it or pushing was out of contract for
  this run.
- Evidence: `maintainer-follow-up.md` written by the packet host; orchestrator
  ssh probes; write-run status `written_paths` limited to the darwin snapshot
  and follow-up note.
- Impact: the packet stalls after the darwin snapshot; union/coverage/
  versions/closeout blocked.
- Workaround used: reproduced the CI `union-report-validate` fallback step
  locally with repository tooling only — scratch worktree at candidate commit
  `df0762ea`; pinned upstream asset
  `codex-x86_64-unknown-linux-musl.tar.gz` from `rust-v0.144.6`
  (sha256 `6a9def51a0ad8cea6684d8eb3bf033c89f33e3bc5cfe492f1a1e0a718451a1c6`,
  109369631 bytes); `SOURCE_DATE_EPOCH=1784382712` from the release
  `published_at`; generator invocation identical to the CI recipe. Because
  the Podman machine's `Rosetta: true` flag is inert (only `qemu-*` binfmt
  handlers are registered, so `rustc` still SIGSEGVs in-container), the
  orchestrator cross-compiled the workspace `xtask` on the macOS host to a
  static `x86_64-unknown-linux-musl` binary (`rustup target add` +
  `RUSTFLAGS="-C linker=rust-lld -C target-feature=+crt-static" cargo build
  -p xtask --release --target x86_64-unknown-linux-musl`) and ran it with the
  real pinned codex binary inside a `docker.io/library/alpine` linux/amd64
  container — plain CLI execution works under qemu even though `rustc` does
  not. No hand-authored snapshot content.
- Likely root cause: the maintenance relay assumes either CI runners or a
  reachable native Linux host; neither exists in the current local-first
  transport, and the packet host does not know about the Rosetta-backed
  container path.
- Proposed fix / owner / priority: teach the executor (or a documented
  runbook step) the containerized required-target generation path, or
  re-enable a CI lane for platform evidence (owner: xtask / wrappers team).
  Priority: high.
- Blocking: worked around for this run.

## Entry 6a — refresh-agent cannot represent a completed uplift run

- Workflow step / timestamp: post-uplift validation, 2026-07-21.
- Observed issue: after the packet host landed all 68 required uplifts (the
  coverage report shows 0 missing rows), `refresh-agent --dry-run` refuses:
  "field `support_surface_audit` must match the shared derived maintenance
  contract". The frozen request records the pre-uplift discovery (68 rows);
  the post-uplift derivation records none; the tool has no state
  distinguishing "stale request" from "request satisfied by landed uplifts",
  so packet docs cannot be regenerated after the work lands.
- Evidence: refusal output above; identical mechanism previously (correctly)
  blocked the run when the mismatch pointed the other way.
- Impact: any post-uplift packet-doc refresh is impossible without rewriting
  the frozen request to a zero-discovery state, which would erase the run's
  audit record; operators must treat the frozen 68-row request as the
  historical contract and skip refresh.
- Workaround used: left the frozen request untouched as the run's contract of
  record and proceeded to review/closeout, which own post-run validation.
- Likely root cause: the derived-audit equality check compares against live
  repo state with no "satisfied" classification for rows whose uplift landed
  in this same run.
- Proposed fix / owner / priority: teach the derivation to classify frozen
  required-uplift rows that are now covered as `satisfied` (still matching),
  or add an explicit post-run mode (owner: xtask agent-maintenance).
  Priority: medium.
- Blocking: no.

## Entry 6b — watcher release-history window is first-page-only

- Workflow step / timestamp: establish-maintenance-truth, 2026-07-21.
- Observed issue: `fetch_github_releases` requests
  `releases?per_page=100` without pagination. For openai/codex the first page
  currently yields only 17 stable `rust-v` releases (oldest `0.140.0`).
- Evidence: crates/xtask/src/agent_maintenance/watch.rs (single fetch, no
  `page` loop); live fetch count above.
- Impact: none for `latest_stable_minus_one` today, but any future policy or
  audit that needs deeper history (or a repo whose page 1 is all
  drafts/prereleases) would silently compute from a truncated window.
- Workaround used: none needed.
- Likely root cause: minimal first implementation.
- Proposed fix / owner / priority: paginate or document the window as a
  deliberate contract (owner: xtask). Priority: low.
- Blocking: follow-up only.

## Entry 6c — worker worktrees are created at session-start HEAD, not the current candidate

- Workflow step / timestamp: remediation dispatch, 2026-07-21.
- Observed issue: background worker worktrees were provisioned at the
  session-start commit (`1ae6ee12`, staging HEAD) rather than the branch tip
  current at dispatch time. The parser-fix packet ran against a 1-commit-stale
  base (benign, docs-only delta) and the review-remediation packet initially
  blocked outright because its base `0b613a42` and the packet's target files
  did not exist in the worktree; the worker also mis-read the ancestry
  (inverted `merge-base --is-ancestor` direction) and reported false
  divergence.
- Evidence: worker reports for both packets; ancestry verified by the lead
  (`1ae6ee12` is an ancestor of `0b613a42`; the worktree branch had no unique
  commits).
- Impact: one fully wasted worker round-trip plus operator reconciliation on
  every packet whose base advanced mid-session.
- Workaround used: instructed the worker to fast-forward its own worktree
  branch to the candidate SHA and relaunch the identical packet.
- Likely root cause: worktree provisioning snapshots the session-start HEAD;
  packets must therefore state an explicit checkout step, and workers must
  verify base revision before launching (one did, one did not).
- Proposed fix / owner / priority: make the worker launcher (or agent
  definition) check out the packet's stated base revision as a hard precondition
  and fail fast with a clear message (owner: .claude agents / launcher script).
  Priority: medium.
- Blocking: worked around.
- Escalation (same run): resuming a completed worktree-isolated agent is
  dangerous — the harness auto-cleans an unchanged worktree at completion, so
  the resumed agent's relative paths silently resolved to the MAIN repository
  checkout. The resumed worker switched the primary checkout from the
  candidate branch to `staging` and stashed the operator's uncommitted
  friction-log edit before stopping to ask for direction. No commits or refs
  were lost; the lead restored the branch and popped the stash. Rule adopted
  for the rest of the run: never resume a completed worktree-isolated worker
  for new work — always spawn a fresh agent (fresh worktree); workers must
  verify `git rev-parse --git-dir`/worktree identity and the packet base SHA
  before any git mutation. Proposed fix: keep the worktree alive while an
  agent can still be resumed, or fail resumes whose worktree is gone (owner:
  harness/agent definitions). Priority: high.

## Entry 7a — executor's Codex host leaked an orphaned, unsandboxed nested process pair

- Workflow step / timestamp: execute-canonical-lane / remediation, 2026-07-21
  (observed ~17:56 local, spawned 16:54 local).
- Observed issue: during `execute-agent-maintenance --write`, the Codex host
  (itself running `--dangerously-bypass-approvals-and-sandbox`) spawned a
  nested `xtask execute-agent-maintenance --dry-run --codex-binary ...` whose
  own `codex exec --dangerously-bypass-approvals-and-sandbox --cd <main repo>`
  child survived the parent run's completion (both re-parented to PID 1) and
  kept running ~60 minutes later.
- Evidence: `ps -eo pid,ppid,lstart,command` showed PID 4379 (nested dry-run,
  ppid 1, started 16:54:29) and child PID 4821 (codex exec, started 16:54:51)
  alive at 17:56 while the write run had finished at 17:21.
- Impact: an unsandboxed agent process kept running against the main
  workspace after the run ended (mutation risk; none materialized — tracked
  tree verified clean), and a downstream worker's process-exit monitor keyed
  on `codex exec` was deadlocked by the stale process.
- Workaround used: verified main-repo cleanliness, then killed exactly PIDs
  4379/4821.
- Likely root cause: execute-agent-maintenance does not run its host in a
  managed process group and does not reap descendants on exit; the host
  prompt does not forbid leaving background probes running.
- Proposed fix / owner / priority: spawn the host in its own process group
  and terminate the group when the run concludes; consider prompt guidance
  against leaving background probes running (owner: xtask executor).
  Priority: medium-high.
- Blocking: worked around; fix is follow-up only.

## Entry 7 — Codex snapshot discovery mistakes a wrapped description for a subcommand

- Workflow step / timestamp: version-scoped artifact refresh, 2026-07-21.
- Observed issue: the frozen generator command for the locally installed
  `codex-cli 0.144.6` aborts before writing the macOS target snapshot:
  `cargo run -p xtask -- codex-snapshot --codex-binary /opt/homebrew/bin/codex
  --out-file cli_manifests/codex/snapshots/0.144.6/aarch64-apple-darwin.json
  --supplement cli_manifests/codex/supplement/commands.json` fails while trying
  to execute `codex help app-server daemon daemon` (exit 2, unrecognized
  subcommand `daemon`).
- Evidence: `codex help app-server daemon` lists valid daemon children, but the
  wrapped continuation line `daemon` in the description for
  `enable-remote-control` is parsed as another command token by
  `crates/xtask/src/codex_snapshot/discovery.rs`.
- Impact: no trustworthy `0.144.6` snapshot, union, coverage report, or version
  metadata can be created. Consequently, the target-to-baseline non-TUI diff
  cannot be established and no maintenance closeout is permitted.
- Workaround used: none. Hand-authoring or copying an artifact would defeat the
  packet's upstream-evidence requirement.
- Likely root cause: command discovery accepts any indented, token-like line
  in a `Commands:` section without distinguishing wrapped continuation lines
  from command rows.
- Proposed fix / owner / priority: make the xtask help parser retain the
  command-column alignment (and add a regression fixture for the wrapped
  `daemon` line), then rerun this unchanged frozen request (owner: xtask /
  wrappers team). Priority: high.
- Blocking: yes — the needed xtask change is outside this packet's frozen
  writable envelope; treat it as the packet's allowed
  `outside_registry_maintenance_write_envelope` follow-up.

## Entry 8 — sandboxed Codex worker cannot commit inside its own worktree; wrapper agent ends before collecting the result

- Workflow step / timestamp: review-remediation dispatch, 2026-07-22.
- Observed issue: the remediation worker ran the Codex CLI under
  `--sandbox workspace-write` whose writable roots were `[workdir, /tmp,
  $TMPDIR]`. Codex completed all functional work and self-verified green, but
  the final `git commit` failed: `fatal: Unable to create
  '.../.git/worktrees/agent-a136c0118b9272b87/index.lock': Operation not
  permitted`. A linked worktree keeps its index under the MAIN repo's
  `.git/worktrees/<name>/`, which is outside the sandbox's writable roots, so
  no commit could be produced. Separately, the wrapping Claude sub-agent
  reached `end_turn` ("still running… I'll resume once notified") and its task
  was marked completed without ever re-waking to collect Codex's final
  message; its last logged report is therefore stale and carries no commit
  hash or verification evidence.
- Evidence: Codex `--output-last-message` file shows all four findings
  implemented, `cargo fmt/clippy/test/support-matrix --check/codex-validate`
  passing (under the correct package id `unified-agent-api-codex`), and the
  explicit `index.lock … Operation not permitted` commit failure; the
  sub-agent transcript ends at the pre-completion "still running" message.
- Impact: the lead cannot rely on worker prose for the commit; the candidate
  exists only as uncommitted working-tree changes in the linked worktree and
  must be integrated by the (unsandboxed) lead session. One monitor round-trip
  was effectively wasted.
- Workaround used: lead inspected the worktree diff directly with
  `git -C <worktree> diff HEAD`, verified it against the allowed write set and
  the four findings, and integrated it into the candidate branch from the main
  repo (which is not sandboxed) rather than resuming the worker.
- Likely root cause: `workspace-write` sandbox roots do not include the linked
  worktree's git index directory under the main `.git/worktrees/<name>/`; and
  the worker launch pattern lets the wrapping agent reach `end_turn` before the
  background completion monitor fires, so results are never harvested.
- Proposed fix / owner / priority: either add the worktree's
  `.git/worktrees/<name>/` index path to the sandbox writable set, or have the
  launcher stage-and-commit outside the sandbox after Codex exits; and require
  the wrapping agent to block on the completion monitor (not `end_turn`) before
  reporting (owner: launcher script / agent definitions). Priority: medium.
- Blocking: worked around — no closeout impact.

## Entry 9 — regenerating coverage without re-deriving the audit silently invalidates the frozen request at closeout

- Workflow step / timestamp: closeout, 2026-07-22.
- Observed issue: `close-agent-maintenance` refused the run with
  `field support_surface_audit must match the shared derived maintenance
  contract`. The passthrough-surface commit (`0b613a42`) had regenerated
  `cli_manifests/codex/reports/0.144.6/coverage.any.json` to record the 68
  discovered surfaces as covered passthrough, but did not re-derive the
  request's `support_surface_audit`. Because `derive_support_surface_audit`
  reads `coverage.any.json`, the frozen request (68 discovered / 68 required
  uplifts, last regenerated at `b430c419`) no longer matched the live
  derivation (0 discovered / 0 required uplifts), and every request load —
  including closeout — failed the equality check.
- Evidence: `git log staging..HEAD -- coverage.any.json` shows `0b613a42`
  touched it after the last request regen `b430c419`; a fresh
  `prepare-agent-maintenance --write` with the identical frozen identity
  values re-derived the audit to `discovered_upstream_surface=0`,
  `required_uplifts_this_run=0`, `deferred_preexisting_gaps=2`; no identity
  field changed.
- Impact: closeout is blocked until the audit is reconciled; none of the
  green gates catch this drift because they validate manifests/publications,
  not request-vs-derivation consistency.
- Workaround used: regenerated the request and packet docs via
  `prepare-agent-maintenance --dry-run`/`--write` then `refresh-agent`
  (idempotent), re-ran green gates, then recorded the closeout.
- Likely root cause: the step that lands the passthrough surface and
  regenerates coverage does not also re-run `prepare-agent-maintenance`
  (or an audit re-derive) to keep `support_surface_audit` in sync with the
  coverage it just rewrote; the frozen contract and its own derivation
  inputs drift apart within a single run.
- Proposed fix / owner / priority: after any coverage/gap regeneration,
  re-derive `support_surface_audit` (or fail a pre-closeout consistency
  check that runs `validate_support_surface_audit` and points at
  `prepare-agent-maintenance`) so the request never lags its derivation
  inputs (owner: xtask executor / agent-maintenance). Priority: medium-high.
- Blocking: worked around — closeout completed after reconciliation.
