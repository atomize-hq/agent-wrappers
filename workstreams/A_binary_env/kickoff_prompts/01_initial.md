You are starting Workstream A (Binary + Env Isolation), Task A1-design-env-api.
Goal: design how the crate accepts a pinned Codex binary path and an app-scoped CODEX_HOME per invocation, and how all Command spawns use a shared env-prep helper.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs.
Deliverable: an API proposal (doc comments or short design note in-repo) that keeps backward compatibility and outlines directory expectations under CODEX_HOME.
