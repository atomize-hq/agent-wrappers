use super::*;

#[cfg(unix)]
#[tokio::test]
async fn features_list_maps_overrides_and_json_flag() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
echo "$PWD" 1>&2
printf "%s\n" "$@" 1>&2
cat <<'JSON'
[{"name":"json-stream","stage":"stable","enabled":true},{"name":"cloud-exec","stage":"experimental","enabled":false}]
JSON
"#,
    );

    let workdir = dir.path().join("features-workdir");
    std_fs::create_dir_all(&workdir).unwrap();

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .working_dir(&workdir)
        .approval_policy(ApprovalPolicy::OnRequest)
        .search(true)
        .build();

    let output = client
        .list_features(
            FeaturesListRequest::new()
                .json(true)
                .profile("dev")
                .config_override("features.extras", "true"),
        )
        .await
        .unwrap();

    assert_eq!(output.format, FeaturesListFormat::Json);
    assert_eq!(output.features.len(), 2);
    assert_eq!(output.features[0].stage, Some(CodexFeatureStage::Stable));
    assert!(output.features[0].enabled);
    assert!(!output.features[1].enabled);

    let mut lines = output.stderr.lines();
    let pwd = lines.next().unwrap();
    assert_eq!(Path::new(pwd), workdir.as_path());

    let args: Vec<_> = lines.map(str::to_string).collect();
    assert_eq!(
        args,
        vec![
            "features",
            "list",
            "--config",
            "features.extras=true",
            "--profile",
            "dev",
            "--ask-for-approval",
            "on-request",
            "--search",
            "--json"
        ]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn supports_help_review_fork_resume_and_features_commands() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
printf "%s\n" "$@"
"#,
    );

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let features = client
        .features(FeaturesCommandRequest::new())
        .await
        .unwrap();
    assert_eq!(
        features.stdout.lines().collect::<Vec<_>>(),
        vec!["features"]
    );

    let help = client
        .help(HelpCommandRequest::new(HelpScope::Root).command(["exec", "review"]))
        .await
        .unwrap();
    assert_eq!(
        help.stdout.lines().collect::<Vec<_>>(),
        vec!["help", "exec", "review"]
    );

    let review = client
        .review(
            ReviewCommandRequest::new()
                .base("main")
                .commit("abc123")
                .title("hello")
                .uncommitted(true)
                .prompt("please review"),
        )
        .await
        .unwrap();
    assert_eq!(
        review.stdout.lines().collect::<Vec<_>>(),
        vec![
            "review",
            "--base",
            "main",
            "--commit",
            "abc123",
            "--title",
            "hello",
            "--uncommitted",
            "please review"
        ]
    );

    let exec_review = client
        .exec_review(
            ExecReviewCommandRequest::new()
                .base("main")
                .commit("abc123")
                .title("hello")
                .uncommitted(true)
                .json(true)
                .prompt("please review"),
        )
        .await
        .unwrap();
    assert_eq!(
        exec_review.stdout.lines().collect::<Vec<_>>(),
        vec![
            "exec",
            "review",
            "--base",
            "main",
            "--commit",
            "abc123",
            "--json",
            "--skip-git-repo-check",
            "--title",
            "hello",
            "--uncommitted",
            "please review"
        ]
    );

    let resume = client
        .resume_session(
            ResumeSessionRequest::new()
                .all(true)
                .last(true)
                .session_id("sess-1")
                .prompt("resume prompt"),
        )
        .await
        .unwrap();
    assert_eq!(
        resume.stdout.lines().collect::<Vec<_>>(),
        vec!["resume", "--all", "--last", "sess-1", "resume prompt"]
    );

    let fork = client
        .fork_session(
            ForkSessionRequest::new()
                .all(true)
                .last(true)
                .session_id("sess-1")
                .prompt("fork prompt"),
        )
        .await
        .unwrap();
    assert_eq!(
        fork.stdout.lines().collect::<Vec<_>>(),
        vec!["fork", "--all", "--last", "sess-1", "fork prompt"]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn cloud_list_parses_json_and_maps_args() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
printf "%s\n" "$@" 1>&2
cat <<'JSON'
{"tasks":[],"cursor":null}
JSON
"#,
    );

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let output = client
        .cloud_list(
            CloudListRequest::new()
                .json(true)
                .env_id("env-1")
                .limit(3)
                .cursor("cur-1"),
        )
        .await
        .unwrap();

    assert_eq!(output.json, Some(json!({"tasks": [], "cursor": null})));
    assert_eq!(
        output.stderr.lines().collect::<Vec<_>>(),
        vec!["cloud", "list", "--env", "env-1", "--limit", "3", "--cursor", "cur-1", "--json"]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn cloud_exec_maps_args_and_rejects_empty_env_id() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
printf "%s\n" "$@"
"#,
    );

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let output = client
        .cloud_exec(
            CloudExecRequest::new("env-1")
                .attempts(2)
                .branch("main")
                .query("hello"),
        )
        .await
        .unwrap();
    assert_eq!(
        output.stdout.lines().collect::<Vec<_>>(),
        vec![
            "cloud",
            "exec",
            "--env",
            "env-1",
            "--attempts",
            "2",
            "--branch",
            "main",
            "hello"
        ]
    );

    let err = client
        .cloud_exec(CloudExecRequest::new("  "))
        .await
        .unwrap_err();
    assert!(matches!(err, CodexError::EmptyEnvId));
}

