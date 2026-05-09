//! Shared test helpers.
//!
//! `templates/example-worker/` ships only sources (no rendered README.md /
//! skill.md / skills/*.md). Tests that need rendered output use
//! `RenderedTemplate::lock` to render in place and clean up afterwards.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

pub fn example_worker_dir() -> PathBuf {
    repo_root().join("templates/example-worker")
}

/// Renders `templates/example-worker/` in place and removes the rendered
/// artifacts on drop. The mutex serializes parallel tests in the same
/// binary; cargo runs separate test binaries sequentially so cross-binary
/// races don't apply.
pub struct RenderedTemplate {
    worker: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl RenderedTemplate {
    pub fn lock() -> Self {
        static MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        let guard = MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        let worker = example_worker_dir();
        let outputs = iii_skill_core::render::render_worker(&worker)
            .expect("render_worker should succeed against templates/example-worker");
        std::fs::write(worker.join("README.md"), &outputs.readme).unwrap();
        std::fs::write(worker.join("skill.md"), &outputs.skill).unwrap();
        std::fs::create_dir_all(worker.join("skills")).unwrap();
        for (name, body) in &outputs.leaves {
            std::fs::write(worker.join("skills").join(format!("{name}.md")), body).unwrap();
        }
        RenderedTemplate {
            worker,
            _guard: guard,
        }
    }

    pub fn worker(&self) -> &Path {
        &self.worker
    }
}

impl Drop for RenderedTemplate {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.worker.join("README.md"));
        let _ = std::fs::remove_file(self.worker.join("skill.md"));
        let _ = std::fs::remove_dir_all(self.worker.join("skills"));
    }
}
