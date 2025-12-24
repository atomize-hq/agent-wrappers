# Audit of the Codex Wrapper and Coverage Gaps

The existing **Codex Rust wrapper** appears to cover **most** of OpenAI’s Codex CLI functionality. The Codex CLI itself offers a rich set of commands (e.g. exec, resume, apply, diff, sandbox, etc.) and flags. Below is a summary of the Codex CLI surface versus what the wrapper currently implements:

* **Core Commands:** The Codex CLI provides top-level commands like exec (and alias e), resume, apply (alias a), sandbox (alias debug), login/logout for authentication, mcp/mcp-server for Model Context Protocol servers, app-server for app integrations, features listing, etc[\[1\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L13). The wrapper includes Rust methods or helpers for *all the major stable commands*:

* **exec:** via CodexClient::send\_prompt and related streaming methods (wraps codex exec)[\[2\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L70-L73). It automatically adds flags like \--skip-git-repo-check and applies default timeouts and safe environment settings[\[3\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L61-L69).

* **resume:** via CodexClient::send\_prompt\_with(...) using an ExecRequest that can specify \--last or a session ID (this wraps codex exec resume under the hood)[\[2\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L70-L73). There’s also a higher-level CodexClient::resume example in docs.

* **apply & diff:** implemented as CodexClient::apply() and CodexClient::diff(), which shell out to codex apply/codex diff and capture stdout/stderr and exit code[\[4\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1899-L1907)[\[5\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1942-L1950). This covers the CLI’s ability to preview and apply code changes.

* **sandbox:** exposed via CodexClient::run\_sandbox(...), which runs codex sandbox \<platform\> for macOS, Linux, or Windows. The wrapper forwards appropriate flags like \--full-auto (and \--log-denials on macOS) and config overrides for sandbox runs[\[6\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L72-L75). (It intentionally does **not** forward certain global overrides like approval policy on this subcommand, since those don’t apply in sandbox mode[\[6\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L72-L75).) The wrapper doesn’t gate platform support – if the underlying CLI’s sandbox helper isn’t available (e.g. Windows), it will simply return a non-zero exit as an error[\[7\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L73-L75).

* **execpolicy check:** available via CodexClient::check\_execpolicy(...), wrapping codex execpolicy check with support for multiple \--policy files and the \--pretty flag[\[2\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L70-L73). The wrapper parses the JSON decision output (allow/prompt/forbidden) into a Rust enum for convenience.

* **features list:** available via CodexClient::list\_features(...), wrapping codex features list. The wrapper even supports the CLI’s optional \--json output for this command and will fall back to parsing text if JSON isn’t available[\[2\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L70-L73).

* **app-server generate:** exposed via CodexClient::generate\_app\_server\_bindings(...), which can call codex app-server generate-ts or generate-json-schema as needed[\[2\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L70-L73). It handles creating the output directory and even supports passing a \--prettier path for formatting the TypeScript output.

* **Utility commands:** The wrapper also covers utility subcommands. For example, CodexClient::start\_responses\_api\_proxy(...) wraps codex responses-api-proxy (it pipes in the API key and forwards \--port, \--server-info, etc.)[\[7\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L73-L75). Similarly, CodexClient::stdio\_to\_uds(...) wraps codex stdio-to-uds to bridge JSON-RPC over a Unix domain socket[\[7\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L73-L75).

* **Auth & Sessions:** The wrapper provides an AuthSessionHelper that can run codex login status, codex login (with API key or ChatGPT OAuth), and codex logout under an isolated CODEX\_HOME[\[8\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1372-L1381)[\[9\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1388-L1396). This covers checking login state and automating headless logins with an API key. It doesn’t implement anything beyond these basics – which matches the CLI’s capabilities (the CLI itself only has login/logout commands for auth). Notably, the wrapper’s auth integration is intentionally *basic*: it can ensure you’re logged in or trigger the login flow, but it doesn’t try to embed the full interactive OAuth – it spawns the CLI’s login process and leaves any browser-based steps to the user[\[9\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1388-L1396). This is acceptable since we typically handle auth once and then operate headlessly.

* **Flags and Options:** Virtually all **relevant CLI flags** are represented in the wrapper’s builder or request APIs. The Codex CLI has a set of **global flags** (like \-m/--model, \-p/--profile, \-s/--sandbox, \-a/--ask-for-approval, \--full-auto, \--dangerously-bypass-..., \-C/--cd, \--add-dir, \-i/--image, \--search, etc.)[\[10\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L22-L31), as well as **command-specific flags** (\--json, \--color, \--output-schema, \--output-last-message for exec; \--last/--all for resume; etc.)[\[11\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L34-L42). The wrapper’s CodexClientBuilder supports these through methods or configuration:

* For example, .model("..."), .profile("..."), .sandbox\_mode(...), .approval\_policy(...), .full\_auto(true), .dangerously\_bypass\_approvals\_and\_sandbox(true), .cd("dir"), .add\_dir("dir"), .image("file"), .search(true) are all provided to map to those flags[\[12\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L24-L31)[\[13\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L66-L69). Feature toggles are handled via .enable\_feature(...)/.disable\_feature(...) or .config\_override(...) for arbitrary config keys.

* Exec-specific toggles: .json(true) on the builder will cause \--json streaming to be used[\[14\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L62-L69), and the code uses ColorMode::Never by default to force non-ANSI output unless overridden (to simplify parsing)[\[14\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L62-L69). The wrapper also supports capturing artifacts: you can provide an ExecRequest with .output\_last\_message(path) or .output\_schema(path) to have the CLI write those out; if you don’t specify, it generates temp files under the hood[\[15\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L26-L34). These correspond to \--output-last-message and \--output-schema flags. The \--skip-git-repo-check is always applied by default for non-interactive exec calls[\[16\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L61-L68).

* In summary, **no major CLI flags appear to be missing** from the wrapper; the “CLI parity” notes confirm the builder and request structs expose all important toggles[\[13\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L66-L69). This means as new flags are added to Codex CLI, we should aim to update the wrapper accordingly.

* **What’s *Not* Covered (Current Gaps):** Based on the audit, the wrapper intentionally omits or hasn’t yet implemented a few **experimental or less-used commands**:

* **codex cloud exec:** The Codex CLI has a top-level cloud command (currently experimental) which in turn has an exec subcommand[\[1\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L13). This is likely used to delegate tasks to Codex Cloud (OpenAI’s cloud-hosted agent service) instead of running locally. The wrapper does *not* have a specific method for cloud exec at this time – presumably because this feature is experimental and not a priority yet. If cloud execution becomes stable or necessary, we’d want to add support (or at least allow opting into it via config).

* **codex mcp subcommands (list/get/add/remove/login/logout):** These are experimental CLI commands for managing MCP server definitions and require enabling a config flag in Codex (experimental\_use\_rmcp\_client)[\[17\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L7-L14). The wrapper currently focuses on launching an MCP server (mcp-server) and interacting through it (via the JSON-RPC flows in mcp.rs), rather than wrapping the CLI’s management commands. In other words, we don’t have dedicated Rust functions for codex mcp list or codex mcp add – those can still be executed by shelling out if needed, but given their experimental nature, it’s reasonable that the wrapper hasn’t integrated them yet.

* **codex completion:** The CLI likely has a hidden completion command to generate shell completions. This isn’t relevant for programmatic use, so the wrapper doesn’t expose anything for it (not needed for Substrate).

* **Interactive TUI mode:** Running codex with no arguments launches the interactive terminal UI. The wrapper **never uses** the TUI mode (by design) – we always call specific subcommands in “headless” mode. This is intentional, as Substrate’s goal is to orchestrate agents without direct user interaction. So while not a “gap” per se, it’s worth noting we always pass a command (like exec) or use \--json to avoid ever invoking the full-screen UI. This means certain UI-only features (like the real-time TUI display or slash commands interface) are out of scope for the wrapper.

* **Anything not in Codex CLI v0.61:** Our reference CLI version (v0.61 as per the CLI\_MATRIX.md) is what the wrapper targets[\[18\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L1-L9). If OpenAI releases new major commands or flags in future versions, those would initially be unsupported until we update. For example, if a future Codex CLI adds a new subcommand codex analyze or new flags, we’d need to catch those. (We’ll address how to keep up with such changes in the next section.)

Overall, the codex-wrapper is in **excellent shape**, providing near one-to-one parity with the Codex CLI’s stable features. The only omissions are experimental features that we likely deferred (cloud mode, MCP management) and trivial commands (completions, help). Thus, the immediate “audit” action item is simply to **monitor upcoming Codex CLI releases** for any new flags or subcommands so we can add support as needed.

# Strategy for Maintaining Long-Lived CLI Wrappers

Given that we plan to support not only OpenAI’s Codex CLI but also other evolving AI coding CLI tools (Anthropic’s Claude Code, Google’s Gemini CLI, etc.), we need a **sustainable process** to track updates and quickly integrate new features or tools. The key goals are:

* **Stay Up-to-Date with CLI Changes:** Whenever a new version of a CLI tool is released, we should be able to identify what changed (new commands, flags, behaviors).

* **Minimize Manual Work:** We want to avoid manually trawling through release notes or diffing help text by eye for every update. Instead, we can automate the inventory of commands/flags.

* **Multi-Tool Extensibility:** Use a common approach that works for Codex, Claude Code, Gemini, and any others, despite differences in how they’re distributed (some are open-source, some closed-source).

* **Isolation & Testing:** Ensure we can test these CLI tools in isolation (in a sandbox or temporary context) so that integrating them into Substrate doesn’t risk our main environment. Substrate’s design already leans toward isolated execution (e.g. separate CODEX\_HOME directories, etc.), which we’ll continue for other tools.

Below, we outline a plan centered on **manifest files** and an **update pipeline**:

## 1\. CLI Command/Flag Manifest (JSON or TOML)

We will maintain a **structured manifest** for each supported CLI tool, listing its commands, subcommands, and flags (including descriptions or metadata as needed). This manifest acts as a **source of truth** for what the CLI can do, and it can be versioned alongside our code.

* For example, we might have a file codex-cli-v0.61.json (or .toml) that contains an object like:

* {  
    "version": "0.61",  
    "commands": {  
       "exec": {  
           "aliases": \["e"\],  
           "description": "Execute a prompt (non-interactive)",  
           "flags": {  
               "--skip-git-repo-check": {},  
               "--json": {},  
               "--output-schema": {"takes\_value": "path"},  
               "--output-last-message": {"takes\_value": "path"},  
               "--color": {"values": \["always","never","auto"\], "default": "auto"},  
               // ...  
           },  
           "subcommands": {  
               "resume": {  
                   "description": "Resume an exec session",  
                   "flags": {  
                       "--last": {},  
                       "--all": {}  
                   }  
               }  
           }  
       },  
       "resume": {  
           "description": "Resume an interactive session",  
           "flags": {"--last": {}, "--all": {}}  
       },  
       "apply": { /\* ... \*/ },  
       "diff": { /\* ... \*/ },  
       "sandbox": {  
           "aliases": \["debug"\],  
           "subcommands": {  
               "macos": {}, "linux": {}, "windows": {}  
           },  
           "flags": {  
             "--full-auto": {},  
             "--log-denials": {"platform": "macos"}  
           }  
       },  
       "login": { /\* ... \*/ },  
       "logout": { /\* ... \*/ },  
       "mcp": {  
           "flags": {},   
           "subcommands": {"list":{}, "get":{}, "add":{}, "remove":{}, "login":{}, "logout":{}}  
       },  
       "mcp-server": {},  
       "app-server": {  
           "subcommands": {"generate-ts": {}, "generate-json-schema": {}}  
       },  
       "features": {  
           "subcommands": {"list": {"flags": {"--json": {}}}}  
       },  
       "execpolicy": {  
           "subcommands": {"check": {"flags": {"--policy": {"repeatable": true}, "--pretty": {}}}}  
       },  
       "responses-api-proxy": {"flags": {"--port":{}, "--server-info":{}, "--http-shutdown":{}, "--upstream-url":{}}},  
       "stdio-to-uds": {}  
    }  
  }

* (The above is illustrative – the idea is to capture the CLI surface. We can choose how detailed to get; e.g. including descriptions and accepted values is nice to have, but at minimum we want the names of commands and flags.)

* **Why JSON/TOML?** A structured format allows programmatic comparison and generation. For instance, if we have codex-cli-v0.61.json and later codex-cli-v0.70.json, we can diff the two to see what changed. It also allows us to write validators or even code generators if we ever choose (for example, conceivably generating some boilerplate Rust code or tests from this data).

* The manifest for Codex can also include known **configuration keys** and **feature flags** if needed (as our current CLI\_MATRIX.md does[\[19\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L53-L62)). This is slightly separate from CLI flags, but important for understanding new toggles. We might keep config keys in the same file or a separate one. For now, commands/flags are the priority.

* We should store these manifest files in our repo (perhaps under a cli-manifests/ directory or similar). That way, updates are tracked via Git history. It also allows the team to easily review changes in a pull request when a manifest is updated (which signals “the CLI changed here”).

## 2\. Automated Inventory Extraction

Rather than writing the manifest by hand for each new release, we can **automate the extraction** of CLI info via a script or small program. The approach can differ per tool:

**For Codex CLI (OpenAI):** We can leverage the CLI’s built-in help text. Running codex \--help prints top-level usage and commands, and each subcommand has its own help (codex exec \--help, codex resume \--help, etc.). A script can invoke these with the latest Codex binary and parse the output: \- Since Codex CLI is built in Rust (and likely using clap or a similar parser), its help output is fairly structured (flags are listed with a leading \-- or \-, subcommands are listed, etc.). We can parse lines or use regex to identify command names and flags. \- We’ll need to handle multi-level commands. E.g., the script would first run codex \--help to get top-level commands (exec, resume, apply, …). Then recursively run codex \<subcmd\> \--help for each to get flags or deeper subcommands (for example, codex exec \--help will reveal resume as a sub-subcommand, and we’d run codex exec resume \--help). \- Some special cases: The sandbox command expects a platform argument, which might not appear in \--help output (as noted in our matrix, these aren’t shown in \--help but are valid options)[\[20\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L12-L14). For such cases, we may need to *hardcode* certain known variants or consult documentation. The same goes for anything the CLI help might omit. \- The output can be converted to our JSON/TOML format directly in the script.

**For Claude Code (Anthropic):** Claude’s CLI (claude command) is **closed-source and distributed via npm** (package @anthropic-ai/claude-code). We can still treat it as a black box: after installation, run claude \--help to hopefully get a list of commands. If the \--help output is insufficient or too minimal (since it might default to launching an interactive session), we’ll rely on official docs. Anthropic provides documentation on “Claude Code best practices” and presumably some CLI reference. In our script, we might need a curated list of Claude’s commands (from docs or usage) as a starting point. Likely, Claude Code’s interface is similar (e.g., claude to start interactive, and perhaps subcommands like claude exec for one-shot tasks, etc., but we’ll confirm via docs).

* **Auth for Claude CLI:** It requires an API key (or login) on first run. Our automation script might need to handle that (perhaps by setting an env var or passing a dummy key for help commands). Since we only need \--help, it might not require full login to display usage.

* Given Claude Code is **proprietary (closed-source)[\[21\]](https://prompt.security/blog/ai-coding-assistants-make-a-cli-comeback#:~:text=,focuses%20on%20vetted%20enterprise%20integration)**, we expect fewer surprise flags – new features will be announced via Anthropic’s release notes. But we’ll still treat it like Codex: parse the help or documentation for changes.

**For Gemini CLI (Google):** Gemini CLI is **fully open-source** and likely uses a Node.js or TS command framework (looking at the repo, it has packages/cli and commands). We have an advantage here: Google’s documentation for Gemini CLI is detailed, and the project invites contributions[\[21\]](https://prompt.security/blog/ai-coding-assistants-make-a-cli-comeback#:~:text=,focuses%20on%20vetted%20enterprise%20integration). We can retrieve commands in two ways: \- By installing it (npm install \-g @google/gemini-cli) and running gemini \--help and subcommand helps, similar to Codex. \- Or by inspecting their docs (the Gemini CLI docs have a “Commands” section that likely enumerates all commands) or even the source code (since it’s open source). \- Gemini CLI might have a slightly different command structure. For example, it might rely on a REPL by default but provide a **headless mode** or commands for non-interactive use[\[22\]](https://geminicli.com/docs/#:~:text=,Themes)[\[23\]](https://geminicli.com/docs/#:~:text=,all%20keyboard%20shortcuts%20to%20improve). We should confirm how to run a single prompt or script with it (the docs mention a headless mode for scripting[\[24\]](https://geminicli.com/docs/#:~:text=,Settings)). \- We should capture things like Gemini’s sub-tools (it has built-in tools for web search, filesystem, etc., possibly invoked via special syntax rather than separate CLI commands). These might not need listing as separate commands, but any CLI flags (e.g., to select model, or to control search permissions) should be tracked.

**General Approach:** We will likely create a **unified script** (perhaps in Python or Rust) that takes as input which CLI to analyze and produces the manifest. It could be something like:

\# Pseudocode/commands  
python scrape\_cli\_help.py \--tool codex \--binary ./codex \--output codex-cli-v0.70.json  
python scrape\_cli\_help.py \--tool claude \--binary $(which claude) \--output claude-cli-vX.json  
python scrape\_cli\_help.py \--tool gemini \--binary $(which gemini) \--output gemini-cli-vY.json

The script can contain tool-specific logic if needed (for instance, if claude \--help doesn’t list subcommands, we might call claude someKnownCommand \--help based on prior knowledge, or parse their docs site).

This process should be **iterative**. For the first run, we might manually verify the script’s output against known documentation to ensure it’s accurate. Once it’s working, updating to a new CLI version becomes as easy as running the script with the new binary.

## 3\. Tracking Versions and Changes

To know when to update, we should watch for new CLI releases:

* **Codex CLI:** OpenAI’s Codex CLI is on GitHub (openai/codex) with tags, and they publish release notes (changelog)[\[25\]](https://github.com/openai/codex#:~:text=If%20you%27re%20running%20into%20upgrade,entry%20on%20brew%20upgrade%20codex)[\[26\]](https://github.com/openai/codex#:~:text=Codex%20can%20access%20MCP%20servers,refer%20to%20the%20config%20docs). We can monitor the GitHub releases or the Changelog on OpenAI’s dev site. A simple approach is to subscribe to notifications or set up a small checker (even a GitHub Action that periodically checks the latest release tag of openai/codex). When a new version is out, we fetch the new binary and run our manifest script.

* **Claude Code:** Since it’s closed-source, releases might be announced via Anthropic’s channels or simply by a new NPM version. We can monitor the NPM package @anthropic-ai/claude-code for updates. Perhaps a weekly check of the package version or an RSS feed from Anthropic’s blog.

* **Gemini CLI:** As an open-source project (google-gemini/gemini-cli on GitHub), it has a rapid release cadence (stable weekly, previews, etc.)[\[27\]](https://github.com/google-gemini/gemini-cli#:~:text=npm%20install%20). We should pin to stable releases for our support. Monitoring can be via GitHub releases or even an npm check for @google/gemini-cli. Google’s documentation site or blog might also announce big changes (e.g., introduction of Gemini 3 Pro, etc.).

When a new version is detected: \- **Fetch and Test**: Download/install the new CLI (for Codex, get the binary from the GitHub release; for Claude/Gemini, update via npm) in a safe environment. \- **Run the Manifest Script**: Generate the new JSON/TOML manifest. \- **Diff the Manifest**: Compare with the previous version’s manifest to see what’s new or changed. For example, maybe a new flag \--experimental-feature appears under exec, or a new subcommand codex export shows up, etc. \- **Update Wrapper Accordingly**: \- If it’s a **new flag or subcommand that we want to support**, implement the corresponding Rust code (e.g. add a builder method for a new flag, or a new API function for a new command). Our manifest diff will highlight these needs. \- If a flag was removed or changed, adjust usage or deprecate our corresponding option if needed. \- If purely internal or not applicable changes occur (e.g., changes to interactive UI or something we deliberately don’t expose), we note them but might not need code changes. \- Importantly, we will maintain backward compatibility where possible. The wrapper can use **feature detection** at runtime to gracefully handle older binaries. In fact, our wrapper already does some of this: for instance, it runs codex \--version and codex features list to detect available capabilities and toggles features accordingly[\[28\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L56-L63). We should extend this concept to other tools too. For example, if the Codex CLI introduces a new required flag in version X, we might only add it if the detected version \>= X, etc.

* **Manifest versioning**: We might choose to keep only the latest manifest per tool in the repo for simplicity (updating it in-place). Alternatively, keeping historical files (like one per version) can be useful for diff history, but could clutter the repo. A compromise: keep a *single manifest* for each tool representing the latest supported version, but include metadata of the last update (e.g., “last updated for Codex CLI 0.70 on 2025-12-01”). Git history will anyway allow us to see what changed from the previous commit (which corresponds to the previous version). This way, our repo doesn’t bloat with many files per version, but we still get traceability.

## 4\. Workflow Automation (Local Script → CI Pipeline)

To start, we’ll implement this as a **local script or CLI tool** in our repo. This allows quick iteration and manual running. Once we’re confident, we can automate parts of it with CI:

* We can add a **GitHub Action** that triggers on a schedule (maybe daily or weekly) or on demand, which will run a lightweight job to check for new CLI versions. For example:

* The job could call an API or use npm view to get the latest version of @openai/codex and compare with a stored version file.

* If a new version is found, it could optionally open an issue or notify us. We might not want full auto-update without oversight (since introducing a new CLI version into our system might require testing).

* Another GH Action can be a manual dispatch (triggered by a button or command) that **runs the manifest generation** using the new version and perhaps even raises a PR with the updated manifest (and possibly stub code changes if we automate that).

* In the long run, we could integrate version bumps more seamlessly. For example, if Codex CLI releases 0.xy, an action could automatically update a constant or the manifest and run our test suite to see if everything passes. However, given the importance of testing with real AI interactions, an automated PR for review is safer than automated direct merges.

* We will ensure the CI environment has what it needs: Node and npm (for Claude/Gemini CLIs), ability to download binaries (for Codex), etc. Linux should be fine for running Codex and Gemini. For Claude on CI, since it’s Node-based, Linux is fine as well. We’ll need to use **WSL or a Windows runner if we ever test Windows-specific behavior**, but that can be a separate matrix in CI for later (Windows is experimental for us, and we prefer WSL which CI can’t easily emulate, so we might skip Windows automation for now).

* **Iteration speed:** By doing this first as a local script, you (the developer) can run it as soon as you hear of a new feature. This satisfies the near-term need for agility. Then the GH Action just serves as a safety net to catch anything we miss (or to satisfy curiosity on nightly builds of these CLIs, etc.).

## 5\. Supporting Multiple CLI Agents in Substrate

With Codex as the model, we’ll extend the same wrapper/orchestration approach to other agents like Claude Code and Gemini CLI. Key considerations for each:

* **Unified Interface vs. Separate Modules:** We likely will create separate Rust crates or modules for each tool’s wrapper (similar to how we have a codex crate now). This keeps the code clean and focused (since each tool has its quirks). Substrate can then orchestrate them through a common trait or abstraction (e.g., a trait for “AI Coding Agent” that CodexClient, ClaudeClient, GeminiClient all implement, providing methods like send\_prompt, resume\_last, etc. appropriate to that tool). Designing that abstraction is a future step, but having the manifest of capabilities will inform it. For instance, if only some agents support a “cloud exec” concept, or only some support a particular sandboxing feature, we might design the trait to accommodate optional support.

* **Differences in Auth & Context:**

* Codex uses ChatGPT auth (or API key) and stores state in \~/.codex. Claude Code uses an Anthropic API key (likely stored in its config dir), and Gemini CLI uses Google account OAuth (one logs in via browser for a token, stored in config). Each will need isolated “home” directories to avoid conflicts, similar to how we use CODEX\_HOME. We should prepare to have e.g. CLAUDE\_HOME or similar if supported, or otherwise manage their config locations. Our manifest can also track environment variables or config file locations that are important.

* From an update perspective, the manifest covers CLI commands, but we should also note how to detect login status or programmatically authenticate. For Codex, we have login status. For Claude, we might see if claude status exists or if it just fails when not logged in. For Gemini, documentation suggests it may prompt a browser login on first run; we might script around that using service accounts or pre-provisioned tokens in enterprise scenarios. In any event, these are mostly one-time setup per machine, and not a frequent update issue.

* **Cross-Platform Binaries:** We will deliver Substrate primarily on Linux (and Mac). We’ll obtain CLI binaries accordingly:

* Codex: distributed as tar.gz per platform on GitHub releases[\[29\]](https://github.com/openai/codex#:~:text=Each%20GitHub%20Release%20contains%20many,likely%20want%20one%20of%20these). We can automate downloading the Linux and macOS archives, extract the codex binary, and store it or reference it. (In development, using the user’s installed one via PATH or CODEX\_BINARY is fine; for a bundled distribution, we’d package a known version for each platform).

* Claude Code: distributed via npm (which delivers a packaged binary after install). We might use npm pack to fetch the tarball, and inside find the binary. (The reddit thread suggests the code is obfuscated, but a binary is there). Alternatively, Anthropic’s install script downloads a binary via curl[\[30\]](https://www.claude.com/product/claude-code#:~:text=,JetBrains). We could use their script to grab the binary directly for packaging.

* Gemini CLI: as an npm package (open-source), or even build from source. Since it’s open-source, we could compile it if needed, but using the npm distribution is easier. It likely just installs a Node-based CLI tool globally. Packaging it might mean including Node or packaging as a standalone binary via something like pkg/nexe if needed – but initially we can assume the user/developer will install it.

* **Version Pinning:** We will likely **pin specific versions** of each CLI in our Substrate releases for stability. The manifest helps here: for example, Substrate v1.0 might bundle Codex CLI 0.70, Claude CLI 1.2, Gemini CLI 0.3. If the user has a different version installed, our wrapper might still work (if backward-compatible), but ideally we use the pinned one for consistency. The manifest file doubles as documentation of what versions we officially support/tested.

* **Manifest for Each Tool:** We’ll maintain separate manifests (e.g., claude-cli.json, gemini-cli.json). Each will have its own structure reflecting that tool’s syntax. For instance, Claude might not call its main command exec – it could just run on claude \<prompt\> directly, or use different terminology. Gemini might have additional interactive commands. We’ll tailor the manifest to each but keep the general format similar so that it’s easy to scan and compare.

* The manifest can also include any **notable feature flags or limitations**. E.g., “Claude Code has a \--no-sandbox flag or requires \--accept-terms on first run” – such things can be noted.

* Including an **“experimental” or “beta” marker** for certain commands in the manifest could be useful. Codex’s manifest clearly labels some commands as experimental (cloud, some mcp items)[\[31\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L14). We can carry that info so we know which parts of the CLI might change or require caution in use.

## 6\. Example: New Release Integration Workflow

Let’s walk through how we’d handle a hypothetical new release for Codex and for another tool:

* **Codex CLI 0.62 released:** Suppose OpenAI releases v0.62 with a new subcommand codex plan (just as an example) and a new flag \--dry-run on codex apply. Once we get wind of this:

* We run our scrape\_cli\_help.py \--tool codex \--binary \~/downloads/codex-0.62 \--output codex-cli-v0.62.json.

* We diff codex-cli-v0.62.json against our current codex-cli.json (which was v0.61). The diff shows:

  * New top-level command plan with its flags.

  * New flag \--dry-run under apply.

* We update the wrapper code: implement a CodexClient::plan(...) if we think this command is useful to expose (or perhaps decide to hold off if it’s not relevant). Also add .dry\_run(true) option on the apply builder or incorporate it always if it fits.

* Run tests with the new binary to ensure everything works. Possibly update examples or docs.

* Commit the updated manifest (now reflecting v0.62) and code. The manifest in Git serves as evidence of exactly what CLI state we’ve targeted.

* **Claude Code update:** Suppose Anthropic updates Claude CLI adding a new “cloud” mode or similar. We detect via npm that claude-code@1.1.0 is out. We install it in a sandbox, run claude \--help and any subcommand helps. If Anthropic provides a reference doc, we compare our manifest against that doc for changes.

* Maybe they added a claude audit command or a flag \--unsafe to allow riskier changes. We record those in claude-cli.json.

* If relevant, implement or adjust our Claude wrapper (once we build it) accordingly.

* **Gemini CLI weekly update:** Gemini’s open source nature means frequent releases, but they might be minor. We’ll likely target specific stable milestones (e.g., Gemini CLI 1.0). Still, we keep an eye. If they add, say, a new sub-tool integration or a gemini export-session command, we’d update the manifest and consider exposing it.

One nice side-effect: having a manifest and automated diff means even if we **don’t immediately implement** a new CLI feature, we *know about it*. This is important for planning. For instance, if Codex introduces “cloud exec” as stable, our manifest will show it. We might decide to implement support or deliberately ignore it (but at least that decision is conscious). The manifest diff can also alert us to deprecations (if something disappears from \--help, that’s a hint it was removed or renamed).

## 7\. Long-Term Maintenance and Consistency

To ensure our approach remains efficient in the long run:

* We’ll integrate a step in our development cycle or release checklist: “Update CLI manifests and confirm wrapper parity.” This makes sure that before we cut a new release of Substrate, we’ve synced up with the latest CLI versions we intend to support.

* The manifests can also serve as a **registry of capabilities** for the Substrate orchestration logic. For example, Substrate could read the manifest at runtime to decide how to use an agent. However, since our wrappers already encapsulate capabilities in code, we may not need to dynamically read the manifest – it’s more of a development aid. Still, conceivably, if one wanted to list “which agents support a ‘sandbox’ feature,” one could consult the manifests for a flag or command named sandbox.

* We must remain **mindful of breaking changes** in these tools. If a CLI tool drastically changes (e.g., Anthropic releases “Claude Code v2” with completely different CLI), our manifest-and-diff approach will catch that, but the wrapper might require a more significant rework. In such cases, we might version our wrappers (for instance, a ClaudeClientV1 vs ClaudeClientV2 if needed). Hopefully, these tools remain backward compatible in their syntax as they evolve.

* **Community and Documentation:** Because these CLI tools are frontier tech, community forums and official docs are valuable. We should keep an eye on those as well. Sometimes the \--help might not tell the whole story (e.g. hidden flags or environment variables). For Codex, we have the OpenAI developer reference which catalogs all commands/flags in detail[\[32\]](https://developers.openai.com/codex/cli/reference/#:~:text=Codex%20CLI%20reference)[\[33\]](https://developers.openai.com/codex/cli/reference/#:~:text=codex%20resume) – our manifest is essentially mirroring that reference. For other tools, we’ll incorporate information from their docs (for example, Google’s Gemini CLI docs on “Commands” and “Headless mode” give insight into usage patterns[\[34\]](https://geminicli.com/docs/#:~:text=Section%20titled%20%E2%80%9CCLI%E2%80%9D)).

## 8\. Considering Linux, macOS, and Windows Support

You mentioned that **Linux is first-class, macOS second, and Windows (via PowerShell or preferably WSL) is experimental**. Our strategy accommodates this as follows:

* Our manifest generation and testing will be done primarily on Linux (and possibly macOS). The CLI surfaces on Linux vs macOS should be identical in terms of commands and flags. We’ll be careful if any platform-specific differences exist (e.g., Codex sandbox has a macOS-only flag \--log-denials, which we note and include with a platform tag in the manifest[\[6\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L72-L75)).

* For Windows, since we prefer WSL, users would essentially use the Linux binaries under WSL. This simplifies things (we treat it like Linux environment). If we ever attempt native Windows support, we’d have to incorporate Windows binary paths and potentially different help output formatting. (For instance, codex.exe \--help should be the same, but line endings or path examples might differ. Not a big issue for parsing.)

* The update pipeline doesn’t need to run on Windows at all – we can do everything in Linux CI containers. When packaging, we just ensure to include the Windows binary for Codex (they do provide one) and any Windows specifics. But given experimental status, we can defer deep Windows integration until more stable.

## 9\. Extending to New CLI Tools Quickly

When a **new CLI-based agent emerges** (you mentioned possibly others cropping up), we can onboard it quickly by following the same pattern:

1. **Obtain the CLI** (install or download).

2. **Run \--help to gather commands/flags**, create an initial manifest.

3. **Design a minimal wrapper** that covers core use-cases (very likely “execute a prompt and get output” is the primary one, plus any unique features of that agent).

4. **Iterate** to add more coverage as needed.

By having the manifest and a structured way to compare it to our existing wrappers, we reduce the chance of forgetting a flag or misnaming something. It essentially serves as a checklist for implementing the wrapper.

For example, if tomorrow there’s a “XYZ CLI” for coding: \- After manifesting it, we see it has commands A, B, C. We implement those in an XyzClient. \- If some commands overlap conceptually with Codex/Claude/Gemini, we might use similar naming in our API for consistency. \- If it has a novel concept (say a xyz review command that others don’t), we implement it in that wrapper and perhaps later consider if a generalized interface can include that concept.

## 10\. Conclusion and Next Steps

To summarize, **Codex’s wrapper is nearly feature-complete** for the current CLI version, with only experimental features left out. We will create a **JSON/TOML manifest** to catalog Codex CLI commands and flags (we effectively already have this in CLI\_MATRIX.md[\[31\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L14)[\[10\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L22-L31) – we’ll transcribe it into machine-readable form). Then, using scripting and automation, we’ll keep this manifest (and our code) up-to-date with each new release. We’ll apply the same strategy to upcoming integrations like Claude Code and Gemini CLI, acknowledging the differences (Claude is proprietary, Gemini is open-source, etc.). The manifest-driven approach, combined with runtime feature detection and careful version pinning, will ensure Substrate’s orchestration layer can **leverage the best capabilities of each agent** while remaining robust to changes.

By investing in this systematic pipeline now, we’ll save time and avoid bugs in the future – when OpenAI or others push an update, we can confidently say “we have a process for that.” This will keep Substrate’s agent orchestration on the cutting edge, without constantly playing catch-up or risking broken functionality due to a missed flag. In short, we’ll **leverage** the rapid improvements of tools like Codex, Claude, Gemini rather than fight them, which aligns perfectly with our philosophy of orchestrating the best available AI engines instead of reinventing them.

---

**Sources:**

* Codex CLI commands, flags, and wrapper parity notes[\[31\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L14)[\[35\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L66-L73) (from our CLI\_MATRIX.md in the repo).

* Prompt Security’s comparison of Claude Code vs Gemini CLI (noting Claude is closed-source and Gemini open-source)[\[21\]](https://prompt.security/blog/ai-coding-assistants-make-a-cli-comeback#:~:text=,focuses%20on%20vetted%20enterprise%20integration).

* OpenAI Codex CLI documentation and README (for installation and usage context)[\[25\]](https://github.com/openai/codex#:~:text=If%20you%27re%20running%20into%20upgrade,entry%20on%20brew%20upgrade%20codex)[\[26\]](https://github.com/openai/codex#:~:text=Codex%20can%20access%20MCP%20servers,refer%20to%20the%20config%20docs).

---

[\[1\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L13) [\[2\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L70-L73) [\[6\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L72-L75) [\[7\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L73-L75) [\[10\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L22-L31) [\[11\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L34-L42) [\[12\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L24-L31) [\[13\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L66-L69) [\[17\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L7-L14) [\[18\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L1-L9) [\[19\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L53-L62) [\[20\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L12-L14) [\[31\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L6-L14) [\[35\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md#L66-L73) CLI\_MATRIX.md

[https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI\_MATRIX.md](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/CLI_MATRIX.md)

[\[3\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L61-L69) [\[14\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L62-L69) [\[16\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md#L61-L68) crates/codex/README.md

[https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/README.md)

[\[4\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1899-L1907) [\[5\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1942-L1950) [\[8\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1372-L1381) [\[9\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L1388-L1396) [\[15\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L26-L34) [\[28\]](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs#L56-L63) lib.rs

[https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs](https://github.com/atomize-hq/codex-wrapper/blob/caf991ae4fbd8b46953b5825f2b749f63b4272cd/crates/codex/src/lib.rs)

[\[21\]](https://prompt.security/blog/ai-coding-assistants-make-a-cli-comeback#:~:text=,focuses%20on%20vetted%20enterprise%20integration) AI Coding Assistants for Terminal: Claude Code, Gemini CLI & Qodo Compared

[https://prompt.security/blog/ai-coding-assistants-make-a-cli-comeback](https://prompt.security/blog/ai-coding-assistants-make-a-cli-comeback)

[\[22\]](https://geminicli.com/docs/#:~:text=,Themes) [\[23\]](https://geminicli.com/docs/#:~:text=,all%20keyboard%20shortcuts%20to%20improve) [\[24\]](https://geminicli.com/docs/#:~:text=,Settings) [\[34\]](https://geminicli.com/docs/#:~:text=Section%20titled%20%E2%80%9CCLI%E2%80%9D) Welcome to Gemini CLI documentation | Gemini CLI

[https://geminicli.com/docs/](https://geminicli.com/docs/)

[\[25\]](https://github.com/openai/codex#:~:text=If%20you%27re%20running%20into%20upgrade,entry%20on%20brew%20upgrade%20codex) [\[26\]](https://github.com/openai/codex#:~:text=Codex%20can%20access%20MCP%20servers,refer%20to%20the%20config%20docs) [\[29\]](https://github.com/openai/codex#:~:text=Each%20GitHub%20Release%20contains%20many,likely%20want%20one%20of%20these) GitHub \- openai/codex: Lightweight coding agent that runs in your terminal

[https://github.com/openai/codex](https://github.com/openai/codex)

[\[27\]](https://github.com/google-gemini/gemini-cli#:~:text=npm%20install%20) GitHub \- google-gemini/gemini-cli: An open-source AI agent that brings the power of Gemini directly into your terminal.

[https://github.com/google-gemini/gemini-cli](https://github.com/google-gemini/gemini-cli)

[\[30\]](https://www.claude.com/product/claude-code#:~:text=,JetBrains) Claude Code | Claude

[https://www.claude.com/product/claude-code](https://www.claude.com/product/claude-code)

[\[32\]](https://developers.openai.com/codex/cli/reference/#:~:text=Codex%20CLI%20reference) [\[33\]](https://developers.openai.com/codex/cli/reference/#:~:text=codex%20resume) Codex CLI reference

[https://developers.openai.com/codex/cli/reference/](https://developers.openai.com/codex/cli/reference/)