#[cfg(unix)]
#[tokio::test]
async fn mcp_list_get_and_add_map_args_and_parse_json() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
printf "%s\n" "$@" 1>&2
cat <<'JSON'
{"servers":[{"name":"files"}]}
JSON
"#,
    );

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let list = client
        .mcp_list(McpListRequest::new().json(true))
        .await
        .unwrap();
    assert_eq!(list.json, Some(json!({"servers": [{"name": "files"}]})));
    assert_eq!(
        list.stderr.lines().collect::<Vec<_>>(),
        vec!["mcp", "list", "--json"]
    );

    let get = client
        .mcp_get(McpGetRequest::new("files").json(true))
        .await
        .unwrap();
    assert_eq!(get.json, Some(json!({"servers": [{"name": "files"}]})));
    assert_eq!(
        get.stderr.lines().collect::<Vec<_>>(),
        vec!["mcp", "get", "--json", "files"]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn mcp_add_maps_transports_and_validates_required_fields() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
printf "%s\n" "$@"
"#,
    );

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let stdio = client
        .mcp_add(
            McpAddRequest::stdio("files", vec![OsString::from("node"), OsString::from("srv")])
                .env("TOKEN", "abc"),
        )
        .await
        .unwrap();
    assert_eq!(
        stdio.stdout.lines().collect::<Vec<_>>(),
        vec![
            "mcp",
            "add",
            "files",
            "--env",
            "TOKEN=abc",
            "--",
            "node",
            "srv"
        ]
    );

    let http = client
        .mcp_add(
            McpAddRequest::streamable_http("http", "https://example.test")
                .bearer_token_env_var("TOKEN_ENV"),
        )
        .await
        .unwrap();
    assert_eq!(
        http.stdout.lines().collect::<Vec<_>>(),
        vec![
            "mcp",
            "add",
            "http",
            "--url",
            "https://example.test",
            "--bearer-token-env-var",
            "TOKEN_ENV"
        ]
    );

    let err = client
        .mcp_add(McpAddRequest::stdio("files", Vec::new()))
        .await
        .unwrap_err();
    assert!(matches!(err, CodexError::EmptyMcpCommand));

    let err = client
        .mcp_add(McpAddRequest::streamable_http("bad", "  "))
        .await
        .unwrap_err();
    assert!(matches!(err, CodexError::EmptyMcpUrl));
}

#[cfg(unix)]
#[tokio::test]
async fn app_server_codegen_maps_overrides_and_prettier() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
"#,
    );

    let workdir = dir.path().join("workdir");
    std_fs::create_dir_all(&workdir).unwrap();
    let out_dir = dir.path().join("out/ts");
    let prettier = dir.path().join("bin/prettier.js");

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .working_dir(&workdir)
        .approval_policy(ApprovalPolicy::OnRequest)
        .search(true)
        .build();

    let result = client
        .generate_app_server_bindings(
            AppServerCodegenRequest::typescript(&out_dir)
                .prettier(&prettier)
                .profile("dev")
                .config_override("features.codegen", "true"),
        )
        .await
        .unwrap();

    let mut lines = result.stdout.lines();
    let pwd = lines.next().unwrap();
    assert_eq!(Path::new(pwd), workdir.as_path());

    let args: Vec<_> = lines.map(str::to_string).collect();
    assert_eq!(
        args,
        vec![
            "app-server",
            "generate-ts",
            "--out",
            out_dir.to_string_lossy().as_ref(),
            "--config",
            "features.codegen=true",
            "--profile",
            "dev",
            "--ask-for-approval",
            "on-request",
            "--search",
            "--prettier",
            prettier.to_string_lossy().as_ref(),
        ]
    );
    assert!(out_dir.is_dir());
    assert_eq!(result.out_dir, out_dir);
    assert!(result.status.success());
}

