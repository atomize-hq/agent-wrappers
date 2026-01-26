use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Parser)]
pub struct Args {
    /// Path to the `codex` binary to snapshot.
    #[arg(long)]
    pub codex_binary: PathBuf,

    /// Output directory (writes `current.json` and optional raw help captures under it).
    #[arg(long)]
    pub out_dir: PathBuf,

    /// Also write raw `--help` output under `raw_help/<version>/...` for debugging parser drift.
    #[arg(long)]
    pub capture_raw_help: bool,

    /// Path to `cli_manifests/codex/supplement/commands.json` (schema v1).
    #[arg(long)]
    pub supplement: Option<PathBuf>,

    /// Override `collected_at` (RFC3339). Intended for determinism in tests/CI.
    #[arg(long)]
    pub collected_at: Option<String>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid codex binary path: {0}")]
    InvalidCodexBinary(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("supplement file version must be 1 (got {0})")]
    SupplementVersion(u32),
    #[error("invalid collected_at (must be RFC3339): {0}")]
    CollectedAt(String),
}

pub fn run(args: Args) -> Result<(), Error> {
    let codex_binary = fs::canonicalize(&args.codex_binary)
        .map_err(|_| Error::InvalidCodexBinary(args.codex_binary.clone()))?;
    if !codex_binary.is_file() {
        return Err(Error::InvalidCodexBinary(codex_binary));
    }

    fs::create_dir_all(&args.out_dir)?;

    let collected_at = match args.collected_at {
        Some(s) => {
            OffsetDateTime::parse(&s, &Rfc3339).map_err(|_| Error::CollectedAt(s.clone()))?;
            s
        }
        None => OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| {
                // Should be infallible with well-known formatter; keep a deterministic fallback.
                "1970-01-01T00:00:00Z".to_string()
            }),
    };

    let binary_meta = BinaryMetadata::collect(&codex_binary)?;
    let (version_output, semantic_version, channel, commit) = probe_version(&codex_binary)?;

    let mut command_entries = BTreeMap::<Vec<String>, CommandSnapshot>::new();
    let mut visited = BTreeSet::<Vec<String>>::new();

    let root_help = run_codex_help(&codex_binary, &[])?;
    let root_parsed = parse_help(&root_help);

    let version_dir = semantic_version
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    if args.capture_raw_help {
        write_raw_help(&args.out_dir, &version_dir, &[], &root_help)?;
    }

    for token in root_parsed.subcommands {
        collect_command_recursive(
            &codex_binary,
            &args.out_dir,
            &version_dir,
            args.capture_raw_help,
            vec![token],
            &mut visited,
            &mut command_entries,
        )?;
    }

    let (known_omissions, supplemented) =
        apply_supplements(args.supplement.as_deref(), &mut command_entries)?;

    let mut commands: Vec<CommandSnapshot> = command_entries.into_values().collect();
    commands.sort_by(|a, b| cmp_path(&a.path, &b.path));

    let snapshot = SnapshotV1 {
        snapshot_schema_version: 1,
        tool: "codex-cli".to_string(),
        collected_at,
        binary: BinarySnapshot {
            sha256: binary_meta.sha256,
            size_bytes: binary_meta.size_bytes,
            platform: BinaryPlatform {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            },
            version_output,
            semantic_version,
            channel,
            commit,
        },
        commands,
        features: None,
        known_omissions: if supplemented {
            Some(known_omissions)
        } else {
            None
        },
    };

    let json = serde_json::to_string_pretty(&snapshot)?;
    let out_path = args.out_dir.join("current.json");
    fs::write(out_path, format!("{json}\n"))?;
    Ok(())
}

fn collect_command_recursive(
    codex_binary: &Path,
    out_dir: &Path,
    version_dir: &str,
    capture_raw_help: bool,
    path: Vec<String>,
    visited: &mut BTreeSet<Vec<String>>,
    out: &mut BTreeMap<Vec<String>, CommandSnapshot>,
) -> Result<(), Error> {
    if !visited.insert(path.clone()) {
        return Ok(());
    }

    let help = run_codex_help(codex_binary, &path)?;
    let parsed = parse_help(&help);

    if capture_raw_help {
        write_raw_help(out_dir, version_dir, &path, &help)?;
    }

    let mut flags = parsed.flags;
    if !flags.is_empty() {
        flags.sort_by(flag_sort_key);
    }

    let entry = CommandSnapshot {
        path: path.clone(),
        about: parsed.about,
        usage: parsed.usage,
        stability: None,
        platforms: None,
        args: if parsed.args.is_empty() {
            None
        } else {
            Some(parsed.args)
        },
        flags: if flags.is_empty() { None } else { Some(flags) },
    };

    out.insert(path.clone(), entry);

    for sub in parsed.subcommands {
        let mut next = path.clone();
        next.push(sub);
        collect_command_recursive(
            codex_binary,
            out_dir,
            version_dir,
            capture_raw_help,
            next,
            visited,
            out,
        )?;
    }

    Ok(())
}

