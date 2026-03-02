# Seam map — Explicit cancellation (`agent_api`)

## Seams

1) **SEAM-1 — Cancellation contract (public API + semantics)**
   - Owns: the cancellation API surface and its exact behavior.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
     - updates to `docs/specs/universal-agent-api/contract.md`
     - updates to `docs/specs/universal-agent-api/run-protocol-spec.md`

2) **SEAM-2 — Harness cancellation propagation**
   - Owns: how cancellation is wired into the backend harness driver tasks without breaking
     drain-on-drop.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md`

3) **SEAM-3 — Backend termination responsibilities**
   - Owns: what each built-in backend must provide to enable best-effort termination (Codex + Claude
     Code), including any wrapper-level hooks required.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`

4) **SEAM-4 — Tests**
   - Owns: the tests that pin explicit cancellation behavior and prevent regression.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md`

