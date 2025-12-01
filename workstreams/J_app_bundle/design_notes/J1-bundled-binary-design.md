# Workstream J1: Bundled Binary & Home Isolation Design

Objective: define an opt-in, app-bundled Codex binary flow that never touches the user's global install, keeps credentials/history per app/project, and exposes a helper without changing current defaults.

## Goals and Constraints
- Isolation first: when an app opts in, it must not depend on `PATH` or a user-level `CODEX_HOME`; every spawn should point at an app-owned binary and `CODEX_HOME`.
- Default behavior stays as-is (`CODEX_BINARY` > `codex` on `PATH`) unless the new helper is invoked.
- Host apps own binary download/pinning/upgrades. The wrapper only resolves paths and probes capabilities; it never auto-updates.
- Clear failure: if the pinned binary or home is missing/unreadable, return a typed error instead of falling back to any global state.

## Bundled Binary Contract
- **Bundle root**: app-chosen directory such as `~/.myapp/codex-bin`. Must be writable by the app and not shared with other products.
- **Platform slice**: subdirectory named after the runtime target (`darwin-arm64`, `darwin-x64`, `linux-x64`, `windows-x64`). This keeps multi-platform bundles side-by-side and avoids collisions.
- **Version slice**: subdirectory matching the pinned Codex build (semantic version or channel+build id). Example tree:
  ```
  ~/.myapp/codex-bin/
    darwin-arm64/
      1.2.3/
        codex         # executable; codex.exe on Windows
        VERSION       # optional text file with the pinned version string
        checksums.txt # optional (host-managed)
  ```
- **Selection rules**: host apps pass `(bundle_root, version)` to the helper. Platform defaults to the running target triple string; callers may override when they embed a different layout. The helper never consults `CODEX_BINARY` or `PATH`.
- **Verification**: helper canonicalizes the binary path, ensures it exists and is executable, and can optionally compare the on-disk `VERSION` (or `codex --version` output) to the requested version string for diagnostics. Capability caches key off the canonical path so version swaps invalidate cleanly.
- **Updates**: hosts download new builds into a new `<platform>/<version>` directory, then update their config to point at that version. Old versions can be GC'd by the host; the wrapper will not auto-switch or download. Failure to find the pinned binary is a hard error surfaced to the caller.

## `CODEX_HOME` Selection and Auth Placement
- **Per-project homes**: apps should derive a deterministic home per project/workspace (e.g., `~/.myapp/codex-homes/<project-slug>/`). Prefer a slug+hash of the project root to avoid collisions. Do not reuse the user's global Codex home.
- **Layout**: each home uses the standard `CODEX_HOME` tree: `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/`, `logs/`. `CodexHomeLayout::materialize` can pre-create roots/log/conversation dirs when requested.
- **Auth seeding**: if an app wants to reuse an existing login, copy only `auth.json` and `.credentials.json` from a trusted seed home (e.g., a prior app-scoped home) into the new project home before spawning Codex. Do not copy history/logs between projects. If no seed exists, run the login flow under the project home via `AuthSessionHelper`.
- **Multiple homes**: maintain separate homes per project/workspace; optionally keep a shared "seed" auth home (app-owned) to reduce login prompts. Swapping homes should flush any cached capability data keyed by `CODEX_HOME` if it ever grows to include per-home state, but today capability caches are keyed solely by binary path.

## Wrapper Helper Surface
- Add a new opt-in helper (name to finalize during implementation) exposed by the crate, e.g.:
  ```rust
  pub struct BundledBinarySpec<'a> {
      pub bundle_root: &'a Path,
      pub version: &'a str,
      pub platform: Option<&'a str>, // defaults to runtime platform label
  }

  pub struct BundledBinary {
      pub binary_path: PathBuf,
      pub version: String,
      pub platform: String,
  }

  pub fn resolve_bundled_binary(spec: BundledBinarySpec) -> Result<BundledBinary, BundledBinaryError>;
  ```
  Behavior: build `<bundle_root>/<platform>/<version>/<codex|codex.exe>`, canonicalize it, assert readability/executability, and return a typed error on missing/mismatched files. No fallback to `CODEX_BINARY`/`PATH`. The returned path feeds `CodexClientBuilder::binary(...)`.
- Helper must not alter global defaults: existing builders still honor `CODEX_BINARY` and `codex` on `PATH` until the helper's return value is passed explicitly.
- Suggested usage pattern:
  1. Host selects bundle root/version from its config, calls the helper, and logs a clear error if missing.
  2. Host derives a project-specific `CODEX_HOME` root and sets it on the builder (`.codex_home(...)` with `create_home_dirs(true)`).
  3. Host uses `AuthSessionHelper` under that home to ensure login, optionally seeding auth files from a shared app-owned home.

## Compatibility Notes
- The helper is purely additive. Existing env-based binary selection, capability probing, and auth helpers remain unchanged unless the host opts into the bundled path resolver.
- All environment injection stays per-subprocess (`CommandEnvironment` sets `CODEX_BINARY`/`CODEX_HOME` on each spawn) so parent process state is untouched even in the bundled flow.
