//! Capture Git diffs after a run.

use std::path::Path;

use crate::git;

use super::model::{DiffSummary, FileDiffStat};
use super::store::{self, RunPaths, StoreError};

/// Capture tracked diffs vs HEAD, write patches, return summary.
pub fn capture_diff_for_run(
    cwd: &Path,
    run_paths: &RunPaths,
    base_commit: Option<String>,
    dirty_before: bool,
    generate_rollback: bool,
) -> Result<DiffSummary, StoreError> {
    let dirty_after = git::is_worktree_dirty(cwd);
    let untracked = git::untracked_paths(cwd);

    let forward = git::diff_vs_head(cwd).unwrap_or_default();
    let reverse = if generate_rollback {
        git::diff_vs_head_reverse(cwd).unwrap_or_default()
    } else {
        String::new()
    };
    let numstat = git::numstat_vs_head(cwd).unwrap_or_default();

    let files: Vec<FileDiffStat> = numstat
        .into_iter()
        .map(|e| FileDiffStat {
            path: e.path,
            added: e.added,
            deleted: e.deleted,
        })
        .collect();

    let lines_added: u64 = files.iter().map(|f| f.added).sum();
    let lines_deleted: u64 = files.iter().map(|f| f.deleted).sum();
    let files_changed = files.len() as u32;

    let has_forward = !forward.trim().is_empty();
    let has_reverse = !reverse.trim().is_empty();

    store::write_text(&run_paths.diff_patch, &forward)?;
    if generate_rollback {
        store::write_text(&run_paths.rollback_patch, &reverse)?;
    }

    let rollback_status = if !generate_rollback {
        "disabled".to_string()
    } else if files_changed == 0 && untracked.is_empty() {
        "no_changes".to_string()
    } else if has_reverse && untracked.is_empty() {
        "available".to_string()
    } else if has_reverse && !untracked.is_empty() {
        "partial".to_string()
    } else if !has_reverse && !untracked.is_empty() {
        "partial".to_string()
    } else {
        "unavailable".to_string()
    };

    let summary = DiffSummary {
        base_commit,
        dirty_before,
        dirty_after,
        files_changed,
        lines_added: lines_added as u32,
        lines_deleted: lines_deleted as u32,
        files,
        untracked,
        diff_patch: has_forward,
        rollback_patch: has_reverse && generate_rollback,
        rollback_status,
    };

    store::write_diff_summary(run_paths, &summary)?;
    Ok(summary)
}

/// Summary when Git is not available.
pub fn no_git_summary(dirty_before: bool) -> DiffSummary {
    DiffSummary {
        base_commit: None,
        dirty_before,
        dirty_after: dirty_before,
        files_changed: 0,
        lines_added: 0,
        lines_deleted: 0,
        files: Vec::new(),
        untracked: Vec::new(),
        diff_patch: false,
        rollback_patch: false,
        rollback_status: "no_git".to_string(),
    }
}
