//! Git detection and diff helpers via the system `git` CLI.

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

/// One file's numstat line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumstatEntry {
    pub path: String,
    pub added: u64,
    pub deleted: u64,
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

/// True when `git status --porcelain` is empty.
pub fn is_worktree_clean(cwd: &Path) -> bool {
    match status_porcelain(cwd) {
        Ok(s) => s.trim().is_empty(),
        Err(_) => false,
    }
}

pub fn is_worktree_dirty(cwd: &Path) -> bool {
    !is_worktree_clean(cwd)
}

/// Raw porcelain status (empty string if clean).
pub fn status_porcelain(cwd: &Path) -> Result<String, String> {
    git_stdout(cwd, &["status", "--porcelain"]).map_err(|_| "git status failed".to_string())
}

/// Untracked paths from porcelain (`?? path`).
pub fn untracked_paths(cwd: &Path) -> Vec<String> {
    let Ok(raw) = status_porcelain(cwd) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let line = line.trim_end();
            if let Some(rest) = line.strip_prefix("?? ") {
                Some(rest.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// All uncommitted tracked changes vs HEAD (staged + unstaged).
pub fn diff_vs_head(cwd: &Path) -> Result<String, String> {
    git_stdout(cwd, &["diff", "HEAD"]).map_err(|_| "git diff HEAD failed".to_string())
}

/// Reverse of uncommitted tracked changes vs HEAD.
pub fn diff_vs_head_reverse(cwd: &Path) -> Result<String, String> {
    git_stdout(cwd, &["diff", "-R", "HEAD"]).map_err(|_| "git diff -R HEAD failed".to_string())
}

/// Parse `git diff --numstat HEAD`.
pub fn numstat_vs_head(cwd: &Path) -> Result<Vec<NumstatEntry>, String> {
    let raw =
        git_stdout(cwd, &["diff", "--numstat", "HEAD"]).map_err(|_| "git diff --numstat failed".to_string())?;
    Ok(parse_numstat(&raw))
}

fn parse_numstat(raw: &str) -> Vec<NumstatEntry> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        // format: <added>\t<deleted>\t<path>
        // binary: -\t-\t<path>
        let mut parts = line.splitn(3, '\t');
        let added_s = parts.next().unwrap_or("0");
        let deleted_s = parts.next().unwrap_or("0");
        let path = parts.next().unwrap_or("").to_string();
        if path.is_empty() {
            continue;
        }
        let added = if added_s == "-" {
            0
        } else {
            added_s.parse().unwrap_or(0)
        };
        let deleted = if deleted_s == "-" {
            0
        } else {
            deleted_s.parse().unwrap_or(0)
        };
        out.push(NumstatEntry {
            path,
            added,
            deleted,
        });
    }
    out
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

#[cfg(test)]
mod tests {
    use super::parse_numstat;

    #[test]
    fn parse_numstat_basic() {
        let raw = "10\t2\tsrc/foo.rs\n3\t0\tREADME.md\n-\t-\tbin.dat\n";
        let entries = parse_numstat(raw);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "src/foo.rs");
        assert_eq!(entries[0].added, 10);
        assert_eq!(entries[0].deleted, 2);
        assert_eq!(entries[2].added, 0);
    }
}
