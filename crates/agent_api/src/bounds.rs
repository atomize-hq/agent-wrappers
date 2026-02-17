use crate::{AgentWrapperCompletion, AgentWrapperEvent, AgentWrapperEventKind};

pub(crate) const CHANNEL_BOUND_BYTES: usize = 128;
pub(crate) const TEXT_BOUND_BYTES: usize = 65_536;
pub(crate) const MESSAGE_BOUND_BYTES: usize = 4_096;
pub(crate) const DATA_BOUND_BYTES: usize = 65_536;

pub(crate) fn enforce_event_bounds(event: AgentWrapperEvent) -> Vec<AgentWrapperEvent> {
    let mut event = event;
    event.channel = enforce_channel_bound(event.channel);
    event.message = event.message.map(enforce_message_bound);
    event.data = event.data.map(enforce_data_bound);

    if event.kind != AgentWrapperEventKind::TextOutput {
        return vec![event];
    }

    let Some(text) = event.text.clone() else {
        return vec![event];
    };

    if text.len() <= TEXT_BOUND_BYTES {
        return vec![event];
    }

    split_utf8_chunks(&text, TEXT_BOUND_BYTES)
        .into_iter()
        .map(|chunk| {
            let mut e = event.clone();
            e.text = Some(chunk);
            e
        })
        .collect()
}

pub(crate) fn enforce_completion_bounds(
    completion: AgentWrapperCompletion,
) -> AgentWrapperCompletion {
    let mut completion = completion;
    completion.data = completion.data.map(enforce_data_bound);
    completion
}

fn enforce_channel_bound(channel: Option<String>) -> Option<String> {
    let channel = channel?;
    if channel.len() <= CHANNEL_BOUND_BYTES {
        Some(channel)
    } else {
        None
    }
}

fn enforce_message_bound(message: String) -> String {
    if message.len() <= MESSAGE_BOUND_BYTES {
        return message;
    }

    const SUFFIX: &str = "…(truncated)";
    let suffix_bytes = SUFFIX.len();
    if MESSAGE_BOUND_BYTES > suffix_bytes {
        let prefix = utf8_truncate_to_bytes(&message, MESSAGE_BOUND_BYTES - suffix_bytes);
        let mut out = String::with_capacity(MESSAGE_BOUND_BYTES);
        out.push_str(prefix);
        out.push_str(SUFFIX);
        out
    } else {
        utf8_truncate_to_bytes("…", MESSAGE_BOUND_BYTES).to_string()
    }
}

fn enforce_data_bound(data: serde_json::Value) -> serde_json::Value {
    let bytes = serde_json::to_vec(&data)
        .map(|v| v.len())
        .unwrap_or(usize::MAX);
    if bytes <= DATA_BOUND_BYTES {
        data
    } else {
        serde_json::json!({ "dropped": { "reason": "oversize" } })
    }
}

fn split_utf8_chunks(text: &str, bound_bytes: usize) -> Vec<String> {
    if bound_bytes == 0 {
        return Vec::new();
    }
    if text.len() <= bound_bytes {
        return vec![text.to_string()];
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    while start < text.len() {
        let mut end = std::cmp::min(start + bound_bytes, text.len());
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            let ch_len = text[start..]
                .chars()
                .next()
                .map(|ch| ch.len_utf8())
                .unwrap_or(1);
            end = std::cmp::min(start + ch_len, text.len());
        }
        out.push(text[start..end].to_string());
        start = end;
    }
    out
}

fn utf8_truncate_to_bytes(s: &str, bound_bytes: usize) -> &str {
    if s.len() <= bound_bytes {
        return s;
    }
    let mut end = std::cmp::min(bound_bytes, s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentWrapperEventKind, AgentWrapperKind};

    #[test]
    fn channel_over_bound_is_dropped() {
        let event = AgentWrapperEvent {
            agent_kind: AgentWrapperKind("codex".to_string()),
            kind: AgentWrapperEventKind::Status,
            channel: Some("a".repeat(CHANNEL_BOUND_BYTES + 1)),
            text: None,
            message: None,
            data: None,
        };
        let out = enforce_event_bounds(event);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].channel, None);
    }

    #[test]
    fn message_over_bound_is_truncated_with_suffix() {
        let event = AgentWrapperEvent {
            agent_kind: AgentWrapperKind("codex".to_string()),
            kind: AgentWrapperEventKind::Error,
            channel: None,
            text: None,
            message: Some("a".repeat(MESSAGE_BOUND_BYTES + 10)),
            data: None,
        };
        let out = enforce_event_bounds(event);
        assert_eq!(out.len(), 1);
        let message = out[0].message.as_deref().expect("message");
        assert!(message.len() <= MESSAGE_BOUND_BYTES);
        assert!(message.ends_with("…(truncated)"));
    }

    #[test]
    fn text_over_bound_is_split_deterministically() {
        let text = "a".repeat(TEXT_BOUND_BYTES + 10);
        let event = AgentWrapperEvent {
            agent_kind: AgentWrapperKind("codex".to_string()),
            kind: AgentWrapperEventKind::TextOutput,
            channel: Some("assistant".to_string()),
            text: Some(text.clone()),
            message: None,
            data: None,
        };
        let out = enforce_event_bounds(event);
        assert!(out.len() >= 2);
        for e in out.iter() {
            let t = e.text.as_deref().expect("text");
            assert!(t.len() <= TEXT_BOUND_BYTES);
        }
        let recombined: String = out
            .iter()
            .map(|e| e.text.as_deref().unwrap())
            .collect::<Vec<_>>()
            .join("");
        assert_eq!(recombined, text);
    }

    #[test]
    fn data_over_bound_is_replaced_with_dropped_reason() {
        let large = serde_json::Value::String("a".repeat(DATA_BOUND_BYTES + 10));
        let event = AgentWrapperEvent {
            agent_kind: AgentWrapperKind("codex".to_string()),
            kind: AgentWrapperEventKind::ToolCall,
            channel: None,
            text: None,
            message: None,
            data: Some(large),
        };
        let out = enforce_event_bounds(event);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].data.as_ref().and_then(|v| v.get("dropped")),
            Some(&serde_json::json!({ "reason": "oversize" }))
        );
    }

    #[test]
    fn completion_data_over_bound_is_replaced_with_dropped_reason() {
        let completion = AgentWrapperCompletion {
            status: std::process::Command::new("true")
                .status()
                .expect("spawn true"),
            final_text: None,
            data: Some(serde_json::Value::String("a".repeat(DATA_BOUND_BYTES + 10))),
        };
        let bounded = enforce_completion_bounds(completion);
        assert_eq!(
            bounded.data.as_ref().and_then(|v| v.get("dropped")),
            Some(&serde_json::json!({ "reason": "oversize" }))
        );
    }
}