#[cfg(unix)]
#[tokio::test]
async fn app_server_codegen_surfaces_non_zero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
echo "ts error"
echo "bad format" 1>&2
exit 5
"#,
    );

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let out_dir = dir.path().join("schema");
    let err = client
        .generate_app_server_bindings(AppServerCodegenRequest::json_schema(&out_dir))
        .await
        .unwrap_err();

    match err {
        CodexError::NonZeroExit { status, stderr } => {
            assert_eq!(status.code(), Some(5));
            assert!(stderr.contains("bad format"));
        }
        other => panic!("expected NonZeroExit, got {other:?}"),
    }
    assert!(out_dir.is_dir());
}

#[cfg(unix)]
#[tokio::test]
async fn responses_api_proxy_maps_flags_and_parses_server_info() {
    let dir = tempfile::tempdir().unwrap();
    let server_info = dir.path().join("server-info.json");
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
info_path=""
while [[ $# -gt 0 ]]; do
  if [[ $1 == "--server-info" ]]; then
info_path=$2
  fi
  shift
done
read -r key || exit 1
echo "key:${key}"
if [[ -n "$info_path" ]]; then
  printf '{"port":4567,"pid":1234}\n' > "$info_path"
fi
"#,
    );

    let workdir = dir.path().join("responses-workdir");
    std_fs::create_dir_all(&workdir).unwrap();

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .working_dir(&workdir)
        .build();

    let mut proxy = client
        .start_responses_api_proxy(
            ResponsesApiProxyRequest::new("sk-test-123")
                .port(8080)
                .server_info(&server_info)
                .http_shutdown(true)
                .upstream_url("https://example.com/v1/responses"),
        )
        .await
        .unwrap();

    assert_eq!(
        proxy.server_info_path.as_deref(),
        Some(server_info.as_path())
    );

    let stdout = proxy.child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();

    let pwd = lines.next_line().await.unwrap().unwrap();
    assert_eq!(Path::new(&pwd), workdir.as_path());

    let mut args = Vec::new();
    for _ in 0..8 {
        args.push(lines.next_line().await.unwrap().unwrap());
    }
    assert_eq!(
        args,
        vec![
            "responses-api-proxy",
            "--port",
            "8080",
            "--server-info",
            server_info.to_string_lossy().as_ref(),
            "--http-shutdown",
            "--upstream-url",
            "https://example.com/v1/responses",
        ]
    );

    let api_key_line = lines.next_line().await.unwrap().unwrap();
    assert_eq!(api_key_line, "key:sk-test-123");

    let info = proxy.read_server_info().await.unwrap().unwrap();
    assert_eq!(info.port, 4567);
    assert_eq!(info.pid, 1234);

    let status = proxy.child.wait().await.unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn responses_api_proxy_rejects_empty_api_key() {
    let client = CodexClient::builder().build();
    let err = client
        .start_responses_api_proxy(ResponsesApiProxyRequest::new("  "))
        .await
        .unwrap_err();
    assert!(matches!(err, CodexError::EmptyApiKey));
}

#[cfg(unix)]
#[tokio::test]
async fn stdio_to_uds_maps_args_and_pipes_stdio() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("bridge.sock");
    let script_path = write_fake_codex(
        dir.path(),
        r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
while read -r line; do
  echo "relay:${line}"
done
"#,
    );

    let workdir = dir.path().join("uds-workdir");
    std_fs::create_dir_all(&workdir).unwrap();

    let client = CodexClient::builder()
        .binary(&script_path)
        .mirror_stdout(false)
        .quiet(true)
        .working_dir(&workdir)
        .build();

    let request = StdioToUdsRequest::new(&socket_path).working_dir(&workdir);
    let mut child = match client.stdio_to_uds(request.clone()) {
        Ok(child) => child,
        Err(CodexError::Spawn { source, .. }) if source.raw_os_error() == Some(26) => {
            time::sleep(Duration::from_millis(25)).await;
            client.stdio_to_uds(request).unwrap()
        }
        Err(other) => panic!("unexpected spawn error: {other:?}"),
    };

    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();

    let pwd = lines.next_line().await.unwrap().unwrap();
    assert_eq!(Path::new(&pwd), workdir.as_path());

    let arg_one = lines.next_line().await.unwrap().unwrap();
    let arg_two = lines.next_line().await.unwrap().unwrap();
    assert_eq!(arg_one, "stdio-to-uds");
    assert_eq!(arg_two, socket_path.to_string_lossy().as_ref());

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"ping\n").await.unwrap();
    stdin.shutdown().await.unwrap();
    drop(stdin);

    let echoed = lines.next_line().await.unwrap().unwrap();
    assert_eq!(echoed, "relay:ping");

    let status = time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("stdio-to-uds wait timed out")
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn stdio_to_uds_rejects_empty_socket_path() {
    let client = CodexClient::builder().build();
    let err = client
        .stdio_to_uds(StdioToUdsRequest::new(PathBuf::new()))
        .unwrap_err();
    assert!(matches!(err, CodexError::EmptySocketPath));
}
