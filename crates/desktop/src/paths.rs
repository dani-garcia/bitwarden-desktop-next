//! Filesystem paths shared across modules.

use std::path::PathBuf;

/// Workspace-root `data/` folder. Resolves from cwd in the dev `cargo run`
/// case; falls back to the executable's parent directory when cwd can't be
/// read (sandboxed `.app` bundles, packagers that launch from `/`, the
/// directory was deleted out from under us, etc.).
pub fn data_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        return cwd.join("data");
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        return parent.join("data");
    }
    PathBuf::from("data")
}
