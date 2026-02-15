use claude_code::ClaudePrintRequest;

fn idx(argv: &[String], needle: &str) -> Option<usize> {
    argv.iter().position(|s| s == needle)
}

#[test]
fn root_flags_appear_before_prompt() {
    let req = ClaudePrintRequest::new("hello")
        .agent("agent-a")
        .agents(r#"{"agents":[]}"#)
        .allow_dangerously_skip_permissions(true)
        .append_system_prompt("append")
        .betas(["b1", "b2"])
        .continue_session(true)
        .debug(true)
        .debug_file("debug.log")
        .disable_slash_commands(true)
        .fallback_model("fallback")
        .files(["spec1", "spec2"])
        .fork_session(true)
        .from_pr(true)
        .ide(true)
        .include_partial_messages(true)
        .max_budget_usd(1.25)
        .mcp_debug(true)
        .no_session_persistence(true)
        .plugin_dirs(["/tmp/plugins"])
        .replay_user_messages(true)
        .resume_value("session-1")
        .session_id("session-1")
        .setting_sources("env,file")
        .settings("settings.json")
        .system_prompt("system")
        .tools(["tool-a", "tool-b"])
        .verbose(true);

    let argv = req.argv();
    let prompt_idx = idx(&argv, "hello").expect("prompt present");

    for key in [
        "--agent",
        "--agents",
        "--allow-dangerously-skip-permissions",
        "--append-system-prompt",
        "--betas",
        "--continue",
        "--debug",
        "--debug-file",
        "--disable-slash-commands",
        "--fallback-model",
        "--file",
        "--fork-session",
        "--from-pr",
        "--ide",
        "--include-partial-messages",
        "--max-budget-usd",
        "--mcp-debug",
        "--no-session-persistence",
        "--plugin-dir",
        "--replay-user-messages",
        "--resume",
        "--session-id",
        "--setting-sources",
        "--settings",
        "--system-prompt",
        "--tools",
        "--verbose",
    ] {
        let i = idx(&argv, key).unwrap_or_else(|| panic!("missing flag {key}"));
        assert!(i < prompt_idx, "flag {key} should precede prompt");
    }

    assert_eq!(argv[prompt_idx], "hello");
}

#[test]
fn resume_value_wins_over_resume_bool() {
    let argv = ClaudePrintRequest::new("hello")
        .resume(true)
        .resume_value("abc")
        .argv();
    let i = idx(&argv, "--resume").expect("resume");
    assert_eq!(argv.get(i + 1).map(String::as_str), Some("abc"));
}

#[test]
fn chrome_mode_emits_exactly_one_flag() {
    let chrome = ClaudePrintRequest::new("hello").chrome().argv();
    assert!(idx(&chrome, "--chrome").is_some());
    assert!(idx(&chrome, "--no-chrome").is_none());

    let no_chrome = ClaudePrintRequest::new("hello").no_chrome().argv();
    assert!(idx(&no_chrome, "--chrome").is_none());
    assert!(idx(&no_chrome, "--no-chrome").is_some());
}

#[test]
fn no_prompt_omits_prompt_positional() {
    let argv = ClaudePrintRequest::new("hello")
        .no_prompt()
        .continue_session(true)
        .argv();
    assert!(idx(&argv, "hello").is_none());
}
