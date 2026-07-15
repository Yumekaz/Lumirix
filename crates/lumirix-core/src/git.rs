//! Git detection via the system `git` CLI (Phase 1 — no libgit2).

use std::path::Path;
use std::process::Command;

/// Snapshot of Git state for status / init messaging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInfo {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub commit: Option<String>,
}

impl GitInfo {
    pub fn not_a_repo() -> Self {
        Self {
            is_repo: false,
            branch: None,
            commit: None,
        }
    }
}

/// Probe Git for the given working directory.
pub fn detect_git(cwd: &Path) -> GitInfo {
    if !git_ok(cwd, &["rev-parse", "--is-inside-work-tree"]) {
        return GitInfo::not_a_repo();
    }

    let inside = git_stdout(cwd, &["rev-parse", "--is-inside-work-tree"])
        .map(|s| s.trim() == "true")
        .unwrap_or(false);

    if !inside {
        return GitInfo::not_a_repo();
    }

    let branch = git_stdout(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD");

    // Prefer short SHA; fall back to full.
    let commit = git_stdout(cwd, &["rev-parse", "--short", "HEAD"])
        .or_else(|_| git_stdout(cwd, &["rev-parse", "HEAD"]))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    GitInfo {
        is_repo: true,
        branch,
        commit,
    }
}

fn git_ok(cwd: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String, ()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|_| ())?;

    if !output.status.success() {
        return Err(());
    }

    String::from_utf8(output.stdout).map_err(|_| ())
}
