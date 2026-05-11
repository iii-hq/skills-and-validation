use anyhow::Context;
use std::path::Path;

/// Format the rules + artifact for sending as the user message in the AI check.
///
/// Layout:
/// ```text
/// ## Project rules
///
/// {rules trimmed}
///
/// ## Artifact under review
///
/// Path: `{artifact_path}`
///
///    1: <line 1>
///    2: <line 2>
///    ...
/// ```
pub fn build_user_prompt(rules: &str, artifact_path: &str, artifact_text: &str) -> String {
    format!(
        "## Project rules\n\n{rules}\n\n## Artifact under review\n\nPath: `{path}`\n\n{numbered}\n",
        rules = rules.trim(),
        path = artifact_path,
        numbered = line_numbered(artifact_text),
    )
}

/// Same as [`build_user_prompt`] but injects a Diataxis-type hint between
/// the rules and the artifact. The hint tells the model which part of the
/// rule set actually applies to this artifact, since docs mode mixes
/// tutorial / how-to / reference / explanation pages whose voice
/// expectations differ.
pub fn build_user_prompt_with_type(
    rules: &str,
    doc_type: crate::docs::frontmatter::DocType,
    artifact_path: &str,
    artifact_text: &str,
) -> String {
    format!(
        "## Project rules\n\n{rules}\n\n## Artifact type\n\nThis artifact is a Diataxis **{ty}** doc. Apply the rules that match that category and don't flag patterns that are appropriate for it (e.g. tutorial-style \"you'll do X next\" framing is *expected* in tutorials).\n\n## Artifact under review\n\nPath: `{path}`\n\n{numbered}\n",
        rules = rules.trim(),
        ty = doc_type,
        path = artifact_path,
        numbered = line_numbered(artifact_text),
    )
}

/// Parse the model's response. `Ok(())` if the model returned exactly `PASS`
/// (whitespace tolerated). `Err(body)` for any other content; the body is
/// surfaced verbatim to the caller for display.
pub fn parse_response(text: &str) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed == "PASS" {
        Ok(())
    } else {
        Err(trimmed.to_string())
    }
}

/// One Anthropic Messages API call against one rendered artifact. Returns
/// `Ok(Ok(()))` on PASS, `Ok(Err(violation_block))` on FAIL, `Err` on transport
/// or API failure.
pub fn check_artifact(
    artifact: &Path,
    rules: &str,
    system_prompt: &str,
    model: &str,
    api_key_env_var: &str,
    max_tokens: u32,
) -> anyhow::Result<Result<(), String>> {
    let artifact_text = std::fs::read_to_string(artifact)
        .with_context(|| format!("reading {}", artifact.display()))?;
    check_artifact_text(
        &artifact_text,
        artifact,
        rules,
        system_prompt,
        model,
        api_key_env_var,
        max_tokens,
    )
}

/// Same as [`check_artifact`] but takes the artifact text in memory.
/// Used when `verify` renders the skill artifact lazily and doesn't write
/// it to disk first. `display_path` is the canonical path shown in the
/// prompt so the model sees the artifact's real location, not a temp dir.
pub fn check_artifact_text(
    artifact_text: &str,
    display_path: &Path,
    rules: &str,
    system_prompt: &str,
    model: &str,
    api_key_env_var: &str,
    max_tokens: u32,
) -> anyhow::Result<Result<(), String>> {
    let path_str = display_path.display().to_string();
    let user_prompt = build_user_prompt(rules, &path_str, artifact_text);
    call_anthropic(system_prompt, &user_prompt, model, api_key_env_var, max_tokens)
}

/// Same as [`check_artifact`] but augments the user prompt with the
/// artifact's Diataxis type. Use in docs mode so the model applies the
/// right voice rules per artifact.
pub fn check_artifact_with_type(
    artifact: &Path,
    rules: &str,
    system_prompt: &str,
    doc_type: crate::docs::frontmatter::DocType,
    model: &str,
    api_key_env_var: &str,
    max_tokens: u32,
) -> anyhow::Result<Result<(), String>> {
    let artifact_text = std::fs::read_to_string(artifact)
        .with_context(|| format!("reading {}", artifact.display()))?;
    check_artifact_text_with_type(
        &artifact_text,
        artifact,
        rules,
        system_prompt,
        doc_type,
        model,
        api_key_env_var,
        max_tokens,
    )
}

/// In-memory variant of [`check_artifact_with_type`]. See
/// [`check_artifact_text`] for the rationale.
pub fn check_artifact_text_with_type(
    artifact_text: &str,
    display_path: &Path,
    rules: &str,
    system_prompt: &str,
    doc_type: crate::docs::frontmatter::DocType,
    model: &str,
    api_key_env_var: &str,
    max_tokens: u32,
) -> anyhow::Result<Result<(), String>> {
    let path_str = display_path.display().to_string();
    let user_prompt = build_user_prompt_with_type(rules, doc_type, &path_str, artifact_text);
    call_anthropic(system_prompt, &user_prompt, model, api_key_env_var, max_tokens)
}

fn call_anthropic(
    system_prompt: &str,
    user_prompt: &str,
    model: &str,
    api_key_env_var: &str,
    max_tokens: u32,
) -> anyhow::Result<Result<(), String>> {
    let api_key = std::env::var(api_key_env_var)
        .with_context(|| format!("env var {api_key_env_var} not set"))?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system_prompt,
        "messages": [{ "role": "user", "content": user_prompt }],
    });

    let resp = reqwest::blocking::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .with_context(|| "POST https://api.anthropic.com/v1/messages")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().unwrap_or_default();
        anyhow::bail!("Anthropic API returned {status}: {body_text}");
    }

    let json: serde_json::Value = resp
        .json()
        .with_context(|| "parsing Anthropic API response JSON")?;
    let text = json
        .pointer("/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("unexpected response shape: {json}"))?;

    Ok(parse_response(text))
}

fn line_numbered(content: &str) -> String {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| format!("{:>4}: {}", i + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}
