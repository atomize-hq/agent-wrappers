use std::{collections::BTreeMap, time::Duration};

use serde_json::Value;

use super::{BackendDefaults, BackendHarnessAdapter, NormalizedRequest};
use crate::{AgentWrapperError, AgentWrapperRunRequest};

fn validate_extension_keys_fail_closed<A: BackendHarnessAdapter>(
    adapter: &A,
    request: &AgentWrapperRunRequest,
) -> Result<(), AgentWrapperError> {
    let supported: &[&str] = adapter.supported_extension_keys();
    for key in request.extensions.keys() {
        if !supported.contains(&key.as_str()) {
            return Err(AgentWrapperError::UnsupportedCapability {
                agent_kind: adapter.kind().as_str().to_string(),
                capability: key.clone(),
            });
        }
    }
    Ok(())
}

fn merge_env_backend_defaults_then_request(
    defaults: &BTreeMap<String, String>,
    request: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = defaults.clone();
    merged.extend(request.clone());
    merged
}

fn derive_effective_timeout(
    request_timeout: Option<Duration>,
    default_timeout: Option<Duration>,
) -> Option<Duration> {
    request_timeout.or(default_timeout)
}

pub(crate) fn normalize_request<A: BackendHarnessAdapter>(
    adapter: &A,
    defaults: &BackendDefaults,
    request: AgentWrapperRunRequest,
) -> Result<NormalizedRequest<A::Policy>, AgentWrapperError> {
    if request.prompt.trim().is_empty() {
        return Err(AgentWrapperError::InvalidRequest {
            message: "prompt must not be empty".to_string(),
        });
    }

    validate_extension_keys_fail_closed(adapter, &request)?;
    let policy = adapter.validate_and_extract_policy(&request)?;

    let env = merge_env_backend_defaults_then_request(&defaults.env, &request.env);
    let effective_timeout = derive_effective_timeout(request.timeout, defaults.default_timeout);

    let agent_kind = adapter.kind();
    let prompt = request.prompt;
    let working_dir = request.working_dir;

    Ok(NormalizedRequest {
        agent_kind,
        prompt,
        working_dir,
        effective_timeout,
        env,
        policy,
    })
}

#[allow(dead_code)]
fn parse_ext_bool(value: &Value, key: &str) -> Result<bool, AgentWrapperError> {
    value
        .as_bool()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a boolean"),
        })
}

#[allow(dead_code)]
fn parse_ext_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AgentWrapperError> {
    value
        .as_str()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a string"),
        })
}

#[allow(dead_code)]
fn parse_ext_string_enum<'a>(
    value: &'a Value,
    key: &str,
    allowed: &[&str],
) -> Result<&'a str, AgentWrapperError> {
    let raw = parse_ext_string(value, key)?;
    if allowed.contains(&raw) {
        return Ok(raw);
    }

    let allowed = allowed.join(" | ");
    Err(AgentWrapperError::InvalidRequest {
        message: format!("{key} must be one of: {allowed}"),
    })
}

#[cfg(test)]
mod tests;
