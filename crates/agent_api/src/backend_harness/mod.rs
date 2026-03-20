//! Backend harness wiring for wrapper backends.
//!
//! This module is crate-private and is split into orthogonal submodules:
//! - `contract`: shared types + adapter contract
//! - `normalize`: request normalization + extension parsing helpers
//! - `runtime`: event pumping + run orchestration

/// BH-C04 bounded channel default; pinned to preserve existing backend behavior.
pub(crate) const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 32;

mod contract;
mod normalize;
mod runtime;

#[cfg(test)]
mod test_support;

#[allow(unused_imports)]
pub(crate) use contract::DynBackendCompletionFuture;
pub(crate) use contract::{
    BackendDefaults, BackendHarnessAdapter, BackendHarnessErrorPhase, BackendSpawn,
    DynBackendEventStream, NormalizedRequest,
};
#[allow(unused_imports)]
pub(crate) use normalize::{normalize_add_dirs_v1, normalize_request};
pub(crate) use runtime::run_harnessed_backend;
#[allow(unused_imports)]
pub(crate) use runtime::run_harnessed_backend_control;
