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
    let (prefix, suffix) = split_user_prompt(rules, artifact_path, artifact_text);
    format!("{prefix}{suffix}")
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
    let (prefix, suffix) =
        split_user_prompt_with_type(rules, doc_type, artifact_path, artifact_text);
    format!("{prefix}{suffix}")
}

// Split the user prompt into a cacheable prefix (rules — identical across
// every artifact in a run) and a per-artifact suffix. `call_anthropic`
// places a `cache_control` breakpoint at the end of the prefix so the
// second artifact onward gets a discount on the shared rules block.
fn split_user_prompt(rules: &str, artifact_path: &str, artifact_text: &str) -> (String, String) {
    let prefix = format!("## Project rules\n\n{rules}\n\n", rules = rules.trim());
    let suffix = format!(
        "## Artifact under review\n\nPath: `{path}`\n\n{numbered}\n",
        path = artifact_path,
        numbered = line_numbered(artifact_text),
    );
    (prefix, suffix)
}

// Same as [`split_user_prompt`] but folds the Diataxis-type hint into the
// cacheable prefix. Doc-type varies per artifact within a docs run, so this
// produces one cache entry per (rules, doc_type) pair (at most 4 — tutorial,
// how-to, reference, explanation). Mixed runs still hit cache for every
// non-first artifact of each type.
fn split_user_prompt_with_type(
    rules: &str,
    doc_type: crate::docs::frontmatter::DocType,
    artifact_path: &str,
    artifact_text: &str,
) -> (String, String) {
    let prefix = format!(
        "## Project rules\n\n{rules}\n\n## Artifact type\n\nThis artifact is a Diataxis **{ty}** doc. Apply the rules that match that category and don't flag patterns that are appropriate for it (e.g. tutorial-style \"you'll do X next\" framing is *expected* in tutorials).\n\n",
        rules = rules.trim(),
        ty = doc_type,
    );
    let suffix = format!(
        "## Artifact under review\n\nPath: `{path}`\n\n{numbered}\n",
        path = artifact_path,
        numbered = line_numbered(artifact_text),
    );
    (prefix, suffix)
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
    let (prefix, suffix) = split_user_prompt(rules, &path_str, artifact_text);
    call_anthropic(
        system_prompt,
        &prefix,
        &suffix,
        model,
        api_key_env_var,
        max_tokens,
    )
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
    let (prefix, suffix) =
        split_user_prompt_with_type(rules, doc_type, &path_str, artifact_text);
    call_anthropic(
        system_prompt,
        &prefix,
        &suffix,
        model,
        api_key_env_var,
        max_tokens,
    )
}

// `cached_user_prefix` and `uncached_user_suffix` are concatenated to form
// the single user message; the split exists so a `cache_control` breakpoint
// can sit at the end of the prefix. Combined with the breakpoint on the
// system block, the model server reuses the system + rules portion across
// every artifact in a run (ephemeral cache TTL ~5 min). First call pays
// the cache-write premium; calls 2…N read the cache at the discounted rate.
fn call_anthropic(
    system_prompt: &str,
    cached_user_prefix: &str,
    uncached_user_suffix: &str,
    model: &str,
    api_key_env_var: &str,
    max_tokens: u32,
) -> anyhow::Result<Result<(), String>> {
    let api_key = std::env::var(api_key_env_var)
        .with_context(|| format!("env var {api_key_env_var} not set"))?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": [{
            "type": "text",
            "text": system_prompt,
            "cache_control": { "type": "ephemeral" }
        }],
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": cached_user_prefix,
                    "cache_control": { "type": "ephemeral" }
                },
                {
                    "type": "text",
                    "text": uncached_user_suffix
                }
            ]
        }],
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
