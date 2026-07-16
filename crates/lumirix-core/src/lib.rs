//! Lumirix core — deterministic trust infrastructure primitives.

pub mod config;
pub mod db;
pub mod git;
pub mod init;
pub mod paths;
pub mod policy_default;
pub mod risk;
pub mod run;

pub use config::{Config, ConfigError};
pub use git::{detect_git, GitInfo};
pub use init::{format_init_message, init_project, InitError, InitResult};
pub use paths::LumirixPaths;
pub use risk::{evaluate_risks, format_risks_report, RiskFinding, RiskLevel, RiskReport};
pub use run::{
    execute_run, format_diff_report, format_minimal_report, format_risks_for_run,
    format_run_list_line, format_show, load_all_runs, load_diff_summary, load_last,
    load_risk_report, load_run, list_run_ids, resolve_last_run_id, DiffSummary, ExecuteOutcome,
    RunError, RunOptions, RunRecord, RunStatus, StoreError,
};
