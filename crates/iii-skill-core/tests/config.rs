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
    // `version` is the schema version of .skill-check.yaml itself — present
    // and integer-typed (Rust's u32 enforces the integer-ness at parse time).
    assert!(config.version > 0);
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
        "version: 1\nrules:\n  path: ./local-rules\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let config = iii_skill_core::config::load(tmp.path()).expect("config should load");
    assert_eq!(
        config.rules.as_ref().unwrap().path,
        std::path::PathBuf::from("./local-rules")
    );
    assert_eq!(config.version, 1);
}

#[test]
fn errors_when_required_fields_are_missing() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    // Missing both `version` and `ai_check`.
    std::fs::write(tmp.path(), "rules:\n  path: ./x\n").unwrap();
    let result = iii_skill_core::config::load(tmp.path());
    assert!(result.is_err(), "expected an error when required fields are missing");
}

#[test]
fn errors_when_version_is_not_an_integer() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "version: \"0.1.0\"\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let result = iii_skill_core::config::load(tmp.path());
    assert!(result.is_err(), "expected an error when version is non-integer");
}

#[test]
fn v1_implicitly_resolves_to_worker_mode() {
    use iii_skill_core::config::Mode;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "version: 1\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let config = iii_skill_core::config::load(tmp.path()).expect("v1 config should load");
    assert_eq!(config.resolved_mode(), Mode::Worker);
    assert!(config.mode.is_none(), "v1 leaves the `mode` field unset");
}

#[test]
fn v2_requires_explicit_mode() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "version: 2\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let result = iii_skill_core::config::load(tmp.path());
    assert!(
        result.is_err(),
        "v2 without `mode:` should fail; got: {:?}",
        result.map(|_| "ok")
    );
}

#[test]
fn v2_docs_mode_loads_with_globs() {
    use iii_skill_core::config::Mode;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "version: 2\nmode: docs\ndocs:\n  include:\n    - \"**/*.mdx\"\n  exclude:\n    - \"**/CHANGELOG.md\"\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let config = iii_skill_core::config::load(tmp.path()).expect("v2 docs mode should load");
    assert_eq!(config.resolved_mode(), Mode::Docs);
    let docs = config.docs.as_ref().expect("docs block present");
    assert_eq!(docs.include, vec!["**/*.mdx".to_string()]);
    assert_eq!(docs.exclude, vec!["**/CHANGELOG.md".to_string()]);
}

#[test]
fn v2_docs_mode_without_docs_block_errors() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "version: 2\nmode: docs\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let result = iii_skill_core::config::load(tmp.path());
    assert!(
        result.is_err(),
        "docs mode without docs:include should fail"
    );
}

#[test]
fn v2_worker_mode_loads_without_docs_block() {
    use iii_skill_core::config::Mode;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "version: 2\nmode: worker\nai_check:\n  provider: anthropic\n  model: m\n  api_key_env_var: K\n  max_tokens: 100\n",
    )
    .unwrap();
    let config = iii_skill_core::config::load(tmp.path()).expect("v2 worker mode should load");
    assert_eq!(config.resolved_mode(), Mode::Worker);
    assert!(config.docs.is_none());
}
