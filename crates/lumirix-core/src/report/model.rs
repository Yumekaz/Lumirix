//! Trust report data model.

use serde::{Deserialize, Serialize};

use crate::evidence::EvidenceReport;
use crate::risk::RiskReport;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustReport {
    pub run_id: String,
    pub generated_at: String,
    pub verdict: Verdict,
    pub run: RunSection,
    pub summary: String,
    pub changed_files: Vec<ChangedFileRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<RiskReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceReport>,
    pub rollback: RollbackSection,
    pub next_steps: Vec<String>,
    pub policy: String,
    pub artifacts: ArtifactsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verdict {
    pub risk: String,
    pub evidence: String,
    pub recommendation: String,
    pub recommendation_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunSection {
    pub command: String,
    pub agent_name: String,
    pub branch: Option<String>,
    pub base_commit: Option<String>,
    pub exit_code: Option<i32>,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFileRow {
    pub path: String,
    pub added: u64,
    pub deleted: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollbackSection {
    pub status: String,
    pub diff_patch: bool,
    pub rollback_patch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactsSection {
    pub run_dir: String,
    pub stdout: String,
    pub stderr: String,
    pub diff_patch: String,
    pub rollback_patch: String,
    pub risk_json: String,
    pub evidence_json: String,
    pub report_md: String,
    pub report_json: String,
}
