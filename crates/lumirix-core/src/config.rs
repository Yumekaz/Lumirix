//! Project configuration (`.lumirix/config.toml`).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::paths::LumirixPaths;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Lumirix is not initialized in this directory (missing .lumirix/config.toml). Run `lumirix init` first.")]
    NotInitialized,
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Full Lumirix project configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub project: ProjectConfig,
    pub git: GitConfig,
    pub tests: TestsConfig,
    pub risk: RiskConfig,
    pub llm: LlmConfig,
    pub privacy: PrivacyConfig,
    pub reports: ReportsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub package_manager: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitConfig {
    pub require_clean_worktree_before_run: bool,
    pub capture_diff: bool,
    pub generate_rollback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestsConfig {
    pub default_commands: Vec<String>,
    pub parse_junit: bool,
    pub coverage_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskConfig {
    pub default_threshold: String,
    pub fail_ci_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmConfig {
    pub enabled: bool,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyConfig {
    pub local_only: bool,
    pub redact_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportsConfig {
    pub format: Vec<String>,
}

impl Config {
    /// Default config for a project named `name` (spec §14).
    pub fn default_for_project(name: impl Into<String>) -> Self {
        Self {
            project: ProjectConfig {
                name: name.into(),
                language: String::new(),
                package_manager: String::new(),
            },
            git: GitConfig {
                require_clean_worktree_before_run: true,
                capture_diff: true,
                generate_rollback: true,
            },
            tests: TestsConfig {
                default_commands: vec!["npm test".to_string()],
                parse_junit: true,
                coverage_paths: vec!["coverage/lcov.info".to_string()],
            },
            risk: RiskConfig {
                default_threshold: "medium".to_string(),
                fail_ci_on: "high".to_string(),
            },
            llm: LlmConfig {
                enabled: false,
                provider: "none".to_string(),
            },
            privacy: PrivacyConfig {
                local_only: true,
                redact_secrets: true,
            },
            reports: ReportsConfig {
                format: vec!["markdown".to_string(), "json".to_string()],
            },
        }
    }

    /// Serialize to TOML string.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Parse from TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(s)?)
    }

    /// Load config from a file path.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let raw = fs::read_to_string(path)?;
        Self::from_toml_str(&raw)
    }

    /// Load config for the given Lumirix paths (requires init).
    pub fn load(paths: &LumirixPaths) -> Result<Self, ConfigError> {
        if !paths.is_initialized() {
            return Err(ConfigError::NotInitialized);
        }
        Self::load_from_path(&paths.config)
    }

    /// Write config to a file path.
    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_toml_string()?)?;
        Ok(())
    }

    /// Human-readable LLM status line.
    pub fn llm_status_label(&self) -> &'static str {
        if self.llm.enabled {
            "enabled"
        } else {
            "disabled"
        }
    }
}
