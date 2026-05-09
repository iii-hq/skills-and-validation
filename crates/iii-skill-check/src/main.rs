use anyhow::Context;
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "iii-skill-check",
    about = "Validate worker README/skill or docs skill artifacts against project rules"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Continue running even when a newer release is available.
    #[arg(long, global = true)]
    allow_old_version: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Run all configured layers against a worker dir or docs root.
    Verify {
        /// Worker directory (worker mode) or docs root (docs mode).
        target: PathBuf,
        /// Subset of layers to run: structure,vale,ai (comma-separated).
        #[arg(long, default_value = "structure,vale,ai")]
        layers: String,
        /// Override the project-rules directory. Resolution order:
        /// this flag, then `.skill-check.yaml` `rules.path`, then bundled rules.
        #[arg(long)]
        rules_dir: Option<PathBuf>,
        /// Override the Vale config (.vale.ini). Resolution order:
        /// this flag, then sibling `.vale.ini` next to `.skill-check.yaml`,
        /// then bundled `.vale.ini`. Ignored in docs mode (vale config is
        /// generated per run from the in-scope docs' frontmatter types).
        #[arg(long)]
        vale_config: Option<PathBuf>,
    },
    /// Re-render and diff against checked-in artifacts; non-zero on drift.
    VerifyRendered { target: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    iii_skill_core::update_check::run_gate(cli.allow_old_version);
    match cli.command {
        Command::Verify {
            target,
            layers,
            rules_dir,
            vale_config,
        } => dispatch_verify(&target, &layers, rules_dir, vale_config),
        Command::VerifyRendered { target } => dispatch_verify_rendered(&target),
    }
}

fn dispatch_verify(
    target: &Path,
    layers: &str,
    rules_override: Option<PathBuf>,
    vale_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    let (config_path, config) = load_controlling_config(target)?;
    let root = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("`.skill-check.yaml` has no parent dir"))?;
    match config.resolved_mode() {
        iii_skill_core::config::Mode::Worker => verify_worker(
            target,
            root,
            &config,
            layers,
            rules_override,
            vale_override,
        ),
        iii_skill_core::config::Mode::Docs => {
            // Per-target invocation symmetric with worker mode: if the
            // target is a single .md/.mdx file, verify just that doc;
            // otherwise enumerate the docs root.
            if target.is_file() {
                let docs_config = config.docs.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("docs mode but `.skill-check.yaml` has no `docs:` block")
                })?;
                if !iii_skill_core::docs::enumerate::is_in_scope(target, root, docs_config)? {
                    println!(
                        "skipped {} (out of scope per `.skill-check.yaml`)",
                        target.display()
                    );
                    return Ok(());
                }
                verify_doc_file(target, root, &config, layers, rules_override)
            } else {
                verify_docs(root, &config, layers, rules_override)
            }
        }
    }
}

fn dispatch_verify_rendered(target: &Path) -> anyhow::Result<()> {
    let (config_path, config) = load_controlling_config(target)?;
    let root = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("`.skill-check.yaml` has no parent dir"))?;
    match config.resolved_mode() {
        iii_skill_core::config::Mode::Worker => verify_rendered_worker(target),
        iii_skill_core::config::Mode::Docs => {
            if target.is_file() {
                let docs_config = config.docs.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("docs mode but `.skill-check.yaml` has no `docs:` block")
                })?;
                if !iii_skill_core::docs::enumerate::is_in_scope(target, root, docs_config)? {
                    println!(
                        "skipped {} (out of scope per `.skill-check.yaml`)",
                        target.display()
                    );
                    return Ok(());
                }
                verify_rendered_doc_file(target)
            } else {
                verify_rendered_docs(root, &config)
            }
        }
    }
}

/// Walk up from `target` to the nearest `.skill-check.yaml` and load it.
/// All dispatch starts here so the mode field is the single source of
/// truth for which validation surface to run.
fn load_controlling_config(
    target: &Path,
) -> anyhow::Result<(PathBuf, iii_skill_core::config::Config)> {
    let path = find_skill_check_yaml(target).ok_or_else(|| {
        anyhow::anyhow!(
            "no `.skill-check.yaml` found at or above {}",
            target.display()
        )
    })?;
    let config = iii_skill_core::config::load(&path)?;
    Ok((path, config))
}

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

// --- worker mode (existing) -----------------------------------------------

