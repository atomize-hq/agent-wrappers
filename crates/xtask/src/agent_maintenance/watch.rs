use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use clap::{ArgGroup, Parser};
use semver::Version;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    agent_lifecycle::maintenance_request_path,
    agent_registry::{
        AgentRegistry, AgentRegistryEntry, AgentRegistryError, ReleaseWatchMetadata,
        ReleaseWatchSourceKind, ReleaseWatchVersionPolicy,
    },
};

use super::contract_policy::{
    dispatch_kind_str, dispatch_workflow_value, opened_from_path, version_policy_str,
    GENERATED_BY_WORKFLOW,
};

const QUEUE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Parser, Clone)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["check", "emit_json"])
        .multiple(true)
))]
pub struct Args {
    #[arg(long)]
    pub check: bool,

    #[arg(long)]
    pub emit_json: Option<PathBuf>,

    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Validation(String),
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message) | Self::Internal(message) => f.write_str(message),
        }
    }
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Validation(_) => 2,
            Self::Internal(_) => 1,
        }
    }
}

impl From<AgentRegistryError> for Error {
    fn from(value: AgentRegistryError) -> Self {
        match value {
            AgentRegistryError::Validation(message) => Self::Validation(message),
            AgentRegistryError::Read { path, source } => {
                Self::Internal(format!("read agent registry `{path}`: {source}"))
            }
            AgentRegistryError::Toml(source) => {
                Self::Internal(format!("parse agent registry: {source}"))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceWatchQueue {
    pub schema_version: u32,
    pub generated_at: String,
    pub stale_agents: Vec<MaintenanceWatchQueueEntry>,
    #[serde(default)]
    pub failed_agents: Vec<MaintenanceWatchQueueFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceWatchQueueEntry {
    pub agent_id: String,
    pub manifest_root: String,
    pub current_validated: String,
    pub latest_stable: String,
    pub target_version: String,
    pub version_policy: String,
    pub dispatch_kind: String,
    pub dispatch_workflow: String,
    pub maintenance_root: String,
    pub request_path: String,
    pub opened_from: String,
    pub detected_by: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceWatchQueueFailure {
    pub agent_id: String,
    pub error: String,
}

pub fn run(args: Args) -> Result<(), Error> {
    let workspace_root = repo_root();
    let mut stdout = io::stdout();
    run_in_workspace(&workspace_root, args, &mut stdout)
}

pub fn run_in_workspace<W: Write>(
    workspace_root: &Path,
    args: Args,
    writer: &mut W,
) -> Result<(), Error> {
    run_in_workspace_with_resolver(workspace_root, args, writer, resolve_release_history)
}

pub fn run_in_workspace_with_resolver<W, F>(
    workspace_root: &Path,
    args: Args,
    writer: &mut W,
    mut resolve_versions: F,
) -> Result<(), Error>
where
    W: Write,
    F: FnMut(&AgentRegistryEntry, &ReleaseWatchMetadata) -> Result<Vec<Version>, Error>,
{
    let queue = build_watch_queue_with_resolver_and_filter(
        workspace_root,
        args.agent.as_deref(),
        |entry, release_watch| resolve_versions(entry, release_watch),
    )?;
    writeln!(writer, "schema_version: {}", queue.schema_version)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "generated_at: {}", queue.generated_at)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "stale_agents: {}", queue.stale_agents.len())
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "failed_agents: {}", queue.failed_agents.len())
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    for entry in &queue.stale_agents {
        writeln!(
            writer,
            "{} -> {} (current {} latest {} via {})",
            entry.agent_id,
            entry.target_version,
            entry.current_validated,
            entry.latest_stable,
            entry.dispatch_workflow
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }
    for failure in &queue.failed_agents {
        writeln!(writer, "{} failed: {}", failure.agent_id, failure.error)
            .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }

    if let Some(path) = args.emit_json.as_ref() {
        write_queue_json(workspace_root, path, &queue)?;
        writeln!(writer, "emitted_json: {}", path.display())
            .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }

    if args.check && !queue.stale_agents.is_empty() {
        return Err(Error::Validation(format!(
            "maintenance-watch found {} stale enrolled agent(s); rerun with --emit-json to inspect the queue",
            queue.stale_agents.len()
        )));
    }

    Ok(())
}

pub fn build_watch_queue(workspace_root: &Path) -> Result<MaintenanceWatchQueue, Error> {
    build_watch_queue_with_resolver(workspace_root, resolve_release_history)
}

pub fn build_watch_queue_with_resolver<F>(
    workspace_root: &Path,
    mut resolve_versions: F,
) -> Result<MaintenanceWatchQueue, Error>
where
    F: FnMut(&AgentRegistryEntry, &ReleaseWatchMetadata) -> Result<Vec<Version>, Error>,
{
    build_watch_queue_with_resolver_and_filter(workspace_root, None, |entry, release_watch| {
        resolve_versions(entry, release_watch)
    })
}

fn build_watch_queue_with_resolver_and_filter<F>(
    workspace_root: &Path,
    agent_filter: Option<&str>,
    mut resolve_versions: F,
) -> Result<MaintenanceWatchQueue, Error>
where
    F: FnMut(&AgentRegistryEntry, &ReleaseWatchMetadata) -> Result<Vec<Version>, Error>,
{
    let registry = AgentRegistry::load(workspace_root)?;
    let mut stale_agents = Vec::new();
    let mut failed_agents = Vec::new();

    let entries: Vec<&AgentRegistryEntry> = if let Some(agent_id) = agent_filter {
        vec![registry.find(agent_id).ok_or_else(|| {
            Error::Validation(format!(
                "maintenance-watch references unknown agent_id `{agent_id}`"
            ))
        })?]
    } else {
        registry.agents.iter().collect()
    };

    for entry in entries {
        let Some(release_watch) = entry.maintenance.release_watch.as_ref() else {
            continue;
        };
        if !release_watch.enabled {
            continue;
        }

        match evaluate_agent_watch_entry(
            workspace_root,
            entry,
            release_watch,
            &mut resolve_versions,
        ) {
            Ok(Some(queue_entry)) => stale_agents.push(queue_entry),
            Ok(None) => {}
            Err(error) => failed_agents.push(MaintenanceWatchQueueFailure {
                agent_id: entry.agent_id.clone(),
                error: error.to_string(),
            }),
        }
    }

    Ok(MaintenanceWatchQueue {
        schema_version: QUEUE_SCHEMA_VERSION,
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|err| Error::Internal(format!("format queue timestamp: {err}")))?,
        stale_agents,
        failed_agents,
    })
}

fn evaluate_agent_watch_entry<F>(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
    resolve_versions: &mut F,
) -> Result<Option<MaintenanceWatchQueueEntry>, Error>
where
    F: FnMut(&AgentRegistryEntry, &ReleaseWatchMetadata) -> Result<Vec<Version>, Error>,
{
    let current_validated = read_current_validated(workspace_root, entry)?;
    let mut versions = resolve_versions(entry, release_watch)?;
    versions.sort();
    versions.dedup();
    if versions.is_empty() {
        return Err(Error::Validation(format!(
            "maintenance-watch found no stable upstream versions for agent `{}`",
            entry.agent_id
        )));
    }

    let latest_stable = versions
        .last()
        .cloned()
        .ok_or_else(|| Error::Internal("latest stable missing after sort".to_string()))?;
    let Some(target_version) = select_target_version(&versions, release_watch.version_policy)
    else {
        return Ok(None);
    };
    if target_version <= current_validated {
        return Ok(None);
    }

    let dispatch_workflow =
        dispatch_workflow_value(&entry.agent_id, release_watch).map_err(Error::Validation)?;
    let maintenance_root = format!("docs/agents/lifecycle/{}-maintenance", entry.agent_id);
    Ok(Some(MaintenanceWatchQueueEntry {
        agent_id: entry.agent_id.clone(),
        manifest_root: entry.manifest_root.clone(),
        current_validated: current_validated.to_string(),
        latest_stable: latest_stable.to_string(),
        target_version: target_version.to_string(),
        version_policy: version_policy_str(release_watch.version_policy).to_string(),
        dispatch_kind: dispatch_kind_str(release_watch.dispatch_kind).to_string(),
        dispatch_workflow: dispatch_workflow.clone(),
        maintenance_root: maintenance_root.clone(),
        request_path: maintenance_request_path(&entry.agent_id),
        opened_from: opened_from_path(&dispatch_workflow),
        detected_by: GENERATED_BY_WORKFLOW.to_string(),
        branch_name: format!(
            "automation/{}-maintenance-{}",
            entry.agent_id, target_version
        ),
    }))
}

fn read_current_validated(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
) -> Result<Version, Error> {
    let path = workspace_root
        .join(&entry.manifest_root)
        .join("latest_validated.txt");
    let raw = fs::read_to_string(&path)
        .map_err(|err| Error::Validation(format!("read {}: {err}", path.display())))?;
    parse_semver(
        raw.trim(),
        &format!(
            "latest_validated.txt for agent `{}` at {}",
            entry.agent_id,
            path.display()
        ),
    )
}

pub(crate) fn select_target_version(
    versions: &[Version],
    version_policy: ReleaseWatchVersionPolicy,
) -> Option<Version> {
    match version_policy {
        ReleaseWatchVersionPolicy::LatestStableMinusOne => {
            if versions.len() < 2 {
                None
            } else {
                versions.get(versions.len() - 2).cloned()
            }
        }
        ReleaseWatchVersionPolicy::UpstreamStablePointer => versions.last().cloned(),
    }
}

fn resolve_release_history(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
) -> Result<Vec<Version>, Error> {
    match release_watch.upstream.source_kind {
        ReleaseWatchSourceKind::GithubReleases => fetch_github_releases(entry, release_watch),
        ReleaseWatchSourceKind::GcsObjectListing => fetch_gcs_versions(entry, release_watch),
        ReleaseWatchSourceKind::NpmDistTag => fetch_npm_dist_tag_version(entry, release_watch),
    }
}

fn fetch_github_releases(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
) -> Result<Vec<Version>, Error> {
    fetch_github_releases_with_fetcher(entry, release_watch, fetch_text)
}

pub(crate) fn fetch_github_releases_with_fetcher<F>(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
    mut fetch: F,
) -> Result<Vec<Version>, Error>
where
    F: FnMut(&str) -> Result<String, Error>,
{
    let owner = release_watch.upstream.owner.as_deref().ok_or_else(|| {
        Error::Validation(format!(
            "release_watch owner missing for github_releases agent `{}`",
            entry.agent_id
        ))
    })?;
    let repo = release_watch.upstream.repo.as_deref().ok_or_else(|| {
        Error::Validation(format!(
            "release_watch repo missing for github_releases agent `{}`",
            entry.agent_id
        ))
    })?;
    let tag_prefix = release_watch
        .upstream
        .tag_prefix
        .as_deref()
        .ok_or_else(|| {
            Error::Validation(format!(
                "release_watch tag_prefix missing for github_releases agent `{}`",
                entry.agent_id
            ))
        })?;
    let mut versions = Vec::new();
    let mut page = 1usize;
    loop {
        let url = format!(
            "https://api.github.com/repos/{owner}/{repo}/releases?per_page=100&page={page}"
        );
        let body = fetch(&url)?;
        let releases: Vec<GithubRelease> = serde_json::from_str(&body).map_err(|err| {
            Error::Validation(format!(
                "parse GitHub releases for agent `{}` from {url}: {err}",
                entry.agent_id
            ))
        })?;
        let release_count = releases.len();
        for release in releases {
            if release.draft || release.prerelease {
                continue;
            }
            let Some(tag_name) = release.tag_name else {
                continue;
            };
            let Some(raw_version) = tag_name.strip_prefix(tag_prefix) else {
                continue;
            };
            versions.push(parse_semver(
                raw_version,
                &format!("GitHub tag `{tag_name}` for agent `{}`", entry.agent_id),
            )?);
        }
        if release_count < 100 {
            break;
        }
        page += 1;
    }
    Ok(versions)
}

fn fetch_gcs_versions(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
) -> Result<Vec<Version>, Error> {
    fetch_gcs_versions_with_fetcher(entry, release_watch, fetch_text)
}

fn fetch_npm_dist_tag_version(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
) -> Result<Vec<Version>, Error> {
    fetch_npm_dist_tag_version_with_fetcher(entry, release_watch, fetch_text)
}

pub(crate) fn fetch_npm_dist_tag_version_with_fetcher<F>(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
    mut fetch: F,
) -> Result<Vec<Version>, Error>
where
    F: FnMut(&str) -> Result<String, Error>,
{
    let package = release_watch.upstream.package.as_deref().ok_or_else(|| {
        Error::Validation(format!(
            "release_watch package missing for npm_dist_tag agent `{}`",
            entry.agent_id
        ))
    })?;
    let dist_tag = release_watch.upstream.dist_tag.as_deref().ok_or_else(|| {
        Error::Validation(format!(
            "release_watch dist_tag missing for npm_dist_tag agent `{}`",
            entry.agent_id
        ))
    })?;
    // npm scoped packages contain `/` (e.g. `@scope/name`). Encode the package as a
    // single path segment so the scope separator is never transmitted as a path
    // separator by a stricter registry, CDN, or proxy.
    let url = format!(
        "https://registry.npmjs.org/{}",
        percent_encode_query_value(package)
    );
    let body = fetch(&url)?;
    let metadata: NpmPackageMetadata = serde_json::from_str(&body).map_err(|err| {
        Error::Validation(format!(
            "parse npm metadata for agent `{}` from {url}: {err}",
            entry.agent_id
        ))
    })?;
    let raw_version = metadata.dist_tags.get(dist_tag).ok_or_else(|| {
        Error::Validation(format!(
            "npm dist-tag `{dist_tag}` missing for package `{package}` on agent `{}`",
            entry.agent_id
        ))
    })?;

    Ok(vec![parse_semver(
        raw_version,
        &format!(
            "npm dist-tag `{dist_tag}` for package `{package}` on agent `{}`",
            entry.agent_id
        ),
    )?])
}

pub(crate) fn fetch_gcs_versions_with_fetcher<F>(
    entry: &AgentRegistryEntry,
    release_watch: &ReleaseWatchMetadata,
    mut fetch: F,
) -> Result<Vec<Version>, Error>
where
    F: FnMut(&str) -> Result<String, Error>,
{
    let bucket = release_watch.upstream.bucket.as_deref().ok_or_else(|| {
        Error::Validation(format!(
            "release_watch bucket missing for gcs_object_listing agent `{}`",
            entry.agent_id
        ))
    })?;
    let prefix = release_watch.upstream.prefix.as_deref().ok_or_else(|| {
        Error::Validation(format!(
            "release_watch prefix missing for gcs_object_listing agent `{}`",
            entry.agent_id
        ))
    })?;
    let version_marker = release_watch
        .upstream
        .version_marker
        .as_deref()
        .ok_or_else(|| {
            Error::Validation(format!(
                "release_watch version_marker missing for gcs_object_listing agent `{}`",
                entry.agent_id
            ))
        })?;

    let normalized_prefix = if prefix.ends_with('/') {
        prefix.to_string()
    } else {
        format!("{prefix}/")
    };
    let mut page_token: Option<String> = None;
    let mut versions = Vec::new();
    loop {
        let mut url = format!(
            "https://storage.googleapis.com/storage/v1/b/{bucket}/o?prefix={normalized_prefix}"
        );
        if let Some(token) = page_token.as_deref() {
            url.push_str("&pageToken=");
            url.push_str(&percent_encode_query_value(token));
        }
        let body = fetch(&url)?;
        let listing: GcsListingResponse = serde_json::from_str(&body).map_err(|err| {
            Error::Validation(format!(
                "parse GCS listing for agent `{}` from {url}: {err}",
                entry.agent_id
            ))
        })?;
        for item in listing.items {
            let Some(remainder) = item.name.strip_prefix(&normalized_prefix) else {
                continue;
            };
            let Some((candidate, rest)) = remainder.split_once('/') else {
                continue;
            };
            if rest != version_marker {
                continue;
            }
            versions.push(parse_semver(
                candidate,
                &format!("GCS object `{}` for agent `{}`", item.name, entry.agent_id),
            )?);
        }
        let Some(next_page_token) = listing.next_page_token else {
            break;
        };
        page_token = Some(next_page_token);
    }
    Ok(versions)
}

fn percent_encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(&mut encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

/// Build the curl argument vector for an upstream metadata fetch.
///
/// The body is written to `response_path` instead of stdout. curl truncates the
/// output file at the start of each retry attempt, whereas a retried transfer
/// captured from stdout appends the retried body to the partial one. That splices
/// two JSON documents together and corrupts large responses — the
/// `openai/codex` releases page is ~26 MB, so an interrupted transfer there
/// yielded an unparseable body rather than a clean failure.
///
/// Arguments are `OsString`, not `String`: `Path::display` replaces non-Unicode
/// bytes with U+FFFD, so a `TMPDIR` containing arbitrary bytes would send curl a
/// *different* path than the one read back afterwards — turning a successful
/// download into a spurious upstream failure.
pub(crate) fn build_curl_args(
    url: &str,
    response_path: &Path,
    token: Option<&str>,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        OsString::from("-fsSL"),
        OsString::from("--retry"),
        OsString::from("3"),
        OsString::from("--retry-all-errors"),
        // Multi-megabyte bodies routinely exceed a 20s window, and the retry timer
        // starts before the first attempt: too small a budget means a failed large
        // transfer is never retried at all.
        OsString::from("--retry-max-time"),
        OsString::from("120"),
        OsString::from("-H"),
        OsString::from("User-Agent: unified-agent-api-maintenance-watch/1.0"),
        // Never add `-C`/`--continue-at` or `-a`/`--append` here: those re-enable
        // the resume/append semantics whose absence is what keeps a retried
        // transfer from splicing onto a partial body.
        OsString::from("-o"),
        response_path.as_os_str().to_os_string(),
    ];
    if url.starts_with("https://api.github.com/") {
        if let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) {
            args.push(OsString::from("-H"));
            args.push(OsString::from(format!("Authorization: Bearer {token}")));
        }
    }
    args.push(OsString::from(url));
    args
}

fn temp_response_path() -> PathBuf {
    env::temp_dir().join(format!(
        "uaa-maintenance-watch-{}-{}.body",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ))
}

pub(crate) fn fetch_text(url: &str) -> Result<String, Error> {
    let response_path = temp_response_path();
    let token = env::var("GITHUB_TOKEN").ok();
    let args = build_curl_args(url, &response_path, token.as_deref());

    let outcome = Command::new("curl")
        .args(&args)
        .output()
        .map_err(|err| Error::Internal(format!("spawn curl for {url}: {err}")))
        .and_then(|output| {
            if !output.status.success() {
                return Err(Error::Validation(format!(
                    "curl failed for {url} with exit {}: {}",
                    output.status.code().unwrap_or(1),
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            fs::read_to_string(&response_path)
                .map_err(|err| Error::Internal(format!("read curl response body for {url}: {err}")))
        });

    let _ = fs::remove_file(&response_path);
    outcome
}

fn write_queue_json(
    workspace_root: &Path,
    path: &Path,
    queue: &MaintenanceWatchQueue,
) -> Result<(), Error> {
    let output_path = workspace_root.join(path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| Error::Internal(format!("create {}: {err}", parent.display())))?;
    }
    let mut bytes = serde_json::to_vec_pretty(queue)
        .map_err(|err| Error::Internal(format!("serialize queue json: {err}")))?;
    bytes.push(b'\n');
    fs::write(&output_path, bytes)
        .map_err(|err| Error::Internal(format!("write {}: {err}", output_path.display())))
}

fn parse_semver(raw: &str, context: &str) -> Result<Version, Error> {
    Version::parse(raw).map_err(|err| {
        Error::Validation(format!(
            "{context} must be strict semver MAJOR.MINOR.PATCH (got `{raw}`): {err}"
        ))
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("xtask crate should live under crates/xtask")
        .to_path_buf()
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: Option<String>,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GcsListingResponse {
    #[serde(default)]
    items: Vec<GcsObject>,
    #[serde(default, rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GcsObject {
    name: String,
}

#[derive(Debug, Deserialize)]
struct NpmPackageMetadata {
    #[serde(rename = "dist-tags")]
    dist_tags: std::collections::BTreeMap<String, String>,
}
