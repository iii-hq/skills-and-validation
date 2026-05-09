mod common;

use common::repo_root;

/// Templates ship sources only — never rendered artifacts. This test fails
/// if `README.md`, `skill.md`, or any `skills/*.md` is committed under
/// `templates/`, so a careless `iii-skill-render --write` against the
/// example-worker followed by a `git add` doesn't leak rendered output
/// into the released bundle.
#[test]
fn templates_dir_ships_no_rendered_artifacts() {
    let templates = repo_root().join("templates");
    let mut offenders = Vec::new();
    walk(&templates, &mut offenders);
    assert!(
        offenders.is_empty(),
        "rendered artifacts found under templates/ — they should never be \
         checked in (the renderer regenerates them):\n  {}",
        offenders.join("\n  ")
    );
}

fn walk(dir: &std::path::Path, offenders: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            walk(&path, offenders);
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let parent_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let is_rendered = name == "README.md"
            || name == "skill.md"
            || (parent_name == "skills" && name.ends_with(".md"));
        if is_rendered {
            offenders.push(path.display().to_string());
        }
    }
}