fn verify_worker(
    worker: &Path,
    workers_root: &Path,
    config: &iii_skill_core::config::Config,
    layers: &str,
    rules_override: Option<PathBuf>,
    vale_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    let layer_set: HashSet<&str> = layers.split(',').map(|s| s.trim()).collect();
    let artifacts = enumerate_rendered_artifacts(worker);

    let mut all_violations: Vec<iii_skill_core::structure::Violation> = Vec::new();
    let mut ai_failures: Vec<(PathBuf, String)> = Vec::new();

    if layer_set.contains("structure") {
        all_violations.extend(iii_skill_core::structure::check(worker)?);
    }

    if layer_set.contains("vale") {
        let vale_config = resolve_vale_config(workers_root, vale_override.as_deref())?;
        let refs: Vec<&Path> = artifacts.iter().map(|p| p.as_path()).collect();
        all_violations.extend(iii_skill_core::vale::run(&refs, &vale_config)?);
    }

    if layer_set.contains("ai") {
        let rules_dir = resolve_rules_dir(workers_root, &config, rules_override.as_deref())?;
        let rules = load_project_rules(&rules_dir)?;
        let prompt_path = rules_dir.join("_skill-check-prompt.md");
        let system_prompt = std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("reading {}", prompt_path.display()))?;

        for art in &artifacts {
            match iii_skill_core::ai::check_artifact(
                art,
                &rules,
                &system_prompt,
                &config.ai_check.model,
                &config.ai_check.api_key_env_var,
                config.ai_check.max_tokens,
            )? {
                Ok(()) => {}
                Err(body) => ai_failures.push((art.clone(), body)),
            }
        }
    }

    report(&all_violations, &ai_failures, layers, &worker.display().to_string())
}

fn verify_rendered_worker(worker: &Path) -> anyhow::Result<()> {
    let drift = iii_skill_core::render::check_rendered(worker)?;
    if !drift.is_empty() {
        for d in &drift {
            eprintln!("{d}");
        }
        anyhow::bail!(
            "rendered artifacts are out of date — run `iii-skill-render <worker> --write`"
        );
    }
    println!("rendered artifacts match {}", worker.display());
    Ok(())
}

// --- docs mode -------------------------------------------------------------

fn verify_docs(
    root: &Path,
    config: &iii_skill_core::config::Config,
    layers: &str,
    rules_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    let docs_config = config
        .docs
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("docs mode but `.skill-check.yaml` has no `docs:` block"))?;

    let layer_set: HashSet<&str> = layers.split(',').map(|s| s.trim()).collect();
    let docs = iii_skill_core::docs::enumerate::enumerate(root, docs_config)?;
    if docs.is_empty() {
        eprintln!("::warning::no docs matched docs.include / docs.exclude in {}", root.display());
    }

    let mut all_violations: Vec<iii_skill_core::structure::Violation> = Vec::new();
    let mut ai_failures: Vec<(PathBuf, String)> = Vec::new();

    // Resolve frontmatter once per doc; each layer reuses the type.
    let mut typed: Vec<(iii_skill_core::docs::enumerate::DiscoveredDoc, iii_skill_core::docs::frontmatter::DocType)> =
        Vec::new();
    for doc in &docs {
        if layer_set.contains("structure") {
            all_violations.extend(iii_skill_core::docs::structure::check_source(&doc.abs));
        }
        // Try to recover the doc type. If frontmatter is broken, structure
        // already flagged it; vale + ai then skip this doc rather than
        // failing the whole run with a noisy error.
        let body = match std::fs::read_to_string(&doc.abs) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warning: skipping {} (can't read: {e})", doc.rel);
                continue;
            }
        };
        if let Ok(parsed) = iii_skill_core::docs::frontmatter::parse(&body) {
            typed.push((doc.clone(), parsed.frontmatter.doc_type));
        }
    }

    if layer_set.contains("vale") && !typed.is_empty() {
        let styles_path = resolve_styles_path()?;
        let skill_paths: Vec<(PathBuf, iii_skill_core::docs::frontmatter::DocType)> = typed
            .iter()
            .map(|(d, ty)| (d.skill_path(), *ty))
            .collect();
        let refs: Vec<(&Path, iii_skill_core::docs::frontmatter::DocType)> = skill_paths
            .iter()
            .map(|(p, ty)| (p.as_path(), *ty))
            .collect();
        let cfg = iii_skill_core::docs::vale_config::build(&refs, &styles_path);
        let tmp = tempfile::TempDir::new().context("creating temp dir for vale config")?;
        let cfg_path = tmp.path().join(".vale.ini");
        std::fs::write(&cfg_path, cfg).context("writing runtime vale config")?;
        let artifact_paths: Vec<&Path> = skill_paths.iter().map(|(p, _)| p.as_path()).collect();
        all_violations.extend(iii_skill_core::vale::run(&artifact_paths, &cfg_path)?);
    }

    if layer_set.contains("ai") && !typed.is_empty() {
        let rules_dir = resolve_rules_dir(root, config, rules_override.as_deref())?;
        let rules = load_project_rules(&rules_dir)?;
        let prompt_path = rules_dir.join("_skill-check-prompt.md");
        let system_prompt = std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("reading {}", prompt_path.display()))?;
        for (doc, ty) in &typed {
            let skill = doc.skill_path();
            match iii_skill_core::ai::check_artifact_with_type(
                &skill,
                &rules,
                &system_prompt,
                *ty,
                &config.ai_check.model,
                &config.ai_check.api_key_env_var,
                config.ai_check.max_tokens,
            )? {
                Ok(()) => {}
                Err(body) => ai_failures.push((skill, body)),
            }
        }
    }

    report(&all_violations, &ai_failures, layers, &root.display().to_string())
}

