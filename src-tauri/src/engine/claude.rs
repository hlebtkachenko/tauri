// claude.rs — the single place that drives the local `claude` CLI on the user's OAuth
// SUBSCRIPTION (no paid API key). One-shot prompt, JSON-mode result:
//   claude -p <prompt> --system-prompt <sys> --output-format json
//          --strict-mcp-config --setting-sources project --model <m>
//
// Why this exact shape (advisor-verified, ported from the TS reference):
//  - FORCED SUBSCRIPTION: strip ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN so the child can
//    only authenticate via the OAuth session — never a paid API key. If
//    CLAUDE_CODE_OAUTH_TOKEN is set in our env, inject it.
//  - `--strict-mcp-config`: load NO MCP servers (otherwise the CLI starts every globally
//    configured MCP server per spawn and hangs).
//  - `--setting-sources project` + a clean temp cwd: do not auto-load the user's
//    CLAUDE.md / hooks.
//  - `--output-format json`: one JSON object with result / is_error / subtype /
//    total_cost_usd.
//  - timeout: a hung child can never freeze a regulated answer — on timeout we error out
//    and the caller abstains (fail-closed).
//
// Fail-closed: EVERY failure path returns Err. No unwrap/expect on this path.

use std::time::Duration;

use tokio::process::Command;

use super::types::EngineError;

/// Absolute path to the local `claude` binary (the OAuth-subscription CLI).
const CLAUDE_BIN: &str = "/Users/hleb/.local/bin/claude";
const TIMEOUT_SECS: u64 = 90;

#[derive(Debug, Clone)]
pub struct ClaudeJson {
    /// The model's text output.
    pub result: String,
    pub is_error: bool,
    pub cost_usd: f64,
}

/// Run one `claude` JSON-mode call. Returns Err on spawn failure, timeout, non-zero exit,
/// or unparseable stdout. The caller maps Err → abstain.
pub async fn claude_json(
    prompt: &str,
    system: &str,
    model: &str,
) -> Result<ClaudeJson, EngineError> {
    // A fresh empty cwd so no project CLAUDE.md / hooks load.
    let cwd = std::env::temp_dir();

    let mut cmd = Command::new(CLAUDE_BIN);
    cmd.args([
        "-p",
        prompt,
        "--system-prompt",
        system,
        "--output-format",
        "json",
        "--strict-mcp-config",
        "--setting-sources",
        "project",
        "--model",
        model,
    ]);
    cmd.current_dir(cwd);

    // Force subscription auth: remove any API-key env, re-inject the OAuth token if present.
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.env_remove("ANTHROPIC_AUTH_TOKEN");
    if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
        cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
    }

    let fut = cmd.output();
    let output = match tokio::time::timeout(Duration::from_secs(TIMEOUT_SECS), fut).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(EngineError::Claude(format!("spawn failed: {e}"))),
        Err(_) => {
            return Err(EngineError::Claude(format!(
                "timeout after {TIMEOUT_SECS}s"
            )))
        }
    };

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        return Err(EngineError::Claude(format!(
            "claude exit {code}: {}",
            err.trim().chars().take(200).collect::<String>()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| EngineError::Claude(format!("bad json: {e}")))?;

    let result = parsed
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let subtype_not_success = parsed
        .get("subtype")
        .and_then(|v| v.as_str())
        .map(|s| s != "success")
        .unwrap_or(false);
    let is_error = parsed
        .get("is_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || subtype_not_success;
    let cost_usd = parsed
        .get("total_cost_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    Ok(ClaudeJson {
        result,
        is_error,
        cost_usd,
    })
}
