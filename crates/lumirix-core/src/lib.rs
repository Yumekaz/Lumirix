//! Lumirix core — deterministic trust infrastructure primitives.

pub mod config;
pub mod db;
pub mod evidence;
pub mod git;
pub mod init;
pub mod paths;
pub mod policy_default;
pub mod report;
pub mod risk;
pub mod run;

pub use config::{Config, ConfigError};
pub use evidence::{
    evaluate_evidence, format_evidence_report, EvidenceLevel, EvidenceReport, TestRecord, TestsFile,
};
pub use git::{detect_git, is_worktree_clean, is_worktree_dirty, GitInfo};
pub use init::{format_init_message, init_project, InitError, InitResult};
pub use paths::LumirixPaths;
pub use report::{
    build_trust_report, render_markdown, render_text, write_report_artifacts, TrustReport, Verdict,
};
pub use risk::{evaluate_risks, format_risks_report, RiskFinding, RiskLevel, RiskReport};
pub use run::{
    default_rollback_dest, execute_run, export_rollback_patch, format_diff_report,
    format_evidence_for_run, format_minimal_report, format_risks_for_run, format_run_list_line,
    format_show, format_trust_report_md, format_trust_report_text, generate_trust_report,
    load_all_runs, load_diff_summary, load_last, load_risk_report, load_run, list_run_ids,
    resolve_last_run_id, DiffSummary, ExecuteOutcome, RollbackExport, RunError, RunOptions,
    RunRecord, RunStatus, StoreError,
};
