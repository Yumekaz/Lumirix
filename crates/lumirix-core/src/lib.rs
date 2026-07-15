//! Lumirix core — deterministic trust infrastructure primitives.

pub mod config;
pub mod db;
pub mod git;
pub mod init;
pub mod paths;
pub mod policy_default;
pub mod run;

pub use config::{Config, ConfigError};
pub use git::{detect_git, GitInfo};
pub use init::{format_init_message, init_project, InitError, InitResult};
pub use paths::LumirixPaths;
pub use run::{
    execute_run, format_minimal_report, format_run_list_line, format_show, load_all_runs,
    load_last, load_run, list_run_ids, resolve_last_run_id, ExecuteOutcome, RunError, RunRecord,
    RunStatus, StoreError,
};
