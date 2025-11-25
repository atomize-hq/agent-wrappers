//! Probe the Codex binary for version/features and gate optional flags.
//!
//! This example runs `codex --version` and `codex features list` (if available) and then
//! demonstrates gating streaming/logging/artifact flags plus MCP/app-server flows. If the binary
//! is missing, it falls back to sample capability data. Set `CODEX_BINARY` to override the binary
//! path.
//!
//! Example:
//! ```bash
//! cargo run -p codex --example feature_detection
//! CODEX_BINARY=/opt/codex-nightly cargo run -p codex --example feature_detection
//! ```

use std::{
    collections::HashMap,
    env,
    error::Error,
    path::Path,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use tokio::process::Command;

#[derive(Debug, Clone)]
struct Capability {
    version: Option<Version>,
    features: Vec<String>,
}

#[derive(Debug, Clone)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

static CAPABILITY_CACHE: OnceLock<Mutex<HashMap<PathBuf, Capability>>> = OnceLock::new();

impl Version {
    fn parse(raw: &str) -> Option<Self> {
        let tokens: Vec<&str> = raw.split(|c: char| c.is_whitespace() || c == '-').collect();
        let version_str = tokens.iter().find(|token| {
            token
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })?;
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        let major = parts.get(0)?.parse().ok()?;
        let minor = parts.get(1)?.parse().ok()?;
        let patch = parts.get(2).unwrap_or(&"0").parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let binary = resolve_binary();
    let (capability, cached) = if binary_exists(&binary) {
        cached_probe(&binary).await
    } else {
        eprintln!(
            "Binary not found at {}. Using sample capability set.",
            binary.display()
        );
        (sample_capability(), false)
    };

    if let Some(version) = capability.version.as_ref() {
        println!("Detected Codex version: {}", version.as_string());
    } else {
        println!("Version unknown (could not parse output)");
    }
    if cached {
        println!("Capabilities served from cache for {}", binary.display());
    }
    println!("Features: {}", capability.features.join(", "));
    println!(
        "Cache scope: per binary path for this process; refresh probes after upgrading the binary."
    );

    if capability.supports("json-stream") {
        println!("-> Enable streaming examples (stream_events, stream_with_log).");
    } else {
        println!("-> Streaming disabled: feature not reported by the binary.");
    }

    if capability.supports("log-tee") {
        println!("-> Log tee supported; safe to write to log files.");
    } else {
        println!("-> Log tee unavailable; fall back to console-only streaming.");
    }

    if capability.supports("resume") {
        println!(
            "-> Resume supported; enable resume_apply example and prompt for conversation IDs."
        );
    } else {
        println!("-> Resume unsupported; hide resume_apply in your UI.");
    }

    if capability.supports("diff") && capability.supports("apply") {
        println!("-> Diff/apply supported; capture stdout/stderr/exit when applying patches.");
    } else {
        println!("-> Skip codex diff/apply helpers when the binary does not advertise them.");
    }

    if capability.supports("output-last-message") && capability.supports("output-schema") {
        println!("-> Artifact flags supported; enable --output-last-message/--output-schema.");
    } else {
        println!("-> Skip artifact flags when streaming; binary does not advertise them.");
    }

    if capability.supports("mcp-server") && capability.supports("app-server") {
        println!("-> MCP + app-server endpoints available; enable the related examples.");
    } else {
        println!("-> Server endpoints missing; keep MCP/app-server flows disabled.");
    }

    if let Some(update_hook) = update_advisory_hook(&capability) {
        println!("{update_hook}");
    }

    Ok(())
}

impl Capability {
    fn supports(&self, name: &str) -> bool {
        self.features
            .iter()
            .any(|feature| normalize(feature) == name.to_ascii_lowercase())
    }
}

async fn cached_probe(binary: &Path) -> (Capability, bool) {
    let cache = CAPABILITY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(existing) = cache.lock().unwrap().get(binary) {
        return (existing.clone(), true);
    }

    let capability = probe_capabilities(binary).await;
    cache
        .lock()
        .unwrap()
        .insert(binary.to_path_buf(), capability.clone());
    (capability, false)
}

async fn probe_capabilities(binary: &Path) -> Capability {
    let version = run_version(binary)
        .await
        .and_then(|raw| Version::parse(&raw));
    let features = run_features(binary)
        .await
        .unwrap_or_else(|| vec!["json-stream".into(), "output-last-message".into()]);
    Capability { version, features }
}

fn sample_capability() -> Capability {
    Capability {
        version: Some(Version {
            major: 1,
            minor: 4,
            patch: 0,
        }),
        features: vec![
            "json-stream".into(),
            "output-last-message".into(),
            "output-schema".into(),
            "log-tee".into(),
            "diff".into(),
            "apply".into(),
            "resume".into(),
            "app-server".into(),
            "mcp-server".into(),
            "notify".into(),
        ],
    }
}

async fn run_version(binary: &Path) -> Option<String> {
    Command::new(binary)
        .arg("--version")
        .output()
        .await
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
}

async fn run_features(binary: &Path) -> Option<Vec<String>> {
    let output = Command::new(binary)
        .args(["features", "list"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut features = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        features.push(trimmed.to_string());
    }
    Some(features)
}

fn update_advisory_hook(capability: &Capability) -> Option<String> {
    let missing: Vec<&str> = ["json-stream", "log-tee", "diff", "apply"]
        .iter()
        .copied()
        .filter(|name| !capability.supports(name))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let binary_desc = capability
        .version
        .as_ref()
        .map(|v| v.as_string())
        .unwrap_or_else(|| "<unknown>".into());
    Some(format!(
        "Update advisory: binary {binary_desc} is missing {missing}; prompt the user to download the latest release.",
        missing = missing.join(", ")
    ))
}

fn normalize(feature: &str) -> String {
    feature
        .split(|c: char| c.is_whitespace() || c == ':' || c == '=')
        .next()
        .unwrap_or(feature)
        .to_ascii_lowercase()
}

fn resolve_binary() -> PathBuf {
    env::var_os("CODEX_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"))
}

fn binary_exists(path: &Path) -> bool {
    if path.is_absolute() || path.components().count() > 1 {
        std::fs::metadata(path).is_ok()
    } else {
        env::var_os("PATH")
            .and_then(|paths| {
                env::split_paths(&paths)
                    .map(|dir| dir.join(path))
                    .find(|candidate| std::fs::metadata(candidate).is_ok())
            })
            .is_some()
    }
}
