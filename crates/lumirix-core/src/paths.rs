//! Paths under the local `.lumirix/` project store.

use std::path::{Path, PathBuf};

/// Directory name for local Lumirix state.
pub const LUMIRIX_DIR: &str = ".lumirix";

/// Paths relative to a project root (current working directory).
#[derive(Debug, Clone)]
pub struct LumirixPaths {
    pub root: PathBuf,
    pub lumirix_dir: PathBuf,
    pub config: PathBuf,
    pub policies_dir: PathBuf,
    pub default_policy: PathBuf,
    pub runs_dir: PathBuf,
    pub db_dir: PathBuf,
    pub db: PathBuf,
    pub cache_dir: PathBuf,
    pub snapshots_dir: PathBuf,
    pub artifacts_dir: PathBuf,
}

impl LumirixPaths {
    /// Build paths for a project root directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let lumirix_dir = root.join(LUMIRIX_DIR);
        let policies_dir = lumirix_dir.join("policies");
        let db_dir = lumirix_dir.join("db");

        Self {
            config: lumirix_dir.join("config.toml"),
            default_policy: policies_dir.join("default.toml"),
            runs_dir: lumirix_dir.join("runs"),
            db: db_dir.join("lumirix.sqlite"),
            cache_dir: lumirix_dir.join("cache"),
            snapshots_dir: lumirix_dir.join("snapshots"),
            artifacts_dir: lumirix_dir.join("artifacts"),
            policies_dir,
            db_dir,
            lumirix_dir,
            root,
        }
    }

    /// Whether Lumirix has been initialized (config file present).
    pub fn is_initialized(&self) -> bool {
        self.config.is_file()
    }

    /// Project root as a path reference.
    pub fn root(&self) -> &Path {
        &self.root
    }
}
