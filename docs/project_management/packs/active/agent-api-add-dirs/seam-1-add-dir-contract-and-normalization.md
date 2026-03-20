# SEAM-1 — Add-dir contract + normalization semantics

- **Name**: `agent_api.exec.add_dirs.v1`
- **Type**: integration
- **Goal / user value**: pin one backend-neutral meaning for “extra context roots” so callers can
  ask for additional directories without backend-specific drift.

## Scope

- In:
  - Confirm the owner-doc contract in `docs/specs/universal-agent-api/extensions-spec.md`.
  - Pin:
    - closed schema,
    - bounds,
    - trim/resolve/normalize/dedup rules,
    - pre-spawn existence + directory checks,
    - safe error posture,
    - session-flow compatibility.
- Out:
  - Shared code placement for the parser/normalizer.
  - Backend-specific argv wiring.

## Primary interfaces (contracts)

- **Extension key**: `agent_api.exec.add_dirs.v1`
  - **Inputs**:
    - object with required `dirs: string[]`
    - each entry may be absolute or relative
  - **Outputs**:
    - one effective normalized directory list, or `AgentWrapperError::InvalidRequest`

- **Normalization contract**:
  - **Inputs**:
    - raw `dirs` entries
    - effective working directory (per `docs/specs/universal-agent-api/contract.md` "Working directory resolution (effective working directory)")
  - **Outputs**:
    - trimmed, resolved, lexically normalized, deduplicated directory list

- **Runtime rejection contract**:
  - **Inputs**:
    - request passed R0 gating and pre-spawn validation
  - **Outputs**:
    - backend either honors the accepted directory set or fails with
      `AgentWrapperError::Backend` and a safe/redacted message

## Key invariants / rules

- `dirs` is required and the schema is closed in v1.
- Relative paths resolve against the effective working directory (per
  `docs/specs/universal-agent-api/contract.md` "Working directory resolution (effective working directory)").
- There is intentionally no “must stay under working_dir” containment rule.
- v1 requires lexical normalization only.
- Invalid messages for this key must not echo raw path values.
- Resume and fork flows must not silently drop accepted add-dir inputs.

## Dependencies

- Blocks: SEAM-2/3/4/5
- Blocked by: none

## Touch surface

- `docs/adr/0021-universal-agent-api-add-dirs.md`
- `docs/specs/universal-agent-api/extensions-spec.md`

## Verification

- Spec review: confirm the ADR and `extensions-spec.md` say the same thing.
- Drift guard:
  - `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md`
- Integration gate once implementation lands:
  - `make preflight`

## Risks / unknowns

- **Risk**: downstream seams re-interpret “effective working directory” differently.
- **De-risk plan**: force SEAM-2 to consume the already-selected backend working directory rather
  than inventing a second precedence chain for add-dir resolution.

## Rollout / safety

- Additive only.
- When absent, the backend emits no add-dir argv at all.