/// Verify a single doc file. Used when the action iterates `docs-glob`
/// and invokes the binary per-file (mirror of how worker mode iterates
/// `workers-glob`).
fn verify_doc_file(
    source: &Path,
    docs_root: &Path,
    config: &iii_skill_core::config::Config,
    layers: &str,
    rules_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    let layer_set: HashSet<&str> = layers.split(',').map(|s| s.trim()).collect();
    let mut all_violations: Vec<iii_skill_core::structure::Violation> = Vec::new();
    let mut ai_failures: Vec<(PathBuf, String)> = Vec::new();

    if layer_set.contains("structure") {
        all_violations.extend(iii_skill_core::docs::structure::check_source(source));
    }

    let body = std::fs::read_to_string(source)
        .with_context(|| format!("reading {}", source.display()))?;
    let frontmatter = iii_skill_core::docs::frontmatter::parse(&body).ok();
    let skill_path = {
        let mut s = source.as_os_str().to_owned();
        s.push(".skill.md");
        PathBuf::from(s)
    };

    if let Some(parsed) = frontmatter.as_ref() {
        let doc_type = parsed.frontmatter.doc_type;

        if layer_set.contains("vale") && skill_path.is_file() {
            let styles_path = resolve_styles_path()?;
            let cfg = iii_skill_core::docs::vale_config::build(
                &[(skill_path.as_path(), doc_type)],
                &styles_path,
            );
            let tmp = tempfile::TempDir::new().context("creating temp dir for vale config")?;
            let cfg_path = tmp.path().join(".vale.ini");
            std::fs::write(&cfg_path, cfg).context("writing runtime vale config")?;
            all_violations
                .extend(iii_skill_core::vale::run(&[skill_path.as_path()], &cfg_path)?);
        }

        if layer_set.contains("ai") && skill_path.is_file() {
            let rules_dir = resolve_rules_dir(docs_root, config, rules_override.as_deref())?;
            let rules = load_project_rules(&rules_dir)?;
            let prompt_path = rules_dir.join("_skill-check-prompt.md");
            let system_prompt = std::fs::read_to_string(&prompt_path)
                .with_context(|| format!("reading {}", prompt_path.display()))?;
            match iii_skill_core::ai::check_artifact_with_type(
                &skill_path,
                &rules,
                &system_prompt,
                doc_type,
                &config.ai_check.model,
                &config.ai_check.api_key_env_var,
                config.ai_check.max_tokens,
            )? {
                Ok(()) => {}
                Err(body) => ai_failures.push((skill_path.clone(), body)),
            }
        }
    }

    report(&all_violations, &ai_failures, layers, &source.display().to_string())
}

fn verify_rendered_doc_file(source: &Path) -> anyhow::Result<()> {
    let mut s = source.as_os_str().to_owned();
    s.push(".skill.md");
    let skill_path = PathBuf::from(s);

    let rendered = iii_skill_core::docs::render::render_doc(source)?;
    let on_disk = std::fs::read_to_string(&skill_path).unwrap_or_default();
    if on_disk != rendered.body {
        eprintln!(
            "{}.skill.md is out of date — re-run `iii-skill-render {}`",
            source.display(),
            source.display()
        );
        anyhow::bail!("rendered skill artifact is out of date");
    }
    println!("rendered skill artifact matches {}", source.display());
    Ok(())
}

