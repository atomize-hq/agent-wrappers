# Agent Registry Contract

Status: Normative  
Scope: schema and ownership rules for `crates/xtask/data/agent_registry.toml`

## Normative language

This document uses RFC 2119 requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Purpose

Define the committed control-plane truth used by `xtask` for onboarded agents, including the
maintenance governance metadata consumed by `check-agent-drift` and the transport facts used to
derive one shared automated maintenance packet contract.

## Registry ownership

`crates/xtask/data/agent_registry.toml` is the canonical committed source of truth for:

- onboarded agent identity and repo paths
- capability declaration shape owned by the control plane
- publication flags and release track enrollment
- onboarding packet ownership
- maintenance governance checks for already-onboarded agents
- maintenance release-watch enrollment and upstream-watch metadata

Generated docs and maintenance packets MAY reference this registry, but they MUST NOT redefine its
schema.

## Entry shape

Each `[[agents]]` entry MUST continue to declare the existing identity, capability, publication,
release, and scaffold fields enforced by `xtask`.

Downstream `xtask` commands that materialize wrapper-crate files MUST use the entry's `crate_path`
directly. They MUST NOT derive a second crate-location contract from `agent_id`.

When wrapper scaffolding derives the crate-local Rust `[lib].name`, it MUST use the final
`crate_path` path component as the source basename, normalize `-` to `_`, and then require the
normalized result to match ASCII `[A-Za-z0-9_]+`. Hyphenated crate directories are therefore
valid location contracts, but basenames containing other punctuation, whitespace, or non-ASCII
characters are invalid for scaffoldable registry entries.

If capability publication is enabled for an agent, the registry publication block is also the
canonical source of the target-scoped publication contract. In particular:

- `publication.capability_matrix_target` MAY be omitted when the agent does not require a
  target-scoped capability publication declaration.
- `publication.capability_matrix_target` MUST be present when
  `publication.capability_matrix_enabled = true` and publication truth depends on a specific
  declared target.
- when present, `publication.capability_matrix_target` MUST equal one entry from
  `canonical_targets`
- target ordering in `canonical_targets` MUST NOT be treated as an implicit publication-selection
  contract

Registry-controlled publication truth for capability advertising MUST be derived from the shared
projection contract reused by publication generation and maintenance drift/closeout checks; callers
MUST NOT restate config-gated capability semantics independently.

If maintenance governance auditing is configured for an agent, it MUST live under:

```toml
[agents.maintenance]
[[agents.maintenance.governance_checks]]
```

If maintenance release-watch enrollment is configured for an agent, it MUST live under:

```toml
[agents.maintenance.release_watch]
[agents.maintenance.release_watch.upstream]
```

Absence of `maintenance.release_watch` is the only “not enrolled” state. Callers MUST NOT create a
second enrollment inventory outside the registry or represent unenrolled agents with
`enabled = false` placeholders.

Approval artifacts and create-lane closeout consumers MUST preserve exactly two maintenance
approval modes:

- `release_watch_enrolled`
- `explicitly_deferred`

These modes do not create a second enrollment contract:

- `release_watch_enrolled` requires committed registry `maintenance.release_watch` truth for the
  same agent
- `explicitly_deferred` forbids committed registry `maintenance.release_watch` truth for the same
  agent
- callers MUST NOT introduce a third approval-maintenance mode or alternate release-watch
  enrollment storage outside the registry

## Maintenance release watch

`maintenance.release_watch` declares the machine-owned watch metadata for upstream release
detection. The schema is:

```toml
[agents.maintenance.release_watch]
enabled = true
version_policy = "latest_stable_minus_one" # or "upstream_stable_pointer"
dispatch_kind = "workflow_dispatch" # or "packet_pr"
dispatch_workflow = "example.yml"    # required only for workflow_dispatch

[agents.maintenance.release_watch.upstream]
source_kind = "github_releases"      # or "gcs_object_listing" or "npm_dist_tag"
```

Required top-level fields:

- `enabled`: boolean. When the block is present, it MUST be `true`.
- `version_policy`: one of `latest_stable_minus_one` or `upstream_stable_pointer`
- `dispatch_kind`: one of `workflow_dispatch` or `packet_pr`

Version-policy rules:

- `latest_stable_minus_one`: target the next-to-latest stable upstream release after the upstream
  history is sorted and deduplicated.
- `upstream_stable_pointer`: target the single upstream-resolved stable-channel version directly;
  no minus-one offset is applied.

Dispatch rules:

- The only live scheduled release-detection entrypoint is `.github/workflows/agent-maintenance-release-watch.yml`. Per-agent watcher workflows MUST NOT be treated as active release-watch entrypoints.
- `dispatch_kind = "packet_pr"` is the steady-state enrolled transport for automated maintenance.
- `dispatch_workflow` MUST be present only when `dispatch_kind = "workflow_dispatch"`.
- `dispatch_workflow` MUST be omitted when `dispatch_kind = "packet_pr"`.
- `dispatch_workflow`, when present, MUST be a non-empty workflow filename.
- The registry omission for `packet_pr` is intentional. Packet generation MUST materialize
  `detected_release.dispatch_workflow = "agent-maintenance-open-pr.yml"` in the request packet so
  the frozen packet still carries one fully resolved dispatch contract.
- `workflow_dispatch` is compatibility-only for historical or manual replay lanes. It MUST NOT
  become a second policy store for support-audit rules, writable-surface narrowing, or gate
  semantics.

Upstream rules:

- `source_kind = "github_releases"` requires:
  - `owner`
  - `repo`
  - `tag_prefix`
- `source_kind = "gcs_object_listing"` requires:
  - `bucket`
  - `prefix`
  - `version_marker`
- `source_kind = "npm_dist_tag"` requires:
  - `package`
  - `dist_tag`
- Source-specific fields from the non-selected source kind MUST NOT be present.

Current committed registry truth enables release-watch metadata for `codex`, `claude_code`, and
`opencode`. Enrolled automated maintenance is expected to converge on `dispatch_kind = "packet_pr"`
for each of those agents. Any temporary or historical `workflow_dispatch` entry exists only as a
compatibility state in the committed registry content, not as a permanent schema-level expectation
for future agents.

## Multi-target parity acquisition

Multi-target parity acquisition — downloading an upstream release for every expected target,
snapshotting each on a matching-OS runner, and merging a complete union — is enabled for an agent
when **both** of the following hold:

1. the agent carries `maintenance.release_watch` in this registry, and
2. the agent's `cli_manifests/<agent>/RULES.json` carries an `acquisition` block.

The registry MUST NOT gain a separate acquisition-enablement field. Such a field would be a second
enrollment inventory, which the “Absence of `maintenance.release_watch` is the only ‘not enrolled’
state” rule above already forbids, and it could disagree with the manifest it claims to describe.

The gate is enforced in one place — `xtask manifest-acquisition-plan` — which fails closed with a
distinct error for each reason (`UnknownAgent`, `NotEnrolled`, `NoAcquisitionBlock`).
`.github/workflows/agent-maintenance-open-pr.yml` treats a clean exit from that command as the
gate, so committed truth is the only input. An agent that satisfies neither condition keeps the
docs-only maintenance path with no behavior change.

Watch and acquire are independent. `maintenance.release_watch.upstream.source_kind` governs
release *detection*; `acquisition.source_kind` in the manifest governs where *binaries* come from.
An agent may legitimately watch one source and acquire from another.

## Maintenance governance checks

Each `governance_checks` entry MUST declare:

- `path`: repo-relative file path to the historical governance surface
- `required`: boolean
- `comparison_kind`: one of:
  - `approved_agent_descriptor`
  - `markdown_capability_claim`
  - `markdown_support_claim`

Additional rules:

- `path` MUST be normalized and repo-relative.
- `path` values MUST be unique within one agent entry.
- `manual_reopen` MUST NOT be modeled in registry metadata; it remains a maintainer-authored
  maintenance-request trigger only.

### `approved_agent_descriptor`

Use this comparison kind only for approval artifacts under:

`docs/agents/lifecycle/<onboarding_pack_prefix>/governance/approved-agent.toml`

Rules:

- `path` MUST match the agent entry’s `scaffold.onboarding_pack_prefix`
- no markdown parser config may be present
- approval and governance comparison MAY omit `capability_matrix_target`; when absent, governance
  comparison MUST treat the field as not asserted rather than as a mismatch
- onboarding approval-mode MUST remain backward-compatible with legacy single-target descriptors,
  while newly generated descriptors MUST include `capability_matrix_target` whenever the registry
  contract requires it

### `markdown_capability_claim`

Use this comparison kind for historical Markdown surfaces that declare capability ids.

Required parser config:

- `start_marker`
- `end_marker`
- `extraction_mode = "inline_code_ids"`

The marked block MUST be the sole machine-audited capability claim for that check.

### `markdown_support_claim`

Use this comparison kind for historical Markdown surfaces that declare support posture.

Required parser config:

- `start_marker`
- `end_marker`
- `extraction_mode = "support_state_lines"`

The marked block MUST contain only structured `key = value` lines. M4 recognizes these keys:

- `backend_support`
- `uaa_support`

## Drift comparison boundary

Maintenance governance checks MUST compare historical governance claims against modeled current
truth, not against generated publication files as their primary source.

Specifically:

- `approved_agent_descriptor` compares against current registry-controlled truth
- `markdown_capability_claim` compares against current capability truth
- `markdown_support_claim` compares against current derived support rows

Prior maintenance closeouts MUST NOT permanently suppress a governance surface. If the same
historical surface drifts again later, `check-agent-drift` MUST report it again.
