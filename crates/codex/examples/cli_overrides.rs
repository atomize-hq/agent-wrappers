use std::{env, path::PathBuf, time::Duration};

use codex::{ApprovalPolicy, CodexClient, ExecRequest, FlagState, LocalProvider, SandboxMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Usage: cargo run -p codex --example cli_overrides -- "<prompt>" [cd]
    // Maps to: codex exec --config model_verbosity=high --config features.search=true
    // --config model_reasoning_effort=low --ask-for-approval on-request
    // --sandbox workspace-write --local-provider ollama --oss --enable builder-toggle
    // --disable legacy-flow --enable request-toggle --search [--cd <dir>]
    let mut args = env::args().skip(1);
    let prompt = args.next().expect("usage: cli_overrides <prompt> [cd]");
    let cd = args.next().map(PathBuf::from);

    let mut builder = CodexClient::builder()
        .timeout(Duration::from_secs(45))
        .mirror_stdout(false)
        .approval_policy(ApprovalPolicy::OnRequest)
        .sandbox_mode(SandboxMode::WorkspaceWrite)
        .local_provider(LocalProvider::Ollama)
        .oss(true)
        .enable_feature("builder-toggle")
        .disable_feature("legacy-flow")
        .config_override("model_verbosity", "high")
        .config_override("features.search", "true");

    if let Some(path) = &cd {
        builder = builder.cd(path);
    }

    let client = builder.build();

    let mut request = ExecRequest::new(prompt)
        .config_override("model_reasoning_effort", "low")
        .enable_feature("request-toggle");
    request.overrides.search = FlagState::Enable;
    if cd.is_none() {
        request.overrides.cd = Some(env::current_dir()?);
    }

    let response = client.send_prompt_with(request).await?;
    println!("{response}");
    Ok(())
}
