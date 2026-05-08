use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

#[test]
fn loads_template_skill_check_yaml() {
    let path = repo_root().join("templates/.skill-check.yaml");
    let config = iii_skill_core::config::load(&path).expect("config should load");
    assert_eq!(config.ai_check.provider, "anthropic");
    assert_eq!(config.ai_check.model, "claude-opus-4-7");
    assert_eq!(config.ai_check.api_key_env_var, "ANTHROPIC_API_KEY");
    assert!(config.ai_check.max_tokens < 10000);
    assert_eq!(config.version.as_deref(), Some("0.1.0"));
    // Template omits rules/styles by design — consumers should let the
    // bundle defaults apply unless they want a local override.
    assert!(config.rules.is_none());
    assert!(config.styles.is_none());
}

#[test]
fn loads_config_with_explicit_rules_path() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "rules:\n  path: ./local-rules\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let config = iii_skill_core::config::load(tmp.path()).expect("config should load");
    assert_eq!(
        config.rules.as_ref().unwrap().path,
        std::path::PathBuf::from("./local-rules")
    );
}

#[test]
fn errors_when_yaml_is_missing_required_fields() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "rules:\n  path: ./x\n").unwrap();
    let result = iii_skill_core::config::load(tmp.path());
    assert!(result.is_err(), "expected an error when ai_check is missing");
}
