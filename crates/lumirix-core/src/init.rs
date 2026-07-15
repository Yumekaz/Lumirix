//! `lumirix init` — create local `.lumirix/` store.

use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::config::Config;
use crate::db::{self, DbError};
use crate::git::{self, GitInfo};
use crate::paths::LumirixPaths;
use crate::policy_default::DEFAULT_POLICY_TOML;

#[derive(Debug, Error)]
pub enum InitError {
    #[error("Lumirix is already initialized in this directory. Use --force to reinitialize.")]
    AlreadyInitialized,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Result of a successful init, used for CLI messaging.
#[derive(Debug)]
pub struct InitResult {
    pub paths: LumirixPaths,
    pub config: Config,
    pub git: GitInfo,
}

/// Initialize Lumirix in `root` (usually the current working directory).
pub fn init_project(root: &Path, force: bool) -> Result<InitResult, InitError> {
    let paths = LumirixPaths::new(root);

    if paths.is_initialized() && !force {
        return Err(InitError::AlreadyInitialized);
    }

    // Create directory tree
    fs::create_dir_all(&paths.lumirix_dir)?;
    fs::create_dir_all(&paths.policies_dir)?;
    fs::create_dir_all(&paths.runs_dir)?;
    fs::create_dir_all(&paths.db_dir)?;
    fs::create_dir_all(&paths.cache_dir)?;
    fs::create_dir_all(&paths.snapshots_dir)?;
    fs::create_dir_all(&paths.artifacts_dir)?;

    let project_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    let config = Config::default_for_project(project_name);
    config.save_to_path(&paths.config)?;

    fs::write(&paths.default_policy, DEFAULT_POLICY_TOML)?;

    db::init_database(&paths.db)?;

    let git = git::detect_git(root);

    Ok(InitResult {
        paths,
        config,
        git,
    })
}

/// Format the success message lines for `lumirix init` (spec Phase 1 criteria).
pub fn format_init_message(result: &InitResult) -> String {
    let mut lines = vec!["Lumirix initialized.".to_string()];

    if result.git.is_repo {
        lines.push("Git repo detected.".to_string());
        if let Some(ref branch) = result.git.branch {
            lines.push(format!("Current branch: {branch}"));
        } else {
            lines.push("Current branch: (detached HEAD)".to_string());
        }
        if let Some(ref commit) = result.git.commit {
            lines.push(format!("Current commit: {commit}"));
        }
    } else {
        lines.push("Git repo: not detected (limited mode).".to_string());
    }

    lines.push(format!("LLM: {}", result.config.llm_status_label()));
    lines.join("\n")
}
