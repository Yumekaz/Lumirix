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
