use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Render `docs/*` + `iii.worker.yaml.name` into `README.md`, `skill.md`, and
/// `skills/*.md` for a worker directory.
#[derive(Parser)]
#[command(name = "iii-skill-render", about, long_about = None)]
struct Cli {
    /// Worker directory to render.
    worker: PathBuf,
    /// Write rendered files to disk; without this flag, renders to memory only.
    #[arg(long)]
    write: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let out = iii_skill_core::render::render_worker(&cli.worker)?;
    println!(
        "rendered {} (readme {} bytes, skill {} bytes, {} leaves)",
        cli.worker.display(),
        out.readme.len(),
        out.skill.len(),
        out.leaves.len(),
    );
    if cli.write {
        std::fs::write(cli.worker.join("README.md"), &out.readme)?;
        std::fs::write(cli.worker.join("skill.md"), &out.skill)?;
        std::fs::create_dir_all(cli.worker.join("skills"))?;
        for (leaf, body) in &out.leaves {
            std::fs::write(cli.worker.join("skills").join(format!("{leaf}.md")), body)?;
        }
        // Cleanup: remove any skills/<name>.md whose source partial no
        // longer exists. Without this, removing a docs/leaves/<name>.md
        // leaves the rendered output behind and the next verify-rendered
        // would (now) flag it as an orphan.
        for disk_leaf in iii_skill_core::render::list_rendered_leaves(&cli.worker) {
            if !out.leaves.contains_key(&disk_leaf) {
                let path = cli
                    .worker
                    .join("skills")
                    .join(format!("{disk_leaf}.md"));
                std::fs::remove_file(&path)?;
                println!("removed stale {}", path.display());
            }
        }
        println!("wrote artifacts to {}", cli.worker.display());
    }
    Ok(())
}
