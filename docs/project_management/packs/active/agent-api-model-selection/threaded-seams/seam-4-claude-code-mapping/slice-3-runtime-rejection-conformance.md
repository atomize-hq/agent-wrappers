### S3 — Runtime rejection conformance and contract publication

- This slice was decomposed because it bundled two concern groups across
  `crates/agent_api` runtime classification and fake-Claude scenario plumbing plus the final
  Claude conformance publication work spanning the canonical spec and focused
  `agent_api`/`claude_code` regression surfaces.
- Archived original: `archive/slice-3-runtime-rejection-conformance.md`
- Sub-slice directory: `slice-3-runtime-rejection-conformance/`

#### Sub-slices

- `subslice-1-runtime-rejection-translation.md` — `S3a`, moving original `S3.T1` into one session
  focused on narrow runtime-rejection detection, safe backend translation, and the dedicated
  fake-Claude parity path after `system init`.
- `subslice-2-contract-publication-and-focused-tests.md` — `S3b`, moving original `S3.T2` into
  one session focused on the canonical Claude mapping doc and the smallest focused backend/argv
  regressions SEAM-5B should inherit instead of rediscovering.

#### Task redistribution

- `S3.T1` moved to `S3a`.
- `S3.T2` moved to `S3b`.
