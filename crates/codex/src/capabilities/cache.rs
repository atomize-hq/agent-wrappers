use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs as std_fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use super::{
    CapabilityFeatureOverrides, CapabilityOverrides, CapabilityProbeStep, CodexCapabilities,
    CodexFeatureFlags,
};

/// Cache interaction policy for capability probes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CapabilityCachePolicy {
    /// Use cached entries when fingerprints match; fall back to probing when
    /// fingerprints differ or are missing and write fresh snapshots back.
    #[default]
    PreferCache,
    /// Always run probes, overwriting any existing cache entry for the binary (useful for TTL/backoff windows or hot-swaps that keep the same path).
    Refresh,
    /// Skip cache reads and writes to force an isolated snapshot.
    Bypass,
}

/// Cache key for capability snapshots derived from a specific Codex binary path.
///
/// Cache lookups should canonicalize the path when possible so symlinked binaries
/// collapse to a single entry.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CapabilityCacheKey {
    /// Canonical binary path when resolvable; otherwise the original path.
    pub binary_path: PathBuf,
}

/// File metadata used to invalidate cached capability snapshots when the binary changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BinaryFingerprint {
    /// Canonical path if the binary resolves through symlinks.
    pub canonical_path: Option<PathBuf>,
    /// Last modification time of the binary on disk (`metadata().modified()`).
    pub modified: Option<SystemTime>,
    /// File length from `metadata().len()`, useful for cheap change detection.
    pub len: Option<u64>,
}

pub(crate) fn capability_cache() -> &'static Mutex<HashMap<CapabilityCacheKey, CodexCapabilities>> {
    static CACHE: OnceLock<Mutex<HashMap<CapabilityCacheKey, CodexCapabilities>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn capability_cache_key(binary: &Path) -> CapabilityCacheKey {
    let canonical = std_fs::canonicalize(binary).unwrap_or_else(|_| binary.to_path_buf());
    CapabilityCacheKey {
        binary_path: canonical,
    }
}

pub(crate) fn has_fingerprint_metadata(fingerprint: &Option<BinaryFingerprint>) -> bool {
    fingerprint.is_some()
}

pub(crate) fn cached_capabilities(
    key: &CapabilityCacheKey,
    fingerprint: &Option<BinaryFingerprint>,
) -> Option<CodexCapabilities> {
    let cache = capability_cache().lock().ok()?;
    let cached = cache.get(key)?;
    if !has_fingerprint_metadata(&cached.fingerprint) || !has_fingerprint_metadata(fingerprint) {
        return None;
    }
    if fingerprints_match(&cached.fingerprint, fingerprint) {
        Some(cached.clone())
    } else {
        None
    }
}

pub(crate) fn update_capability_cache(capabilities: CodexCapabilities) {
    if !has_fingerprint_metadata(&capabilities.fingerprint) {
        return;
    }
    if let Ok(mut cache) = capability_cache().lock() {
        cache.insert(capabilities.cache_key.clone(), capabilities);
    }
}

/// Returns all capability cache entries keyed by canonical binary path.
pub fn capability_cache_entries() -> Vec<CodexCapabilities> {
    capability_cache()
        .lock()
        .map(|cache| cache.values().cloned().collect())
        .unwrap_or_default()
}

/// Returns the cached capabilities for a specific binary path if present.
pub fn capability_cache_entry(binary: &Path) -> Option<CodexCapabilities> {
    let key = capability_cache_key(binary);
    capability_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
}

/// Removes the cached capabilities for a specific binary. Returns true when an entry was removed.
pub fn clear_capability_cache_entry(binary: &Path) -> bool {
    let key = capability_cache_key(binary);
    capability_cache()
        .lock()
        .ok()
        .map(|mut cache| cache.remove(&key).is_some())
        .unwrap_or(false)
}

/// Clears all cached capability snapshots.
pub fn clear_capability_cache() {
    if let Ok(mut cache) = capability_cache().lock() {
        cache.clear();
    }
}

pub(crate) fn current_fingerprint(key: &CapabilityCacheKey) -> Option<BinaryFingerprint> {
    let canonical = std_fs::canonicalize(&key.binary_path).ok();
    let metadata_path = canonical.as_deref().unwrap_or(key.binary_path.as_path());
    let metadata = std_fs::metadata(metadata_path).ok()?;
    Some(BinaryFingerprint {
        canonical_path: canonical,
        modified: metadata.modified().ok(),
        len: Some(metadata.len()),
    })
}

pub(crate) fn fingerprints_match(
    cached: &Option<BinaryFingerprint>,
    fresh: &Option<BinaryFingerprint>,
) -> bool {
    cached == fresh
}

pub(crate) fn finalize_capabilities_with_overrides(
    mut capabilities: CodexCapabilities,
    overrides: &CapabilityOverrides,
    cache_key: CapabilityCacheKey,
    fingerprint: Option<BinaryFingerprint>,
    manual_source: bool,
) -> CodexCapabilities {
    capabilities.cache_key = cache_key;
    capabilities.fingerprint = fingerprint;

    let mut applied = manual_source;

    if let Some(version) = overrides.version.clone() {
        capabilities.version = Some(version);
        applied = true;
    }

    if apply_feature_overrides(&mut capabilities.features, &overrides.features) {
        applied = true;
    }

    if applied
        && !capabilities
            .probe_plan
            .steps
            .contains(&CapabilityProbeStep::ManualOverride)
    {
        capabilities
            .probe_plan
            .steps
            .push(CapabilityProbeStep::ManualOverride);
    }

    capabilities
}

fn apply_feature_overrides(
    features: &mut CodexFeatureFlags,
    overrides: &CapabilityFeatureOverrides,
) -> bool {
    let mut applied = false;

    if let Some(value) = overrides.supports_features_list {
        features.supports_features_list = value;
        applied = true;
    }

    if let Some(value) = overrides.supports_output_schema {
        features.supports_output_schema = value;
        applied = true;
    }

    if let Some(value) = overrides.supports_add_dir {
        features.supports_add_dir = value;
        applied = true;
    }

    if let Some(value) = overrides.supports_mcp_login {
        features.supports_mcp_login = value;
        applied = true;
    }

    applied
}
