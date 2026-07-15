//! Lumirix core — deterministic trust infrastructure primitives.
//!
//! Phase 1: paths, config, git detection, init, SQLite bootstrap.

pub mod config;
pub mod db;
pub mod git;
pub mod init;
pub mod paths;
pub mod policy_default;

pub use config::{Config, ConfigError};
pub use git::{detect_git, GitInfo};
pub use init::{format_init_message, init_project, InitError, InitResult};
pub use paths::LumirixPaths;
