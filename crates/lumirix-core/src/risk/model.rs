//! Risk report types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn display_title(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskFinding {
    pub id: String,
    pub severity: RiskLevel,
    pub category: String,
    pub message: String,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskReport {
    pub run_id: String,
    pub overall_level: RiskLevel,
    pub findings: Vec<RiskFinding>,
}

impl RiskReport {
    pub fn empty(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            overall_level: RiskLevel::None,
            findings: Vec::new(),
        }
    }
}
