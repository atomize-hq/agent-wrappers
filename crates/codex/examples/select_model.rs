//! Demonstrates selecting a custom Codex model.
//! Usage:
//! ```powershell
//! cargo run -p codex --example select_model -- gpt-5-codex -- "Explain rustfmt defaults"
//! ```

use codex::CodexClient;
use std::{env, error::Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let model = args
        .next()
        .ok_or("Provide a model name followed by a prompt")?;
    let prompt_parts: Vec<String> = args.collect();
    if prompt_parts.is_empty() {
        return Err("Provide a prompt after the model".into());
    }
    let prompt = prompt_parts.join(" ");

    let client = CodexClient::builder().model(model).build();
    let response = client.send_prompt(&prompt).await?;
    println!("{response}");
    Ok(())
}
