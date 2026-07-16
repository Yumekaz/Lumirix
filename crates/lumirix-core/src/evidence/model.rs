//! Evidence and test-record types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    None,
    NotNeeded,
    Weak,
    Failed,
    Medium,
    Strong,
}

impl EvidenceLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NotNeeded => "not_needed",
            Self::Weak => "weak",
            Self::Failed => "failed",
            Self::Medium => "medium",
            Self::Strong => "strong",
        }
    }

    pub fn display_title(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::NotNeeded => "Not needed",
            Self::Weak => "Weak",
            Self::Failed => "Failed",
            Self::Medium => "Medium",
            Self::Strong => "Strong",
        }
    }
}

impl std::fmt::Display for EvidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestRecord {
    pub id: String,
    pub command: String,
    pub argv: Vec<String>,
    pub kind: String,
    pub exit_code: Option<i32>,
    pub result: String,
    pub stdout_path: String,
    pub stderr_path: String,
    pub detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestsFile {
    pub run_id: String,
    pub tests: Vec<TestRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceReport {
    pub run_id: String,
    pub level: EvidenceLevel,
    pub reason: String,
    pub tests_detected: bool,
    pub tests_passed: bool,
    pub relevant_tests: bool,
    #[serde(default)]
    pub sensitive_areas: Vec<String>,
    #[serde(default)]
    pub matched_keywords: Vec<String>,
    #[serde(default)]
    pub tests: Vec<TestRecord>,
}
