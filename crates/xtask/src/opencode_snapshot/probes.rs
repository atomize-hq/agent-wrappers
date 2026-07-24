use std::{fs, path::Path, process::Command};

use regex::Regex;
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{util, Error};

pub(super) fn deterministic_rfc3339_now() -> String {
    if let Ok(v) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(secs) = v.parse::<i64>() {
            if let Ok(ts) = OffsetDateTime::from_unix_timestamp(secs) {
                return ts
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
            }
        }
    }
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub(super) struct BinaryMetadata {
    pub(super) sha256: String,
    pub(super) size_bytes: u64,
}

impl BinaryMetadata {
    pub(super) fn collect(path: &Path) -> Result<Self, Error> {
        let bytes = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = hex::encode(hasher.finalize());
        let size_bytes = bytes.len() as u64;
        Ok(Self { sha256, size_bytes })
    }
}

pub(super) fn probe_version(opencode_binary: &Path) -> Result<(String, Option<String>), Error> {
    let mut cmd = Command::new(opencode_binary);
    cmd.arg("--version");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed(util::command_failed_message(
            &cmd, &output,
        )));
    }
    let version_output = util::normalize_text(&output.stdout, &output.stderr)
        .trim()
        .to_string();

    let re_semver = Regex::new(r"(?P<v>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)").unwrap();
    let semantic_version = re_semver
        .captures(&version_output)
        .and_then(|c| c.name("v").map(|m| m.as_str().to_string()));

    Ok((version_output, semantic_version))
}
