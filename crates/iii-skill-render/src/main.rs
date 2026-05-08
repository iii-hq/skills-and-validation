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
        println!("wrote artifacts to {}", cli.worker.display());
    }
    Ok(())
}
