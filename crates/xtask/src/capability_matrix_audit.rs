use std::collections::{BTreeMap, BTreeSet};

use agent_api::AgentWrapperCapabilities;
use clap::Parser;

const AGENT_API_ORTHOGONALITY_ALLOWLIST: [&str; 4] = [
    "agent_api.run",
    "agent_api.events",
    "agent_api.events.live",
    "agent_api.exec.non_interactive",
];

#[derive(Debug, Parser)]
pub struct Args {}

pub fn run(_args: Args) -> Result<(), String> {
    let backends = crate::capability_matrix::collect_builtin_backend_capabilities();
    audit(&backends)
}

fn audit(backends: &BTreeMap<String, AgentWrapperCapabilities>) -> Result<(), String> {
    let all_capability_ids = union_of_capabilities(backends);

    let mut violations = Vec::<Violation>::new();
    for capability_id in all_capability_ids.iter() {
        if !capability_id.starts_with("agent_api.") {
            continue;
        }

        if AGENT_API_ORTHOGONALITY_ALLOWLIST.contains(&capability_id.as_str()) {
            continue;
        }

        let supported_by = supported_backends(backends, capability_id);
        if supported_by.len() < 2 {
            violations.push(Violation {
                capability_id: capability_id.clone(),
                supported_by,
            });
        }
    }

    if violations.is_empty() {
        return Ok(());
    }

    violations.sort_by(|a, b| a.capability_id.cmp(&b.capability_id));
    Err(render_report(backends, &violations))
}

fn union_of_capabilities(
    backends: &BTreeMap<String, AgentWrapperCapabilities>,
) -> BTreeSet<String> {
    let mut all_capability_ids = BTreeSet::<String>::new();
    for capabilities in backends.values() {
        all_capability_ids.extend(capabilities.ids.iter().cloned());
    }
    all_capability_ids
}

fn supported_backends(
    backends: &BTreeMap<String, AgentWrapperCapabilities>,
    capability_id: &str,
) -> Vec<String> {
    backends
        .iter()
        .filter_map(|(backend_id, capabilities)| {
            if capabilities.contains(capability_id) {
                Some(backend_id.clone())
            } else {
                None
            }
        })
        .collect()
}

fn render_report(
    backends: &BTreeMap<String, AgentWrapperCapabilities>,
    violations: &[Violation],
) -> String {
    let built_in_backends: Vec<&str> = backends.keys().map(|id| id.as_str()).collect();

    let mut allowlist_sorted: Vec<&str> = AGENT_API_ORTHOGONALITY_ALLOWLIST.to_vec();
    allowlist_sorted.sort();

    let mut out = String::new();
    out.push_str(&format!(
        "built-in backends: [{}]\n",
        built_in_backends.join(", ")
    ));
    out.push_str(&format!(
        "capability-matrix-audit failed: {} violation(s)\n",
        violations.len()
    ));
    for violation in violations {
        out.push_str(&format!(
            "- {}: supported by {} backend(s): [{}]\n",
            violation.capability_id,
            violation.supported_by.len(),
            violation.supported_by.join(", ")
        ));
    }
    out.push_str(&format!("allowlist: [{}]\n", allowlist_sorted.join(", ")));
    out
}

#[derive(Debug)]
struct Violation {
    capability_id: String,
    supported_by: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(ids: &[&str]) -> AgentWrapperCapabilities {
        AgentWrapperCapabilities {
            ids: ids.iter().map(|id| (*id).to_string()).collect(),
        }
    }

    #[test]
    fn non_allowlisted_agent_api_cap_supported_by_1_backend_fails() {
        let mut backends = BTreeMap::<String, AgentWrapperCapabilities>::new();
        backends.insert("claude_code".to_string(), caps(&["agent_api.run"]));
        backends.insert(
            "codex".to_string(),
            caps(&["agent_api.run", "agent_api.tools.results.v1"]),
        );

        let err = audit(&backends).unwrap_err();
        assert!(err.contains("built-in backends: [claude_code, codex]"));
        assert!(err.contains("agent_api.tools.results.v1"));
    }

    #[test]
    fn non_allowlisted_agent_api_cap_supported_by_2_backends_passes() {
        let mut backends = BTreeMap::<String, AgentWrapperCapabilities>::new();
        backends.insert(
            "claude_code".to_string(),
            caps(&["agent_api.run", "agent_api.tools.results.v1"]),
        );
        backends.insert(
            "codex".to_string(),
            caps(&["agent_api.run", "agent_api.tools.results.v1"]),
        );

        audit(&backends).unwrap();
    }

    #[test]
    fn allowlisted_agent_api_cap_supported_by_1_backend_is_ignored() {
        let mut backends = BTreeMap::<String, AgentWrapperCapabilities>::new();
        backends.insert("claude_code".to_string(), caps(&["agent_api.run"]));
        backends.insert("codex".to_string(), caps(&[]));

        audit(&backends).unwrap();
    }
}
