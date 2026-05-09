use anyhow::Context;
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "iii-skill-check", about = "Validate worker README/skill artifacts against project rules")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run all configured layers against a worker.
    Verify {
        worker: PathBuf,
        /// Subset of layers to run: structure,vale,ai (comma-separated).
        #[arg(long, default_value = "structure,vale,ai")]
        layers: String,
        /// Override the project-rules directory. Resolution order:
        /// this flag, then `.skill-check.yaml` `rules.path`, then bundled rules.
        #[arg(long)]
        rules_dir: Option<PathBuf>,
        /// Override the Vale config (.vale.ini). Resolution order:
        /// this flag, then sibling `.vale.ini` next to `.skill-check.yaml`,
        /// then bundled `.vale.ini`.
        #[arg(long)]
        vale_config: Option<PathBuf>,
    },
    /// Re-render and diff against checked-in artifacts; non-zero on drift.
    VerifyRendered { worker: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Verify {
            worker,
            layers,
            rules_dir,
            vale_config,
        } => run_verify(&worker, &layers, rules_dir, vale_config),
        Command::VerifyRendered { worker } => run_verify_rendered(&worker),
    }
}

fn run_verify(
    worker: &Path,
    layers: &str,
    rules_override: Option<PathBuf>,
    vale_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    let workers_root = worker
        .parent()
        .ok_or_else(|| anyhow::anyhow!("worker dir has no parent: {}", worker.display()))?;
    let config_path = workers_root.join(".skill-check.yaml");
    let config = iii_skill_core::config::load(&config_path)?;

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

    if !all_violations.is_empty() {
        for v in &all_violations {
            let line = v.line.map(|l| format!(":{l}")).unwrap_or_default();
            eprintln!("{}{} — {}", v.file, line, v.message);
        }
    }
    if !ai_failures.is_empty() {
        for (path, body) in &ai_failures {
            eprintln!("\n[AI] {}\n{}", path.display(), body);
        }
    }

    let total = all_violations.len() + ai_failures.len();
    if total > 0 {
        anyhow::bail!("{total} violation(s) across layers [{layers}]");
    }
    println!("verify clean across [{layers}] for {}", worker.display());
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
                "could not locate project-rules — pass --rules-dir, set rules.path \
                 in .skill-check.yaml, or run a release-installed iii-skill-check \
                 with bundled content"
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
                "could not locate .vale.ini — pass --vale-config, place a .vale.ini \
                 next to .skill-check.yaml, or run a release-installed \
                 iii-skill-check with bundled content"
            )
        })
}

fn run_verify_rendered(worker: &Path) -> anyhow::Result<()> {
    let drift = iii_skill_core::render::check_rendered(worker)?;
    if !drift.is_empty() {
        for d in &drift {
            eprintln!("{d}");
        }
        anyhow::bail!("rendered artifacts are out of date — run `iii-skill-render <worker> --write`");
    }
    println!("rendered artifacts match {}", worker.display());
    Ok(())
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
