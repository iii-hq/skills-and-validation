use anyhow::Context;
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "iii-skill-check",
    version = iii_skill_core::update_check::installed_version(),
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
    /// Validate a single file with an explicit doc type.
    ///
    /// Useful for files that don't carry frontmatter (project READMEs,
    /// CHANGELOGs, contributor guides) but still need to follow the same
    /// voice and Diataxis rules. The binary walks up to the nearest
    /// `.skill-check.yaml` for `ai_check` settings; the in-scope check
    /// from docs mode does NOT apply — `check-file` is explicit by design.
    CheckFile {
        /// Path to the file to validate (`.md` / `.mdx`).
        target: PathBuf,
        /// Diataxis type the file should be validated as.
        #[arg(long, value_enum)]
        r#type: DocTypeArg,
        /// Subset of layers to run: structure,vale,ai (comma-separated).
        /// `structure` here only checks llm-only-block balance — there's
        /// no frontmatter to validate.
        #[arg(long, default_value = "structure,vale,ai")]
        layers: String,
        /// Override the project-rules directory. Resolution order:
        /// this flag, then `.skill-check.yaml` `rules.path`, then bundled rules.
        #[arg(long)]
        rules_dir: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum DocTypeArg {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
}

impl From<DocTypeArg> for iii_skill_core::docs::frontmatter::DocType {
    fn from(v: DocTypeArg) -> Self {
        use iii_skill_core::docs::frontmatter::DocType;
        match v {
            DocTypeArg::Tutorial => DocType::Tutorial,
            DocTypeArg::HowTo => DocType::HowTo,
            DocTypeArg::Reference => DocType::Reference,
            DocTypeArg::Explanation => DocType::Explanation,
        }
    }
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
        Command::CheckFile {
            target,
            r#type,
            layers,
            rules_dir,
        } => check_file(&target, r#type.into(), &layers, rules_dir),
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

// --- shared staging --------------------------------------------------------

/// One rendered skill artifact staged on disk so `vale` can read it,
/// alongside the canonical path the user expects to see in violation
/// reports and the in-memory body so AI checks can skip a round-trip
/// read.
struct StagedArtifact {
    /// Where the artifact would live on disk if the renderer had been
    /// run (e.g. `<source>.skill.md`, `<worker>/README.md`). Vale
    /// violations and AI prompts both reference this path.
    canonical: PathBuf,
    /// Where we actually wrote the rendered bytes for vale to read.
    temp: PathBuf,
    /// Rendered bytes — handed to the in-memory AI check so we don't
    /// re-read from disk.
    body: String,
}

/// Replace vale's temp-path file references with the canonical paths.
/// Vale takes file paths via argv and echoes them back in its JSON
/// output; without this rewrite, users would see violations against
/// scratch dirs like `/tmp/xxx/a0/README.md`.
fn rewrite_vale_paths(
    violations: &mut [iii_skill_core::structure::Violation],
    staged: &[StagedArtifact],
) {
    for v in violations.iter_mut() {
        let v_file = v.file.as_str();
        for s in staged {
            if v_file == s.temp.to_string_lossy() {
                v.file = s.canonical.display().to_string();
                break;
            }
        }
    }
}

// --- worker mode -----------------------------------------------------------

fn verify_worker(
    worker: &Path,
    workers_root: &Path,
    config: &iii_skill_core::config::Config,
    layers: &str,
    rules_override: Option<PathBuf>,
    vale_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    let layer_set: HashSet<&str> = layers.split(',').map(|s| s.trim()).collect();

    let mut all_violations: Vec<iii_skill_core::structure::Violation> = Vec::new();
    let mut ai_failures: Vec<(PathBuf, String)> = Vec::new();

    if layer_set.contains("structure") {
        all_violations.extend(iii_skill_core::structure::check(worker)?);
    }

    let needs_artifact = layer_set.contains("vale") || layer_set.contains("ai");
    if !needs_artifact {
        return report(&all_violations, &ai_failures, layers, &worker.display().to_string());
    }

    // Render in memory so vale + ai always see fresh artifacts — running
    // `iii-skill-check verify <worker>` without a prior `render --write`
    // would otherwise silently skip vale + ai when the rendered files
    // don't exist yet.
    let rendered = match iii_skill_core::render::render_worker(worker) {
        Ok(r) => r,
        Err(e) => {
            all_violations.push(iii_skill_core::structure::Violation {
                file: worker.display().to_string(),
                line: None,
                message: format!("render failed (cannot run vale/ai): {e}"),
            });
            return report(&all_violations, &ai_failures, layers, &worker.display().to_string());
        }
    };

    // Mirror the worker layout inside a temp dir so vale's `[**/README.md]`,
    // `[**/skill.md]`, and `[**/skills/*.md]` globs in content/.vale.ini
    // still match.
    let stage = tempfile::TempDir::new()
        .context("creating temp dir for staged worker artifacts")?;
    let stage_root = stage.path().join("worker");
    let stage_skills = stage_root.join("skills");
    std::fs::create_dir_all(&stage_skills)?;

    let mut staged: Vec<StagedArtifact> = Vec::new();

    let readme_tmp = stage_root.join("README.md");
    std::fs::write(&readme_tmp, &rendered.readme)?;
    staged.push(StagedArtifact {
        canonical: worker.join("README.md"),
        temp: readme_tmp,
        body: rendered.readme.clone(),
    });

    let skill_tmp = stage_root.join("skill.md");
    std::fs::write(&skill_tmp, &rendered.skill)?;
    staged.push(StagedArtifact {
        canonical: worker.join("skill.md"),
        temp: skill_tmp,
        body: rendered.skill.clone(),
    });

    for (leaf, body) in &rendered.leaves {
        let leaf_tmp = stage_skills.join(format!("{leaf}.md"));
        std::fs::write(&leaf_tmp, body)?;
        staged.push(StagedArtifact {
            canonical: worker.join("skills").join(format!("{leaf}.md")),
            temp: leaf_tmp,
            body: body.clone(),
        });
    }

    if layer_set.contains("vale") {
        let vale_config = resolve_vale_config(workers_root, vale_override.as_deref())?;
        let refs: Vec<&Path> = staged.iter().map(|s| s.temp.as_path()).collect();
        let mut vs = iii_skill_core::vale::run(&refs, &vale_config)?;
        rewrite_vale_paths(&mut vs, &staged);
        all_violations.extend(vs);
    }

    if layer_set.contains("ai") {
        let rules_dir = resolve_rules_dir(workers_root, config, rules_override.as_deref())?;
        // Worker README/skill/skills/leaves are how-to per content/.vale.ini.
        // Load the matching diataxis guide so the AI sees the same authoring
        // rules a human author reads from iii-doc-authoring/diataxis/doc_howto.md.
        let rules = load_project_rules(
            &rules_dir,
            Some(iii_skill_core::docs::frontmatter::DocType::HowTo),
        )?;
        let prompt_path = rules_dir.join("_skill-check-prompt.md");
        let system_prompt = std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("reading {}", prompt_path.display()))?;

        for art in &staged {
            match iii_skill_core::ai::check_artifact_text(
                &art.body,
                &art.canonical,
                &rules,
                &system_prompt,
                &config.ai_check.model,
                &config.ai_check.api_key_env_var,
                config.ai_check.max_tokens,
            )? {
                Ok(()) => {}
                Err(body) => ai_failures.push((art.canonical.clone(), body)),
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

    // Render each in-scope doc in memory and stage it for vale. Doing the
    // render here (rather than reading `<source>.skill.md` from disk)
    // makes verify self-contained — running it without a prior
    // `iii-skill-render --write` no longer silently skips vale + ai.
    let needs_artifact = layer_set.contains("vale") || layer_set.contains("ai");
    let stage = if needs_artifact {
        Some(
            tempfile::TempDir::new()
                .context("creating temp dir for staged skill artifacts")?,
        )
    } else {
        None
    };
    let mut staged: Vec<StagedArtifact> = Vec::new();
    let mut staged_doc_types: Vec<iii_skill_core::docs::frontmatter::DocType> = Vec::new();

    for (idx, doc) in docs.iter().enumerate() {
        if layer_set.contains("structure") {
            all_violations.extend(iii_skill_core::docs::structure::check_source(&doc.abs));
        }
        if !needs_artifact {
            continue;
        }
        let rendered = match iii_skill_core::docs::render::render_doc(&doc.abs) {
            Ok(r) => r,
            Err(e) => {
                // Surface render failures so vale/ai don't silently
                // "pass" for unrenderable docs; structure layer often
                // flags the same frontmatter issue with more detail.
                all_violations.push(iii_skill_core::structure::Violation {
                    file: doc.rel.clone(),
                    line: None,
                    message: format!("render failed (cannot run vale/ai): {e}"),
                });
                continue;
            }
        };
        let canonical = doc.skill_path();
        let stage_dir = stage.as_ref().unwrap().path().join(format!("a{idx}"));
        std::fs::create_dir_all(&stage_dir)?;
        let basename = canonical.file_name().ok_or_else(|| {
            anyhow::anyhow!("skill artifact has no basename: {}", canonical.display())
        })?;
        let temp = stage_dir.join(basename);
        std::fs::write(&temp, &rendered.body)
            .with_context(|| format!("writing staged artifact to {}", temp.display()))?;
        staged.push(StagedArtifact {
            canonical,
            temp,
            body: rendered.body,
        });
        staged_doc_types.push(rendered.frontmatter.doc_type);
    }

    if layer_set.contains("vale") && !staged.is_empty() {
        let styles_path = resolve_styles_path()?;
        let refs: Vec<(&Path, iii_skill_core::docs::frontmatter::DocType)> = staged
            .iter()
            .zip(staged_doc_types.iter())
            .map(|(s, ty)| (s.temp.as_path(), *ty))
            .collect();
        let cfg = iii_skill_core::docs::vale_config::build(&refs, &styles_path);
        let tmp = tempfile::TempDir::new().context("creating temp dir for vale config")?;
        let cfg_path = tmp.path().join(".vale.ini");
        std::fs::write(&cfg_path, cfg).context("writing runtime vale config")?;
        let artifact_paths: Vec<&Path> = staged.iter().map(|s| s.temp.as_path()).collect();
        let mut vs = iii_skill_core::vale::run(&artifact_paths, &cfg_path)?;
        rewrite_vale_paths(&mut vs, &staged);
        all_violations.extend(vs);
    }

    if layer_set.contains("ai") && !staged.is_empty() {
        let rules_dir = resolve_rules_dir(root, config, rules_override.as_deref())?;
        let prompt_path = rules_dir.join("_skill-check-prompt.md");
        let system_prompt = std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("reading {}", prompt_path.display()))?;
        // Cache rules by doc type so we don't re-read the diataxis guides
        // once per artifact when many share a category.
        let mut rules_by_type: std::collections::HashMap<
            iii_skill_core::docs::frontmatter::DocType,
            String,
        > = std::collections::HashMap::new();
        for (art, ty) in staged.iter().zip(staged_doc_types.iter()) {
            let rules = match rules_by_type.get(ty) {
                Some(r) => r.clone(),
                None => {
                    let r = load_project_rules(&rules_dir, Some(*ty))?;
                    rules_by_type.insert(*ty, r.clone());
                    r
                }
            };
            match iii_skill_core::ai::check_artifact_text_with_type(
                &art.body,
                &art.canonical,
                &rules,
                &system_prompt,
                *ty,
                &config.ai_check.model,
                &config.ai_check.api_key_env_var,
                config.ai_check.max_tokens,
            )? {
                Ok(()) => {}
                Err(body) => ai_failures.push((art.canonical.clone(), body)),
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

    let needs_artifact = layer_set.contains("vale") || layer_set.contains("ai");
    if needs_artifact {
        let skill_path = {
            let mut s = source.as_os_str().to_owned();
            s.push(".skill.md");
            PathBuf::from(s)
        };
        match iii_skill_core::docs::render::render_doc(source) {
            Ok(rendered) => {
                let doc_type = rendered.frontmatter.doc_type;
                let stage = tempfile::TempDir::new()
                    .context("creating temp dir for staged skill artifact")?;
                let basename = skill_path.file_name().ok_or_else(|| {
                    anyhow::anyhow!(
                        "skill artifact has no basename: {}",
                        skill_path.display()
                    )
                })?;
                let temp = stage.path().join(basename);
                std::fs::write(&temp, &rendered.body)
                    .with_context(|| format!("writing staged artifact to {}", temp.display()))?;
                let staged = vec![StagedArtifact {
                    canonical: skill_path.clone(),
                    temp: temp.clone(),
                    body: rendered.body,
                }];

                if layer_set.contains("vale") {
                    let styles_path = resolve_styles_path()?;
                    let cfg = iii_skill_core::docs::vale_config::build(
                        &[(temp.as_path(), doc_type)],
                        &styles_path,
                    );
                    let cfg_tmp = tempfile::TempDir::new()
                        .context("creating temp dir for vale config")?;
                    let cfg_path = cfg_tmp.path().join(".vale.ini");
                    std::fs::write(&cfg_path, cfg).context("writing runtime vale config")?;
                    let mut vs = iii_skill_core::vale::run(&[temp.as_path()], &cfg_path)?;
                    rewrite_vale_paths(&mut vs, &staged);
                    all_violations.extend(vs);
                }

                if layer_set.contains("ai") {
                    let rules_dir =
                        resolve_rules_dir(docs_root, config, rules_override.as_deref())?;
                    let rules = load_project_rules(&rules_dir, Some(doc_type))?;
                    let prompt_path = rules_dir.join("_skill-check-prompt.md");
                    let system_prompt = std::fs::read_to_string(&prompt_path)
                        .with_context(|| format!("reading {}", prompt_path.display()))?;
                    match iii_skill_core::ai::check_artifact_text_with_type(
                        &staged[0].body,
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
            Err(e) => {
                // Surface render failures so vale/ai don't silently "pass"
                // for unrenderable docs; structure layer usually flags the
                // same frontmatter issue with more detail.
                all_violations.push(iii_skill_core::structure::Violation {
                    file: source.display().to_string(),
                    line: None,
                    message: format!("render failed (cannot run vale/ai): {e}"),
                });
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

// --- check-file ------------------------------------------------------------

fn check_file(
    target: &Path,
    doc_type: iii_skill_core::docs::frontmatter::DocType,
    layers: &str,
    rules_override: Option<PathBuf>,
) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!("check-file target is not a file: {}", target.display());
    }
    let (config_path, config) = load_controlling_config(target)?;
    let root = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("`.skill-check.yaml` has no parent dir"))?;

    let layer_set: HashSet<&str> = layers.split(',').map(|s| s.trim()).collect();
    let mut all_violations: Vec<iii_skill_core::structure::Violation> = Vec::new();
    let mut ai_failures: Vec<(PathBuf, String)> = Vec::new();

    if layer_set.contains("structure") {
        // Without frontmatter the only meaningful structure check is
        // llm-only-block balance.
        let body = std::fs::read_to_string(target)
            .with_context(|| format!("reading {}", target.display()))?;
        let starts = body.matches("<!-- llm-only:start -->").count();
        let ends = body.matches("<!-- llm-only:end -->").count();
        if starts != ends {
            all_violations.push(iii_skill_core::structure::Violation {
                file: target.display().to_string(),
                line: None,
                message: format!(
                    "unbalanced llm-only blocks: {starts} start markers, {ends} end markers"
                ),
            });
        }
    }

    if layer_set.contains("vale") {
        let styles_path = resolve_styles_path()?;
        let cfg = iii_skill_core::docs::vale_config::build(&[(target, doc_type)], &styles_path);
        let tmp = tempfile::TempDir::new().context("creating temp dir for vale config")?;
        let cfg_path = tmp.path().join(".vale.ini");
        std::fs::write(&cfg_path, cfg).context("writing runtime vale config")?;
        all_violations.extend(iii_skill_core::vale::run(&[target], &cfg_path)?);
    }

    if layer_set.contains("ai") {
        let rules_dir = resolve_rules_dir(root, &config, rules_override.as_deref())?;
        let rules = load_project_rules(&rules_dir, Some(doc_type))?;
        let prompt_path = rules_dir.join("_skill-check-prompt.md");
        let system_prompt = std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("reading {}", prompt_path.display()))?;
        match iii_skill_core::ai::check_artifact_with_type(
            target,
            &rules,
            &system_prompt,
            doc_type,
            &config.ai_check.model,
            &config.ai_check.api_key_env_var,
            config.ai_check.max_tokens,
        )? {
            Ok(()) => {}
            Err(body) => ai_failures.push((target.to_path_buf(), body)),
        }
    }

    report(&all_violations, &ai_failures, layers, &target.display().to_string())
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

/// Concatenate `content/project-rules/*.md` (the always-on rules), plus
/// the matching diataxis-quadrant authoring guide(s) when a doc type is
/// supplied. Worker artifacts pass `Some(DocType::HowTo)` because every
/// rendered worker README/skill/leaf is how-to in shape; docs-mode
/// artifacts pass the type from their frontmatter.
///
/// The diataxis bundle has both a global file (`doc_workflow.md`,
/// always loaded when a type is given) and a per-quadrant file
/// (`doc_<type>.md`). Both go into the AI prompt's user message so the
/// model sees the same authoring rules a human writer would read.
fn load_project_rules(
    rules_dir: &Path,
    doc_type: Option<iii_skill_core::docs::frontmatter::DocType>,
) -> anyhow::Result<String> {
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
    if let Some(ty) = doc_type {
        if let Some(diataxis_dir) = locate_diataxis_dir() {
            let workflow = diataxis_dir.join("doc_workflow.md");
            if let Ok(body) = std::fs::read_to_string(&workflow) {
                combined.push_str(&format!("# diataxis/doc_workflow.md\n\n{body}\n\n"));
            }
            let type_file = match ty {
                iii_skill_core::docs::frontmatter::DocType::Tutorial => "doc_tutorial.md",
                iii_skill_core::docs::frontmatter::DocType::HowTo => "doc_howto.md",
                iii_skill_core::docs::frontmatter::DocType::Reference => "doc_reference.md",
                iii_skill_core::docs::frontmatter::DocType::Explanation => "doc_explanation.md",
            };
            let type_path = diataxis_dir.join(type_file);
            if let Ok(body) = std::fs::read_to_string(&type_path) {
                combined.push_str(&format!("# diataxis/{type_file}\n\n{body}\n\n"));
            }
        }
    }
    Ok(combined)
}

/// Locate the diataxis writing-guide directory inside the bundled
/// content. Returns `None` (rather than erroring) so a missing diataxis
/// bundle silently falls back to the original project-rules-only prompt
/// — old bundles still validate, just without per-quadrant context.
fn locate_diataxis_dir() -> Option<PathBuf> {
    iii_skill_core::bundle::find_content_root()
        .map(|c| c.join("skills").join("iii-doc-authoring").join("diataxis"))
        .filter(|p| p.is_dir())
}
