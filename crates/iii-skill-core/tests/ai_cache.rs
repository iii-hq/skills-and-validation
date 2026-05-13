use iii_skill_core::ai_cache::{cache_key, PassCache};
use iii_skill_core::docs::frontmatter::DocType;

#[test]
fn cache_key_is_deterministic_for_same_inputs() {
    let a = cache_key("artifact", "rules", "system", "claude-opus-4-7", None);
    let b = cache_key("artifact", "rules", "system", "claude-opus-4-7", None);
    assert_eq!(a, b);
    assert_eq!(a.len(), 64, "expected 64-char SHA-256 hex");
}

#[test]
fn cache_key_changes_when_artifact_changes() {
    let a = cache_key("artifact A", "rules", "system", "claude-opus-4-7", None);
    let b = cache_key("artifact B", "rules", "system", "claude-opus-4-7", None);
    assert_ne!(a, b);
}

#[test]
fn cache_key_changes_when_rules_change() {
    // This is the critical invariant: a rule edit (like the `carved out`
    // addition in v0.2.14) must bust every cached entry. If it didn't,
    // stale PASSes from before the edit would survive a real regression.
    let a = cache_key("artifact", "old rules", "system", "claude-opus-4-7", None);
    let b = cache_key("artifact", "new rules", "system", "claude-opus-4-7", None);
    assert_ne!(a, b);
}

#[test]
fn cache_key_changes_when_system_prompt_changes() {
    let a = cache_key("artifact", "rules", "old prompt", "claude-opus-4-7", None);
    let b = cache_key("artifact", "rules", "new prompt", "claude-opus-4-7", None);
    assert_ne!(a, b);
}

#[test]
fn cache_key_changes_when_model_changes() {
    let a = cache_key("artifact", "rules", "system", "claude-opus-4-7", None);
    let b = cache_key("artifact", "rules", "system", "claude-sonnet-4-6", None);
    assert_ne!(a, b);
}

#[test]
fn cache_key_changes_when_doc_type_changes() {
    let none = cache_key("artifact", "rules", "system", "m", None);
    let howto = cache_key("artifact", "rules", "system", "m", Some(DocType::HowTo));
    let tutorial = cache_key("artifact", "rules", "system", "m", Some(DocType::Tutorial));
    assert_ne!(none, howto);
    assert_ne!(howto, tutorial);
}

#[test]
fn cache_key_is_not_vulnerable_to_field_boundary_collisions() {
    // Without separators, ("ab", "cd") and ("a", "bcd") would hash the
    // same because they produce the same concatenated bytes. NUL
    // separators in cache_key prevent that.
    let a = cache_key("ab", "cd", "system", "m", None);
    let b = cache_key("a", "bcd", "system", "m", None);
    assert_ne!(a, b);
}

#[test]
fn passcache_load_missing_file_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("does-not-exist");
    let cache = PassCache::load(&path).unwrap();
    assert!(!cache.contains("anything"));
}

#[test]
fn passcache_record_and_contains_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache");
    let mut cache = PassCache::load(&path).unwrap();
    assert!(!cache.contains("abc123"));
    cache.record("abc123").unwrap();
    assert!(cache.contains("abc123"));
}

#[test]
fn passcache_persists_to_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache");
    {
        let mut cache = PassCache::load(&path).unwrap();
        cache.record("hash1").unwrap();
        cache.record("hash2").unwrap();
    }
    let reopened = PassCache::load(&path).unwrap();
    assert!(reopened.contains("hash1"));
    assert!(reopened.contains("hash2"));
    assert!(!reopened.contains("hash3"));
}

#[test]
fn passcache_record_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache");
    let mut cache = PassCache::load(&path).unwrap();
    cache.record("same").unwrap();
    cache.record("same").unwrap();
    cache.record("same").unwrap();
    // Reopen and confirm only one entry persisted (idempotent record
    // avoids unbounded file growth on repeated runs hitting the same
    // unchanged artifact).
    drop(cache);
    let contents = std::fs::read_to_string(&path).unwrap();
    let count = contents.lines().filter(|l| l.trim() == "same").count();
    assert_eq!(count, 1);
}

#[test]
fn passcache_skips_comments_and_blanks() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache");
    std::fs::write(
        &path,
        "# header comment\nhash-a\n\nhash-b\n  # indented comment\n",
    )
    .unwrap();
    let cache = PassCache::load(&path).unwrap();
    assert!(cache.contains("hash-a"));
    assert!(cache.contains("hash-b"));
    assert!(!cache.contains("# header comment"));
    assert!(!cache.contains(""));
}
