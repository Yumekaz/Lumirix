//! Run metadata types.

use serde::{Deserialize, Serialize};

/// Persisted run record (`.lumirix/runs/<id>/run.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRecord {
    pub run_id: String,
    pub repo_path: String,
    pub agent_command: String,
    pub agent_argv: Vec<String>,
    pub agent_name: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub base_commit: Option<String>,
    pub branch: Option<String>,
    pub final_commit: Option<String>,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub task: Option<String>,
    /// Git diff summary for this run (absent on older runs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_diff: Option<DiffSummary>,
    /// Overall risk level string (none/low/medium/high/critical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<String>,
    /// Full risk report (also written to risk.json).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<crate::risk::RiskReport>,
    /// Evidence strength (weak/medium/strong/…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_level: Option<String>,
    /// Full evidence report (also written to evidence.json).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<crate::evidence::EvidenceReport>,
    /// Trust recommendation from the report engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// Machine-readable diff capture summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffSummary {
    pub base_commit: Option<String>,
    pub dirty_before: bool,
    pub dirty_after: bool,
    pub files_changed: u32,
    pub lines_added: u32,
    pub lines_deleted: u32,
    pub files: Vec<FileDiffStat>,
    #[serde(default)]
    pub untracked: Vec<String>,
    pub diff_patch: bool,
    pub rollback_patch: bool,
    pub rollback_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDiffStat {
    pub path: String,
    pub added: u64,
    pub deleted: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Error,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One JSONL event line under `events.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub event_id: String,
    pub run_id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub data: serde_json::Value,
}
