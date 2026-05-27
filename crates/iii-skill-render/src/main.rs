use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

/// Render worker partials or docs sources into their skill artifacts.
///
/// Mode is always read from the nearest `.skill-check.yaml` (walking up
/// from the target). What the target should be follows from the mode:
///   - `mode: worker` → target is a worker directory containing
///     `iii.worker.yaml`; renders `README.md`, `skill.md`, `skills/*.md`.
///   - `mode: docs` → target is the docs root (or any path inside it);
///     renders one `<source>.skill.md` per in-scope doc.
///
/// A single `.md` / `.mdx` file is also accepted; the binary walks up
/// from the file to find the controlling `.skill-check.yaml` and renders
/// just that doc.
#[derive(Parser)]
#[command(name = "iii-skill-render", version = iii_skill_core::update_check::installed_version(), about, long_about = None)]
struct Cli {
    /// Worker dir, docs root, or single doc file.
    target: PathBuf,
    /// Write rendered files to disk; without this flag, renders to memory only.
    #[arg(long)]
    write: bool,
    /// Continue running even when a newer release is available.
    #[arg(long)]
    allow_old_version: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    iii_skill_core::update_check::run_gate(cli.allow_old_version);

    let config_path = find_skill_check_yaml(&cli.target).ok_or_else(|| {
        anyhow::anyhow!(
            "no `.skill-check.yaml` found at or above {}",
            cli.target.display()
        )
    })?;
    let config = iii_skill_core::config::load(&config_path)?;

    match config.resolved_mode() {
        iii_skill_core::config::Mode::Worker => {
            if !cli.target.is_dir() {
                anyhow::bail!(
                    "worker mode expects a directory target; {} is not a directory",
                    cli.target.display()
                );
            }
            if !cli.target.join("iii.worker.yaml").is_file() {
                anyhow::bail!(
                    "worker mode but {} has no iii.worker.yaml",
                    cli.target.display()
                );
            }
            render_worker_dir(&cli.target, cli.write)
        }
        iii_skill_core::config::Mode::Docs => {
            let docs_root = config_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("`.skill-check.yaml` has no parent dir"))?;
            if cli.target.is_file() {
                let docs_config = config.docs.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("docs mode but `.skill-check.yaml` has no `docs:` block")
                })?;
                if !iii_skill_core::docs::enumerate::is_in_scope(
                    &cli.target,
                    docs_root,
                    docs_config,
                )? {
                    println!(
                        "skipped {} (out of scope per `.skill-check.yaml`)",
                        cli.target.display()
                    );
                    return Ok(());
                }
                return render_single_doc(&cli.target, cli.write);
            }
            render_docs_root(docs_root, &config, cli.write)
        }
    }
}

/// Walk up from `start` looking for `.skill-check.yaml`. If `start` is a
/// file, the search begins from its parent.
fn find_skill_check_yaml(start: &Path) -> Option<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = cur.join(".skill-check.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = match cur.parent() {
            Some(p) => p.to_path_buf(),
            None => return None,
        };
    }
}

fn render_worker_dir(worker: &Path, write: bool) -> Result<()> {
    let out = iii_skill_core::render::render_worker(worker)?;
    println!(
        "rendered {} (readme {} bytes, skill {} bytes)",
        worker.display(),
        out.readme.len(),
        out.skill.len(),
    );
    if write {
        std::fs::write(worker.join("README.md"), &out.readme)?;
        std::fs::write(worker.join("skill.md"), &out.skill)?;
        // Leaves are now inlined into skill.md / README.md. Remove any
        // leftover skills/ directory from the pre-inlining layout.
        let skills = worker.join("skills");
        if skills.is_dir() {
            std::fs::remove_dir_all(&skills)?;
            println!("removed stale {}", skills.display());
        }
        println!("wrote artifacts to {}", worker.display());
    }
    Ok(())
}

fn render_docs_root(
    root: &Path,
    config: &iii_skill_core::config::Config,
    write: bool,
) -> Result<()> {
    let docs_config = config
        .docs
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("docs mode but `.skill-check.yaml` has no `docs:` block"))?;
    let docs = iii_skill_core::docs::enumerate::enumerate(root, docs_config)?;
    println!("docs in scope: {}", docs.len());

    let mut had_failure = false;
    for doc in &docs {
        match iii_skill_core::docs::render::render_doc(&doc.abs) {
            Ok(rendered) => {
                println!(
                    "rendered {} ({} bytes, type={})",
                    doc.rel,
                    rendered.body.len(),
                    rendered.frontmatter.doc_type
                );
                if write {
                    std::fs::write(doc.skill_path(), &rendered.body)
                        .with_context(|| format!("writing {}", doc.skill_path().display()))?;
                }
            }
            Err(e) => {
                eprintln!("Error: reading {}", doc.rel);
                eprintln!("Caused by:\n    {e}");
                had_failure = true;
            }
        }
    }

    if write {
        // Orphan cleanup: remove any *.skill.md whose source is gone or out
        // of scope, scoped to the in-scope source's parent dirs only so
        // we don't reach into excluded subtrees (vendored fixtures with
        // their own configs, sibling projects, etc.).
        let in_scope_skills: std::collections::HashSet<PathBuf> =
            docs.iter().map(|d| d.skill_path()).collect();
        let scan_dirs: std::collections::HashSet<PathBuf> = docs
            .iter()
            .filter_map(|d| d.abs.parent().map(Path::to_path_buf))
            .collect();
        cleanup_orphan_skills(&scan_dirs, &in_scope_skills)?;
        println!("wrote skill artifacts under {}", root.display());
    }

    if had_failure {
        anyhow::bail!("one or more docs failed to render");
    }
    Ok(())
}

fn render_single_doc(path: &Path, write: bool) -> Result<()> {
    let rendered = iii_skill_core::docs::render::render_doc(path)?;
    println!(
        "rendered {} ({} bytes, type={})",
        path.display(),
        rendered.body.len(),
        rendered.frontmatter.doc_type
    );
    if write {
        let mut skill = path.as_os_str().to_owned();
        skill.push(".skill.md");
        let skill = PathBuf::from(skill);
        std::fs::write(&skill, &rendered.body)
            .with_context(|| format!("writing {}", skill.display()))?;
        println!("wrote {}", skill.display());
    }
    Ok(())
}

/// Remove `*.skill.md` files in `scan_dirs` that aren't in `expected`.
/// Only the listed dirs are scanned (non-recursively) so excluded
/// subtrees stay untouched even if the controlling docs root is the
/// repo root.
fn cleanup_orphan_skills(
    scan_dirs: &std::collections::HashSet<PathBuf>,
    expected: &std::collections::HashSet<PathBuf>,
) -> Result<()> {
    for dir in scan_dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let is_skill = path
                .file_name()
                .and_then(|s| s.to_str())
                .map_or(false, |n| n.ends_with(".skill.md"));
            if !is_skill {
                continue;
            }
            if !expected.contains(&path) {
                std::fs::remove_file(&path)?;
                println!("removed stale {}", path.display());
            }
        }
    }
    Ok(())
}