fn run_codex_help(codex_binary: &Path, path: &[String]) -> Result<String, Error> {
    let mut cmd = Command::new(codex_binary);
    // Prefer `codex help <path...>` for non-root commands. Some Codex CLI versions define
    // subcommands with free positional args (e.g., `codex exec [PROMPT] [COMMAND]`), which can
    // accidentally consume the token `help` and cause `--help` to be parsed as a subcommand.
    //
    // `codex help <path...>` avoids that ambiguity and is how Codex CLI itself recommends
    // requesting help for a subcommand.
    if path.is_empty() {
        cmd.arg("--help");
    } else {
        cmd.arg("help");
        cmd.args(path);
    }
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed(command_failed_message(&cmd, &output)));
    }
    Ok(normalize_text(&output.stdout, &output.stderr))
}

type VersionProbe = (String, Option<String>, Option<String>, Option<String>);

fn probe_version(codex_binary: &Path) -> Result<VersionProbe, Error> {
    let mut cmd = Command::new(codex_binary);
    cmd.arg("--version");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed(command_failed_message(&cmd, &output)));
    }
    let version_output = normalize_text(&output.stdout, &output.stderr)
        .trim()
        .to_string();

    let re_semver = Regex::new(r"(?P<v>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)").unwrap();
    let semantic_version = re_semver
        .captures(&version_output)
        .and_then(|c| c.name("v").map(|m| m.as_str().to_string()));

    let channel = semantic_version.as_ref().map(|v| {
        if v.contains("nightly") {
            "nightly".to_string()
        } else if v.contains("beta") {
            "beta".to_string()
        } else {
            "stable".to_string()
        }
    });

    let re_commit = Regex::new(r"(?i)\b([0-9a-f]{7,40})\b").unwrap();
    let commit = re_commit
        .captures(&version_output)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

    Ok((version_output, semantic_version, channel, commit))
}

fn command_failed_message(cmd: &Command, output: &Output) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{} (exit {})",
        format_command(cmd),
        output.status.code().unwrap_or(-1)
    ));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.trim().is_empty() {
        s.push_str(&format!("\nstdout:\n{stdout}"));
    }
    if !stderr.trim().is_empty() {
        s.push_str(&format!("\nstderr:\n{stderr}"));
    }
    s
}

fn format_command(cmd: &Command) -> String {
    let program = cmd.get_program().to_string_lossy();
    let args = cmd
        .get_args()
        .map(|a| shell_escape(a.to_string_lossy().as_ref()))
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {args}")
    }
}

fn shell_escape(s: &str) -> String {
    if s.bytes().all(
        |b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-' | b'.' | b'/' | b':'),
    ) {
        s.to_string()
    } else {
        format!("{s:?}")
    }
}

fn normalize_text(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    if !stdout.trim().is_empty() {
        stdout.replace("\r\n", "\n")
    } else {
        String::from_utf8_lossy(stderr).replace("\r\n", "\n")
    }
}

#[derive(Debug)]
struct ParsedHelp {
    about: Option<String>,
    usage: Option<String>,
    subcommands: Vec<String>,
    flags: Vec<FlagSnapshot>,
    args: Vec<ArgSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Commands,
    Options,
    Args,
}

