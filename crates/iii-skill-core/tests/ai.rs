use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

fn load_rules() -> String {
    let rules_dir = repo_root().join("content/project-rules");
    let mut rules = String::new();
    let mut entries: Vec<_> = std::fs::read_dir(&rules_dir)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !name.ends_with(".md")
            || name.starts_with('_')
            || name == "README.md"
            || name == "SOURCE.md"
        {
            continue;
        }
        let body = std::fs::read_to_string(&path).unwrap();
        rules.push_str(&format!("# {name}\n\n{body}\n\n"));
    }
    rules
}

fn load_prompt() -> String {
    let p = repo_root()
        .join("content/project-rules")
        .join("_skill-check-prompt.md");
    std::fs::read_to_string(&p).unwrap()
}

/// Print the model's response on both PASS and FAIL so flaky-result triage
/// has the actual text to look at. Use `cargo test -- --show-output` to see
/// it on passing tests; failing tests print captured output automatically.
fn print_ai_response(label: &str, result: &Result<(), String>) {
    eprintln!();
    eprintln!("=== [{label}] AI response ===");
    match result {
        Ok(()) => eprintln!("PASS"),
        Err(body) => eprintln!("{body}"),
    }
    eprintln!("=== end ===");
    eprintln!();
}

#[test]
fn build_user_prompt_includes_rules_and_artifact() {
    let prompt = iii_skill_core::ai::build_user_prompt(
        "Voice: be terse.\n",
        "README.md",
        "# Hello\nworld\n",
    );
    assert!(prompt.contains("Voice: be terse."), "rules absent: {prompt}");
    assert!(prompt.contains("README.md"), "path absent: {prompt}");
    assert!(prompt.contains("# Hello"), "artifact absent: {prompt}");
}

#[test]
fn build_user_prompt_line_numbers_the_artifact() {
    let prompt = iii_skill_core::ai::build_user_prompt(
        "rules",
        "x.md",
        "alpha\nbeta\ngamma\n",
    );
    // Each artifact line should be prefixed with its 1-based line number.
    assert!(prompt.contains("1") && prompt.contains("alpha"));
    assert!(prompt.contains("2") && prompt.contains("beta"));
    assert!(prompt.contains("3") && prompt.contains("gamma"));
}

#[test]
fn parse_response_returns_ok_on_pass() {
    assert!(iii_skill_core::ai::parse_response("PASS").is_ok());
    assert!(iii_skill_core::ai::parse_response("  PASS\n").is_ok());
}

#[test]
fn parse_response_returns_err_on_fail() {
    let r = iii_skill_core::ai::parse_response("FAIL\nREADME.md:5 — voice drift — rephrase");
    assert!(r.is_err());
    let body = r.unwrap_err();
    assert!(body.contains("voice drift"));
    assert!(body.contains("README.md:5"));
}

#[test]
fn parse_response_returns_err_on_anything_other_than_pass() {
    assert!(iii_skill_core::ai::parse_response("").is_err());
    assert!(iii_skill_core::ai::parse_response("Pass.").is_err());
    assert!(iii_skill_core::ai::parse_response("Looks good!").is_err());
}

/// Live API: the example-worker README should pass the AI layer cleanly.
/// Prints the model response either way so a stochastic mis-flag is easy
/// to inspect.
#[test]
fn ai_check_passes_example_readme_when_key_present() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("skipping ai_check_passes_example_readme: ANTHROPIC_API_KEY not set");
        return;
    }

    let example_readme = repo_root().join("fixtures/example-worker").join("README.md");
    let result = iii_skill_core::ai::check_artifact(
        &example_readme,
        &load_rules(),
        &load_prompt(),
        "claude-opus-4-7",
        "ANTHROPIC_API_KEY",
        4000,
    )
    .expect("API call should not error");

    print_ai_response("passes_example_readme", &result);
    assert!(
        result.is_ok(),
        "expected PASS for example-worker README; see printed response above"
    );
}

/// Live API: a worker README full of marketing fluff, tutorial-speak, and
/// hedging should be flagged by the AI layer. Pairs with the PASS test so
/// we know the layer can fail loudly when content actually drifts — not
/// just rubber-stamp anything that lands in front of it.
#[test]
fn ai_check_fails_marketing_fluff_when_key_present() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("skipping ai_check_fails_marketing_fluff: ANTHROPIC_API_KEY not set");
        return;
    }

    // Multiple voice violations from project-rules/voice.md:
    //   - tutorial-speak: "Welcome!", "Let's get started!"
    //   - marketing fluff: "revolutionary", "blazing fast", "the best",
    //     "powerful", "magical", "seamless"
    //   - hedging: "you might want to consider"
    //   - exclamation salad
    let bad_readme = "\
# textstats

Welcome! You're going to love this revolutionary, blazing fast worker — \
the best text analysis tool you will ever use. Let's get started!

This powerful, magical worker offers seamless integration and is \
incredibly easy to use. You might want to consider it for all your \
text analysis needs!

## Install

It is super simple — just run:

```bash
iii worker add textstats
```

That is all there is to it!
";

    // Write to a tempdir + README.md so the path the model sees ends in
    // README.md — the system prompt uses the filename to decide which
    // artifact-type rules to apply.
    let tmp = tempfile::tempdir().unwrap();
    let readme_path = tmp.path().join("README.md");
    std::fs::write(&readme_path, bad_readme).unwrap();

    let result = iii_skill_core::ai::check_artifact(
        &readme_path,
        &load_rules(),
        &load_prompt(),
        "claude-opus-4-7",
        "ANTHROPIC_API_KEY",
        4000,
    )
    .expect("API call should not error");

    print_ai_response("fails_marketing_fluff", &result);
    assert!(
        result.is_err(),
        "expected FAIL for an obviously-fluffy README; see printed response above"
    );
}
