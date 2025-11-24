#[path = "../examples/support/fixtures.rs"]
mod fixtures;

use serde_json::Value;

#[test]
fn streaming_fixture_covers_event_shapes() {
    let events: Vec<&'static str> = fixtures::streaming_events().collect();
    assert!(
        events.iter().any(|line| line.contains("thread.started")),
        "streaming fixture should include thread.start"
    );
    assert!(
        events.iter().any(|line| line.contains("item.")),
        "streaming fixture should include at least one item event"
    );

    for line in events {
        let value: Value = serde_json::from_str(line).expect("valid streaming fixture JSON");
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .expect("fixture event has type");

        if kind.starts_with("item.") {
            let item = value
                .get("item")
                .expect("item.* events include an item body");
            assert!(item.get("type").is_some(), "item body includes type");
            assert!(
                item.get("text").is_some() || item.get("content").is_some(),
                "item body includes text/content"
            );
        }

        if kind == "turn.completed" {
            assert!(
                value.get("usage").is_some(),
                "turn.completed carries token usage"
            );
        }
    }
}

#[test]
fn resume_fixture_includes_thread_and_turn_ids() {
    let resume_events: Vec<&'static str> = fixtures::resume_events().collect();
    assert!(
        !resume_events.is_empty(),
        "resume fixture should not be empty"
    );

    for line in resume_events {
        let value: Value = serde_json::from_str(line).expect("valid resume fixture JSON");
        assert!(value.get("type").is_some(), "resume events carry type");
        if value.get("type").and_then(Value::as_str) == Some("thread.started") {
            assert!(
                value.get("thread_id").is_some(),
                "thread.started carries thread_id"
            );
        }
        if value.get("type").and_then(Value::as_str) == Some("turn.completed") {
            assert!(
                value.get("usage").is_some(),
                "resume turn completion includes usage"
            );
        }
    }
}

#[test]
fn apply_fixture_parses_and_carries_exit_code() {
    let result: Value =
        serde_json::from_str(fixtures::apply_result()).expect("apply result fixture parses");
    assert_eq!(
        result.get("type").and_then(Value::as_str),
        Some("apply.result"),
        "apply result fixture has type"
    );
    assert!(result.get("exit_code").is_some(), "exit_code is present");
    assert!(result.get("stdout").is_some(), "stdout is present");
    assert!(result.get("stderr").is_some(), "stderr is present");
}