fn parse_help(help: &str) -> ParsedHelp {
    let lines: Vec<&str> = help.lines().collect();

    let mut usage: Option<String> = None;
    let mut usage_lines: Vec<String> = Vec::new();
    let mut usage_started = false;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_lowercase().starts_with("usage:") {
            usage_started = true;
            let rest = trimmed["usage:".len()..].trim();
            if !rest.is_empty() {
                usage_lines.push(rest.to_string());
            }
            for cont in lines.iter().skip(idx + 1) {
                if cont.trim().is_empty() {
                    break;
                }
                if cont.starts_with(' ') || cont.starts_with('\t') {
                    let t = cont.trim();
                    if t.ends_with(':') && is_section_header(t) {
                        break;
                    }
                    usage_lines.push(t.to_string());
                } else {
                    break;
                }
            }
            break;
        }
    }
    if usage_started && !usage_lines.is_empty() {
        usage = Some(usage_lines.join("\n"));
    }

    let about = {
        let mut nonempty_indices = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| if l.trim().is_empty() { None } else { Some(i) })
            .collect::<Vec<_>>();
        if nonempty_indices.is_empty() {
            None
        } else {
            let title_idx = nonempty_indices.remove(0);
            let usage_idx = lines
                .iter()
                .position(|l| l.trim_start().to_ascii_lowercase().starts_with("usage:"))
                .unwrap_or(lines.len());
            let mut about_lines = Vec::new();
            for l in lines.iter().take(usage_idx).skip(title_idx + 1) {
                let t = l.trim();
                if t.is_empty() {
                    continue;
                }
                about_lines.push(t.to_string());
            }
            if about_lines.is_empty() {
                None
            } else {
                Some(about_lines.join("\n"))
            }
        }
    };

    let mut subcommands = Vec::new();
    let mut flags = Vec::new();
    let mut args = Vec::new();

    let mut section: Option<Section> = None;

    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            section = None;
            continue;
        }

        if let Some(s) = parse_section_header(t) {
            section = Some(s);
            continue;
        }

        match section {
            Some(Section::Commands) => {
                if let Some(token) = parse_command_token(line) {
                    subcommands.push(token);
                }
            }
            Some(Section::Options) => {
                if let Some(flag) = parse_flag_line(line) {
                    flags.push(flag);
                }
            }
            Some(Section::Args) => {
                if let Some(arg) = parse_arg_line(line) {
                    args.push(arg);
                } else if let Some(last) = args.last_mut() {
                    // Clap frequently wraps argument descriptions onto continuation lines with
                    // deeper indentation. Preserve these as part of the argument note so snapshots
                    // capture positional semantics with useful context.
                    let cont = line.trim();
                    if !cont.is_empty() {
                        match last.note.as_mut() {
                            Some(note) => {
                                note.push('\n');
                                note.push_str(cont);
                            }
                            None => last.note = Some(cont.to_string()),
                        }
                    }
                }
            }
            None => {}
        }
    }

    ParsedHelp {
        about,
        usage,
        subcommands,
        flags,
        args,
    }
}

fn is_section_header(t: &str) -> bool {
    matches!(
        t.trim_end_matches(':').to_ascii_lowercase().as_str(),
        "commands" | "subcommands" | "options" | "flags" | "arguments"
    )
}

fn parse_section_header(t: &str) -> Option<Section> {
    let header = t.trim_end_matches(':').to_ascii_lowercase();
    match header.as_str() {
        "commands" | "subcommands" => Some(Section::Commands),
        "options" | "flags" => Some(Section::Options),
        "arguments" => Some(Section::Args),
        _ => None,
    }
}

fn parse_command_token(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    // Only treat "command list" entries as subcommands when the help output includes an
    // on-the-same-line description. Wrapped descriptions (continuation lines) should not be
    // interpreted as additional command tokens.
    let (head, desc) = split_tokens_and_desc(trimmed);
    if desc.is_empty() {
        return None;
    }
    let token = head.split_whitespace().next()?;
    if token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(token.to_string())
    } else {
        None
    }
}

fn parse_flag_line(line: &str) -> Option<FlagSnapshot> {
    if !line.starts_with(' ') && !line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return None;
    }

    let (tokens_part, _) = split_tokens_and_desc(trimmed);
    let re_long = Regex::new(r"(?P<long>--[A-Za-z0-9][A-Za-z0-9-]*)").unwrap();
    let re_short = Regex::new(r"(?P<short>-[A-Za-z0-9])").unwrap();
    let long = re_long
        .captures(tokens_part)
        .and_then(|c| c.name("long").map(|m| m.as_str().to_string()));
    let short = re_short
        .captures(tokens_part)
        .and_then(|c| c.name("short").map(|m| m.as_str().to_string()));

    let value_name = Regex::new(r"<(?P<name>[^>]+)>")
        .unwrap()
        .captures(tokens_part)
        .and_then(|c| c.name("name").map(|m| m.as_str().to_string()));

    let takes_value = value_name.is_some();

    Some(FlagSnapshot {
        long,
        short,
        takes_value,
        value_name,
        repeatable: None,
        stability: None,
        platforms: None,
    })
}

fn split_tokens_and_desc(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b' ' && bytes[i + 1] == b' ' {
            let tokens = s[..i].trim_end();
            let desc = s[i..].trim();
            return (tokens, desc);
        }
    }
    (s.trim_end(), "")
}

