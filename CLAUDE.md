# Claude Code Working Agreement

Read and follow [AGENTS.md](AGENTS.md) first. It is the shared repository
policy for both Claude Code and Codex; do not duplicate or override it here.

## Worker orchestration

Use `$orchestrate-workers` when work needs delegation and `$cross-review` for
independent review of a candidate change. The lead session keeps architecture,
integration, and final-verification authority. Delegate only bounded packets
with explicit write ownership.

For a Codex worker, use the `codex-profile-worker` agent and explicitly state
the required Codex profile. The repository launcher rejects omitted profiles.
