//! Filesystem paths shared across modules.

use std::path::PathBuf;

/// Workspace-root `data/` folder. Resolved from cwd — the app is expected to be
/// launched from the workspace root (`cargo run`).
pub fn data_dir() -> PathBuf {
    std::env::current_dir()
        .expect("cwd is readable")
        .join("data")
}