fn verify_rendered_docs(
    root: &Path,
    config: &iii_skill_core::config::Config,
) -> anyhow::Result<()> {
    let docs_config = config
        .docs
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("docs mode but `.skill-check.yaml` has no `docs:` block"))?;
    let drift = iii_skill_core::docs::check_rendered::check_rendered(root, docs_config)?;
    if !drift.is_empty() {
        for d in &drift {
            eprintln!("{d}");
        }
        anyhow::bail!(
            "docs skill artifacts are out of date — run `iii-skill-render <docs-root> --write`"
        );
    }
    println!("docs skill artifacts match sources in {}", root.display());
    Ok(())
}

// --- shared helpers --------------------------------------------------------

fn report(
    violations: &[iii_skill_core::structure::Violation],
    ai_failures: &[(PathBuf, String)],
    layers: &str,
    target_label: &str,
) -> anyhow::Result<()> {
    if !violations.is_empty() {
        for v in violations {
            let line = v.line.map(|l| format!(":{l}")).unwrap_or_default();
            eprintln!("{}{} — {}", v.file, line, v.message);
        }
    }
    if !ai_failures.is_empty() {
        for (path, body) in ai_failures {
            eprintln!("\n[AI] {}\n{}", path.display(), body);
        }
    }

    let total = violations.len() + ai_failures.len();
    if total > 0 {
        anyhow::bail!("{total} violation(s) across layers [{layers}]");
    }
    println!("verify clean across [{layers}] for {target_label}");
    Ok(())
}

/// Resolve project-rules directory. Order: CLI flag, config field, bundled.
fn resolve_rules_dir(
    workers_root: &Path,
    config: &iii_skill_core::config::Config,
    cli_override: Option<&Path>,
) -> anyhow::Result<PathBuf> {
    if let Some(p) = cli_override {
        return Ok(p.to_path_buf());
    }
    if let Some(rules) = &config.rules {
        return Ok(workers_root.join(&rules.path));
    }
    iii_skill_core::bundle::find_content_root()
        .map(|c| c.join("project-rules"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not locate project-rules — install via scripts/install.sh \
                 (drops content into ~/.local/share/skill-check/current/), pass \
                 --rules-dir, or set rules.path in .skill-check.yaml"
            )
        })
}

/// Resolve `.vale.ini`. Order: CLI flag, sibling `.vale.ini` in workers_root, bundled.
fn resolve_vale_config(
    workers_root: &Path,
    cli_override: Option<&Path>,
) -> anyhow::Result<PathBuf> {
    if let Some(p) = cli_override {
        return Ok(p.to_path_buf());
    }
    let local = workers_root.join(".vale.ini");
    if local.is_file() {
        return Ok(local);
    }
    iii_skill_core::bundle::find_content_root()
        .map(|c| c.join(".vale.ini"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not locate .vale.ini — install via scripts/install.sh \
                 (drops .vale.ini into ~/.local/share/skill-check/current/content/), \
                 pass --vale-config, or place a .vale.ini next to .skill-check.yaml"
            )
        })
}

/// Resolve the styles directory for docs-mode runtime vale configs. The
/// generated `.vale.ini` references it via `StylesPath = <here>`. Same
/// fallback chain as `resolve_vale_config` minus the CLI flag (no override
/// for docs mode yet).
fn resolve_styles_path() -> anyhow::Result<String> {
    let bundle_root = iii_skill_core::bundle::find_content_root().ok_or_else(|| {
        anyhow::anyhow!(
            "could not locate bundled content — install via scripts/install.sh \
             (drops styles into ~/.local/share/skill-check/current/content/styles/)"
        )
    })?;
    Ok(bundle_root
        .join("styles")
        .to_string_lossy()
        .into_owned())
}

fn enumerate_rendered_artifacts(worker: &Path) -> Vec<PathBuf> {
    let mut out = vec![worker.join("README.md"), worker.join("skill.md")];
    let skills = worker.join("skills");
    if let Ok(entries) = std::fs::read_dir(&skills) {
        let mut leaf_paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
            .collect();
        leaf_paths.sort();
        out.extend(leaf_paths);
    }
    out
}

fn load_project_rules(rules_dir: &Path) -> anyhow::Result<String> {
    let mut combined = String::new();
    let mut entries: Vec<_> = std::fs::read_dir(rules_dir)
        .with_context(|| format!("reading {}", rules_dir.display()))?
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
        let body = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        combined.push_str(&format!("# {name}\n\n{body}\n\n"));
    }
    Ok(combined)
}
