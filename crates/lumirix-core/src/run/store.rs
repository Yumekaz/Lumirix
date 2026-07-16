//! Filesystem store for run artifacts.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::model::{DiffSummary, RunEvent, RunRecord};
use crate::paths::LumirixPaths;
use crate::risk::RiskReport;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("run not found: {0}")]
    NotFound(String),
    #[error("no runs recorded yet")]
    NoRuns,
}

/// Paths for a single run directory.
#[derive(Debug, Clone)]
pub struct RunPaths {
    pub dir: PathBuf,
    pub run_json: PathBuf,
    pub events: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub commands: PathBuf,
    pub diff_patch: PathBuf,
    pub rollback_patch: PathBuf,
    pub diff_summary: PathBuf,
    pub risk_json: PathBuf,
}

impl RunPaths {
    pub fn new(runs_dir: &Path, run_id: &str) -> Self {
        let dir = runs_dir.join(run_id);
        Self {
            run_json: dir.join("run.json"),
            events: dir.join("events.jsonl"),
            stdout: dir.join("stdout.log"),
            stderr: dir.join("stderr.log"),
            commands: dir.join("commands.log"),
            diff_patch: dir.join("diff.patch"),
            rollback_patch: dir.join("rollback.patch"),
            diff_summary: dir.join("diff_summary.json"),
            risk_json: dir.join("risk.json"),
            dir,
        }
    }
}

/// Create run directory and empty log files.
pub fn create_run_dir(paths: &RunPaths) -> Result<(), StoreError> {
    fs::create_dir_all(&paths.dir)?;
    File::create(&paths.stdout)?;
    File::create(&paths.stderr)?;
    File::create(&paths.events)?;
    File::create(&paths.commands)?;
    Ok(())
}

pub fn write_run_json(paths: &RunPaths, record: &RunRecord) -> Result<(), StoreError> {
    let json = serde_json::to_string_pretty(record)?;
    fs::write(&paths.run_json, format!("{json}\n"))?;
    Ok(())
}

pub fn append_event(paths: &RunPaths, event: &RunEvent) -> Result<(), StoreError> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.events)?;
    let line = serde_json::to_string(event)?;
    writeln!(f, "{line}")?;
    Ok(())
}

pub fn write_commands_log(paths: &RunPaths, command: &str) -> Result<(), StoreError> {
    fs::write(&paths.commands, format!("{command}\n"))?;
    Ok(())
}

pub fn write_text(path: &Path, content: &str) -> Result<(), StoreError> {
    fs::write(path, content)?;
    Ok(())
}

pub fn write_diff_summary(paths: &RunPaths, summary: &DiffSummary) -> Result<(), StoreError> {
    let json = serde_json::to_string_pretty(summary)?;
    fs::write(&paths.diff_summary, format!("{json}\n"))?;
    Ok(())
}

pub fn load_diff_summary(paths: &LumirixPaths, run_id: &str) -> Result<Option<DiffSummary>, StoreError> {
    let run_paths = RunPaths::new(&paths.runs_dir, run_id);
    if !run_paths.diff_summary.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&run_paths.diff_summary)?;
    Ok(Some(serde_json::from_str(&raw)?))
}

pub fn write_risk_report(paths: &RunPaths, report: &RiskReport) -> Result<(), StoreError> {
    let json = serde_json::to_string_pretty(report)?;
    fs::write(&paths.risk_json, format!("{json}\n"))?;
    Ok(())
}

pub fn load_risk_report(paths: &LumirixPaths, run_id: &str) -> Result<Option<RiskReport>, StoreError> {
    let run_paths = RunPaths::new(&paths.runs_dir, run_id);
    if !run_paths.risk_json.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&run_paths.risk_json)?;
    Ok(Some(serde_json::from_str(&raw)?))
}

pub fn load_run(paths: &LumirixPaths, run_id: &str) -> Result<RunRecord, StoreError> {
    let run_paths = RunPaths::new(&paths.runs_dir, run_id);
    if !run_paths.run_json.is_file() {
        return Err(StoreError::NotFound(run_id.to_string()));
    }
    let raw = fs::read_to_string(&run_paths.run_json)?;
    Ok(serde_json::from_str(&raw)?)
}

/// List run ids newest-first (lexicographic on `run_YYYY_MM_DD_NNN` works chronologically).
pub fn list_run_ids(paths: &LumirixPaths) -> Result<Vec<String>, StoreError> {
    if !paths.runs_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut ids = Vec::new();
    for entry in fs::read_dir(&paths.runs_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with("run_") && entry.path().join("run.json").is_file() {
            ids.push(name.to_string());
        }
    }
    ids.sort();
    ids.reverse();
    Ok(ids)
}

pub fn resolve_last_run_id(paths: &LumirixPaths) -> Result<String, StoreError> {
    list_run_ids(paths)?
        .into_iter()
        .next()
        .ok_or(StoreError::NoRuns)
}

pub fn load_last(paths: &LumirixPaths) -> Result<RunRecord, StoreError> {
    let id = resolve_last_run_id(paths)?;
    load_run(paths, &id)
}

pub fn load_all_runs(paths: &LumirixPaths) -> Result<Vec<RunRecord>, StoreError> {
    let mut out = Vec::new();
    for id in list_run_ids(paths)? {
        out.push(load_run(paths, &id)?);
    }
    Ok(out)
}