fn parse_arg_line(line: &str) -> Option<ArgSnapshot> {
    if !line.starts_with(' ') && !line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    let (head, desc) = split_tokens_and_desc(trimmed);
    let token = head.split_whitespace().next()?;

    let (token, token_is_variadic) = token
        .strip_suffix("...")
        .map(|t| (t, true))
        .unwrap_or((token, false));

    let (required, mut name) = if token.starts_with('<') && token.ends_with('>') {
        (
            true,
            token
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string(),
        )
    } else if token.starts_with('[') && token.ends_with(']') {
        (
            false,
            token
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string(),
        )
    } else if token
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        // Be conservative: treat bare ALLCAPS tokens as positional args (common in some help
        // formats) but avoid mis-parsing wrapped description lines as arguments.
        (false, token.to_string())
    } else {
        return None;
    };

    let mut variadic = token_is_variadic;
    if let Some(stripped) = name.strip_suffix("...") {
        variadic = true;
        name = stripped.to_string();
    }

    if name.is_empty() {
        return None;
    }

    Some(ArgSnapshot {
        name,
        required,
        variadic,
        note: if desc.is_empty() {
            None
        } else {
            Some(desc.to_string())
        },
    })
}

fn flag_sort_key(a: &FlagSnapshot, b: &FlagSnapshot) -> std::cmp::Ordering {
    cmp_opt_str(&a.long, &b.long).then_with(|| cmp_opt_str(&a.short, &b.short))
}

fn cmp_opt_str(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn cmp_path(a: &[String], b: &[String]) -> std::cmp::Ordering {
    let mut i = 0usize;
    while i < a.len() && i < b.len() {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Equal => i += 1,
            non_eq => return non_eq,
        }
    }
    a.len().cmp(&b.len())
}

fn write_raw_help(
    out_dir: &Path,
    version_dir: &str,
    path: &[String],
    help: &str,
) -> Result<(), Error> {
    let rel = if path.is_empty() {
        PathBuf::from("raw_help").join(version_dir).join("help.txt")
    } else {
        let mut p = PathBuf::from("raw_help").join(version_dir).join("commands");
        for token in path {
            p.push(token);
        }
        p.join("help.txt")
    };
    let full = out_dir.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(full, help)?;
    Ok(())
}

#[derive(Debug)]
struct BinaryMetadata {
    sha256: String,
    size_bytes: u64,
}

impl BinaryMetadata {
    fn collect(path: &Path) -> Result<Self, Error> {
        let bytes = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = hex::encode(hasher.finalize());
        let size_bytes = bytes.len() as u64;
        Ok(Self { sha256, size_bytes })
    }
}

#[derive(Debug, Serialize)]
struct SnapshotV1 {
    snapshot_schema_version: u32,
    tool: String,
    collected_at: String,
    binary: BinarySnapshot,
    commands: Vec<CommandSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    known_omissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct BinarySnapshot {
    sha256: String,
    size_bytes: u64,
    platform: BinaryPlatform,
    version_output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
}

#[derive(Debug, Serialize)]
struct BinaryPlatform {
    os: String,
    arch: String,
}

#[derive(Debug, Serialize)]
struct CommandSnapshot {
    path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    platforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<ArgSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<FlagSnapshot>>,
}

#[derive(Debug, Serialize)]
struct ArgSnapshot {
    name: String,
    required: bool,
    variadic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct FlagSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    takes_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeatable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    platforms: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SupplementV1 {
    version: u32,
    commands: Vec<SupplementCommand>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SupplementCommand {
    path: Vec<String>,
    #[serde(default)]
    platforms: Option<Vec<String>>,
    note: String,
}

fn apply_supplements(
    supplement_path: Option<&Path>,
    commands: &mut BTreeMap<Vec<String>, CommandSnapshot>,
) -> Result<(Vec<String>, bool), Error> {
    let Some(path) = supplement_path else {
        return Ok((Vec::new(), false));
    };
    let text = fs::read_to_string(path)?;
    let supplement: SupplementV1 = serde_json::from_str(&text)?;
    if supplement.version != 1 {
        return Err(Error::SupplementVersion(supplement.version));
    }

    let mut known_omissions = Vec::new();
    let mut any_applied = false;

    for item in supplement.commands {
        let mut applied = false;
        let existed = commands.contains_key(&item.path);
        let entry = commands
            .entry(item.path.clone())
            .or_insert_with(|| CommandSnapshot {
                path: item.path.clone(),
                about: None,
                usage: None,
                stability: None,
                platforms: None,
                args: None,
                flags: None,
            });

        if !existed {
            applied = true;
        }

        if let Some(platforms) = item.platforms {
            if entry.platforms.as_ref() != Some(&platforms) {
                entry.platforms = Some(platforms);
                applied = true;
            }
        }

        if applied {
            any_applied = true;
            known_omissions.push(format!(
                "supplement/commands.json:v1:{}",
                item.path.join(" ")
            ));
        }
    }

    known_omissions.sort();
    known_omissions.dedup();
    Ok((known_omissions, any_applied))
}
