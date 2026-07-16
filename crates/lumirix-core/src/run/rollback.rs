//! Export rollback patch for a captured run.

use std::fs;
use std::path::{Path, PathBuf};

use super::store::{self, StoreError};
use crate::paths::LumirixPaths;

#[derive(Debug)]
pub struct RollbackExport {
    pub run_id: String,
    pub source: PathBuf,
    pub dest: PathBuf,
    pub bytes: u64,
    pub empty: bool,
    pub status_note: String,
}

/// Copy the run's `rollback.patch` to `dest` (default: `./rollback.patch`).
pub fn export_rollback_patch(
    paths: &LumirixPaths,
    run_id: &str,
    dest: &Path,
) -> Result<RollbackExport, StoreError> {
    let run_paths = store::RunPaths::new(&paths.runs_dir, run_id);
    if !run_paths.rollback_patch.is_file() {
        return Err(StoreError::NotFound(format!(
            "rollback.patch for run {run_id} (run a capture that changes tracked files first)"
        )));
    }

    let content = fs::read(&run_paths.rollback_patch)?;
    let empty = content.iter().all(|b| b.is_ascii_whitespace());
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(dest, &content)?;

    let status_note = if empty {
        "Rollback patch is empty (no tracked reverse diff). Untracked files are not reversed."
            .to_string()
    } else {
        "Rollback patch written. Apply with: git apply rollback.patch (review first). Untracked files may still remain."
            .to_string()
    };

    Ok(RollbackExport {
        run_id: run_id.to_string(),
        source: run_paths.rollback_patch,
        dest: dest.to_path_buf(),
        bytes: content.len() as u64,
        empty,
        status_note,
    })
}

/// Default destination path for `--write` without a value.
pub fn default_rollback_dest(cwd: &Path) -> PathBuf {
    cwd.join("rollback.patch")
}
