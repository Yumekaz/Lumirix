//! Agent run wrapper: execute, capture, list, and show runs.

mod diff;
mod id;
pub mod model;
mod process;
pub mod store;

use chrono::Local;
use thiserror::Error;

use crate::config::{Config, ConfigError};
use crate::db::{self, DbError};
use crate::evidence::{self, EvidenceReport, TestsFile};
use crate::git;
use crate::paths::LumirixPaths;
use crate::report::{self, TrustReport};
use crate::risk::{self, RiskReport};

pub use model::{DiffSummary, FileDiffStat, RunEvent, RunRecord, RunStatus};
pub use store::{
    load_all_runs, load_diff_summary, load_last, load_risk_report, load_run, list_run_ids,
    resolve_last_run_id, RunPaths, StoreError,
};

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Lumirix is not initialized in this directory. Run `lumirix init` first.")]
    NotInitialized,
    #[error("no command provided; usage: lumirix run -- <command> [args...]")]
    EmptyCommand,
    #[error(
        "Git worktree is dirty. Commit or stash changes, or re-run with --allow-dirty."
    )]
    DirtyWorktree,
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Process(#[from] process::ProcessError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Outcome of `lumirix run` (includes child exit code for process exit propagation).
#[derive(Debug)]
pub struct ExecuteOutcome {
    pub record: RunRecord,
    pub child_exit_code: i32,
}

/// Options for a captured run.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub task: Option<String>,
    pub allow_dirty: bool,
}

/// Execute a command under Lumirix capture.
pub fn execute_run(
    paths: &LumirixPaths,
    argv: Vec<String>,
    options: RunOptions,
) -> Result<ExecuteOutcome, RunError> {
    if !paths.is_initialized() {
        return Err(RunError::NotInitialized);
    }
    if argv.is_empty() {
        return Err(RunError::EmptyCommand);
    }

    let config = Config::load(paths)?;
    let agent_command = join_command(&argv);
    let agent_name = argv[0].clone();
    let git_info = git::detect_git(&paths.root);

    let dirty_before = if git_info.is_repo {
        git::is_worktree_dirty(&paths.root)
    } else {
        false
    };

    if git_info.is_repo
        && config.git.require_clean_worktree_before_run
        && !options.allow_dirty
        && dirty_before
    {
        return Err(RunError::DirtyWorktree);
    }

    let run_id = id::next_run_id(&paths.runs_dir)?;
    let run_paths = store::RunPaths::new(&paths.runs_dir, &run_id);
    store::create_run_dir(&run_paths)?;
    store::write_commands_log(&run_paths, &agent_command)?;

    let started_at = now_iso();
    let mut record = RunRecord {
        run_id: run_id.clone(),
        repo_path: paths.root.display().to_string(),
        agent_command: agent_command.clone(),
        agent_argv: argv.clone(),
        agent_name,
        started_at: started_at.clone(),
        ended_at: None,
        base_commit: git_info.commit.clone(),
        branch: git_info.branch.clone(),
        final_commit: None,
        status: RunStatus::Running,
        exit_code: None,
        task: options.task,
        git_diff: None,
        risk_level: None,
        risk: None,
        evidence_level: None,
        evidence: None,
        recommendation: None,
    };
    store::write_run_json(&run_paths, &record)?;

    let mut event_n = 0u32;
    let mut next_event = |ty: &str, data: serde_json::Value| -> Result<(), RunError> {
        event_n += 1;
        let event = RunEvent {
            event_id: format!("evt_{event_n:03}"),
            run_id: run_id.clone(),
            timestamp: now_iso(),
            event_type: ty.to_string(),
            data,
        };
        store::append_event(&run_paths, &event)?;
        Ok(())
    };

    next_event(
        "run.started",
        serde_json::json!({
            "command": agent_command,
            "cwd": paths.root.display().to_string(),
            "dirty_before": dirty_before,
            "allow_dirty": options.allow_dirty,
        }),
    )?;
    next_event(
        "command.started",
        serde_json::json!({
            "command": agent_command,
            "argv": argv,
            "cwd": paths.root.display().to_string(),
        }),
    )?;

    let process_result = process::run_teed(
        &argv,
        &paths.root,
        &run_paths.stdout,
        &run_paths.stderr,
    );

    let ended_at = now_iso();
    let (status, exit_code, child_exit) = match process_result {
        Ok(r) => {
            let status = if r.exit_code == 0 {
                RunStatus::Completed
            } else {
                RunStatus::Failed
            };
            (status, Some(r.exit_code), r.exit_code)
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = next_event(
                "command.ended",
                serde_json::json!({ "error": msg, "exit_code": null }),
            );
            // Still attempt diff capture for partial audit.
            if git_info.is_repo && config.git.capture_diff {
                if let Ok(summary) = diff::capture_diff_for_run(
                    &paths.root,
                    &run_paths,
                    git_info.commit.clone(),
                    dirty_before,
                    config.git.generate_rollback,
                ) {
                    let _ = next_event(
                        "git.diff.captured",
                        serde_json::json!({
                            "files_changed": summary.files_changed,
                            "rollback_status": summary.rollback_status,
                        }),
                    );
                    record.git_diff = Some(summary);
                }
            } else if !git_info.is_repo {
                record.git_diff = Some(diff::no_git_summary(dirty_before));
            }
            let _ = apply_risk_evaluation(&mut record, &run_paths, &mut next_event);
            let _ = apply_evidence_evaluation(&mut record, &run_paths, None, &mut next_event);
            let _ = next_event("run.ended", serde_json::json!({ "status": "error" }));
            record.ended_at = Some(ended_at.clone());
            record.status = RunStatus::Error;
            record.exit_code = None;
            store::write_run_json(&run_paths, &record)?;
            let _ = db::upsert_run(
                &paths.db,
                &record.run_id,
                &record.started_at,
                record.ended_at.as_deref(),
                &record.agent_command,
                record.exit_code,
                record.base_commit.as_deref(),
                record.status.as_str(),
            );
            return Err(RunError::Process(e));
        }
    };

    next_event(
        "command.ended",
        serde_json::json!({ "exit_code": exit_code }),
    )?;

    if git_info.is_repo && config.git.capture_diff {
        let summary = diff::capture_diff_for_run(
            &paths.root,
            &run_paths,
            git_info.commit.clone(),
            dirty_before,
            config.git.generate_rollback,
        )?;
        next_event(
            "git.diff.captured",
            serde_json::json!({
                "files_changed": summary.files_changed,
                "lines_added": summary.lines_added,
                "lines_deleted": summary.lines_deleted,
                "rollback_status": summary.rollback_status,
            }),
        )?;
        record.git_diff = Some(summary);
    } else if !git_info.is_repo {
        record.git_diff = Some(diff::no_git_summary(dirty_before));
    }

    apply_risk_evaluation(&mut record, &run_paths, &mut next_event)?;
    apply_evidence_evaluation(&mut record, &run_paths, exit_code, &mut next_event)?;

    record.ended_at = Some(ended_at.clone());
    record.status = status;
    record.exit_code = exit_code;

    apply_trust_report(&mut record, paths, &run_paths, &mut next_event)?;

    next_event(
        "run.ended",
        serde_json::json!({ "status": status.as_str(), "exit_code": exit_code }),
    )?;

    store::write_run_json(&run_paths, &record)?;

    db::upsert_run(
        &paths.db,
        &record.run_id,
        &record.started_at,
        record.ended_at.as_deref(),
        &record.agent_command,
        record.exit_code,
        record.base_commit.as_deref(),
        record.status.as_str(),
    )?;

    Ok(ExecuteOutcome {
        record,
        child_exit_code: child_exit,
    })
}

/// Human-readable detailed view (`lumirix show`).
pub fn format_show(record: &RunRecord, paths: &LumirixPaths) -> String {
    let run_dir = paths.runs_dir.join(&record.run_id);
    let mut lines = vec![
        format!("Run: {}", record.run_id),
        format!("Status: {}", record.status),
        format!("Command: {}", record.agent_command),
    ];
    if let Some(code) = record.exit_code {
        lines.push(format!("Exit code: {code}"));
    } else {
        lines.push("Exit code: (none)".to_string());
    }
    lines.push(format!("Started: {}", record.started_at));
    if let Some(ref e) = record.ended_at {
        lines.push(format!("Ended: {e}"));
    }
    match &record.branch {
        Some(b) => lines.push(format!("Branch: {b}")),
        None => lines.push("Branch: (none)".to_string()),
    }
    match &record.base_commit {
        Some(c) => lines.push(format!("Base commit: {c}")),
        None => lines.push("Base commit: (none)".to_string()),
    }
    if let Some(ref t) = record.task {
        lines.push(format!("Task: {t}"));
    }
    append_diff_lines(&mut lines, record.git_diff.as_ref());
    append_risk_lines(&mut lines, record.risk.as_ref(), false);
    append_evidence_lines(&mut lines, record.evidence.as_ref(), false);
    lines.push(format!("Directory: {}", run_dir.display()));
    lines.push(format!("Stdout: {}", run_dir.join("stdout.log").display()));
    lines.push(format!("Stderr: {}", run_dir.join("stderr.log").display()));
    if run_dir.join("diff.patch").is_file() {
        lines.push(format!("Diff patch: {}", run_dir.join("diff.patch").display()));
    }
    if run_dir.join("rollback.patch").is_file() {
        lines.push(format!(
            "Rollback patch: {}",
            run_dir.join("rollback.patch").display()
        ));
    }
    lines.join("\n")
}

/// Full trust report as terminal text (builds from record).
pub fn format_trust_report_text(record: &RunRecord, paths: &LumirixPaths) -> String {
    let report = report::build_trust_report(record, paths);
    report::render_text(&report)
}

/// Full trust report as Markdown.
pub fn format_trust_report_md(record: &RunRecord, paths: &LumirixPaths) -> String {
    let report = report::build_trust_report(record, paths);
    report::render_markdown(&report)
}

/// Build trust report, write artifacts, return report.
pub fn generate_trust_report(
    record: &RunRecord,
    paths: &LumirixPaths,
) -> Result<TrustReport, RunError> {
    let run_paths = store::RunPaths::new(&paths.runs_dir, &record.run_id);
    let report = report::build_trust_report(record, paths);
    report::write_report_artifacts(&run_paths, &report)?;
    Ok(report)
}

/// Backward-compatible name: terminal trust report.
pub fn format_minimal_report(record: &RunRecord, paths: &LumirixPaths) -> String {
    format_trust_report_text(record, paths)
}

/// `lumirix evidence last` output.
pub fn format_evidence_for_run(record: &RunRecord) -> String {
    if let Some(ref e) = record.evidence {
        return evidence::format_evidence_report(e);
    }
    format!(
        "Evidence: {}\nReason: evidence not captured for this run",
        record.evidence_level.as_deref().unwrap_or("none")
    )
}

/// `lumirix risks last` output.
pub fn format_risks_for_run(record: &RunRecord) -> String {
    if let Some(ref r) = record.risk {
        return risk::format_risks_report(r);
    }
    format!(
        "Overall risk: {}\nFindings: none (risk not captured for this run)",
        record
            .risk_level
            .as_deref()
            .unwrap_or("none")
    )
}

/// `lumirix diff last` output (success-criteria style).
pub fn format_diff_report(record: &RunRecord, paths: &LumirixPaths) -> String {
    let run_dir = paths.runs_dir.join(&record.run_id);
    let mut lines = vec![format!("Run: {}", record.run_id)];

    if let Some(ref d) = record.git_diff {
        lines.push(format!("Files changed: {}", d.files_changed));
        lines.push(format!("Lines added: {}", d.lines_added));
        lines.push(format!("Lines deleted: {}", d.lines_deleted));
        let rb = match d.rollback_status.as_str() {
            "available" => "available",
            "partial" => "partial",
            "no_changes" => "not needed (no changes)",
            "no_git" => "unavailable (no git)",
            "disabled" => "disabled",
            _ => "unavailable",
        };
        lines.push(format!("Rollback patch: {rb}"));
        if !d.files.is_empty() {
            lines.push("Files:".to_string());
            for f in &d.files {
                lines.push(format!("  {} (+{} -{})", f.path, f.added, f.deleted));
            }
        }
        if !d.untracked.is_empty() {
            lines.push("Untracked (not in patch):".to_string());
            for u in &d.untracked {
                lines.push(format!("  {u}"));
            }
        }
        lines.push(format!("dirty_before: {}", d.dirty_before));
        lines.push(format!("dirty_after: {}", d.dirty_after));
    } else {
        lines.push("Files changed: (not captured)".to_string());
        lines.push("Rollback patch: unavailable".to_string());
    }

    if run_dir.join("diff.patch").is_file() {
        lines.push(format!("Diff patch: {}", run_dir.join("diff.patch").display()));
    }
    if run_dir.join("rollback.patch").is_file() {
        lines.push(format!(
            "Rollback patch file: {}",
            run_dir.join("rollback.patch").display()
        ));
    }
    lines.join("\n")
}

/// One-line listing entry.
pub fn format_run_list_line(record: &RunRecord) -> String {
    let exit = record
        .exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "-".to_string());
    let files = record
        .git_diff
        .as_ref()
        .map(|d| d.files_changed.to_string())
        .unwrap_or_else(|| "-".to_string());
    let risk = record
        .risk_level
        .as_deref()
        .unwrap_or("-");
    format!(
        "{:<22}  {:<10}  exit={:<4}  files={:<4}  risk={:<8}  {}",
        record.run_id,
        record.status.as_str(),
        exit,
        files,
        risk,
        record.agent_command
    )
}

fn apply_risk_evaluation<F>(
    record: &mut RunRecord,
    run_paths: &store::RunPaths,
    next_event: &mut F,
) -> Result<(), RunError>
where
    F: FnMut(&str, serde_json::Value) -> Result<(), RunError>,
{
    let mut changed_paths: Vec<String> = Vec::new();
    if let Some(ref d) = record.git_diff {
        for f in &d.files {
            changed_paths.push(f.path.clone());
        }
        for u in &d.untracked {
            changed_paths.push(u.clone());
        }
    }
    let report = risk::evaluate_risks(
        &record.run_id,
        &record.agent_command,
        &record.agent_argv,
        &changed_paths,
    );
    store::write_risk_report(run_paths, &report)?;
    next_event(
        "risk.evaluated",
        serde_json::json!({
            "overall_level": report.overall_level.as_str(),
            "findings": report.findings.len(),
        }),
    )?;
    if !report.findings.is_empty() {
        next_event(
            "risk.detected",
            serde_json::json!({
                "overall_level": report.overall_level.as_str(),
                "findings": report.findings.len(),
            }),
        )?;
    }
    record.risk_level = Some(report.overall_level.as_str().to_string());
    record.risk = Some(report);
    Ok(())
}

fn apply_evidence_evaluation<F>(
    record: &mut RunRecord,
    run_paths: &store::RunPaths,
    exit_code: Option<i32>,
    next_event: &mut F,
) -> Result<(), RunError>
where
    F: FnMut(&str, serde_json::Value) -> Result<(), RunError>,
{
    let mut changed_paths: Vec<String> = Vec::new();
    if let Some(ref d) = record.git_diff {
        for f in &d.files {
            changed_paths.push(f.path.clone());
        }
        for u in &d.untracked {
            changed_paths.push(u.clone());
        }
    }

    let report = evidence::evaluate_evidence(
        &record.run_id,
        &record.agent_command,
        &record.agent_argv,
        exit_code,
        &changed_paths,
    );

    let tests_file = TestsFile {
        run_id: record.run_id.clone(),
        tests: report.tests.clone(),
    };
    store::write_tests_file(run_paths, &tests_file)?;
    store::write_evidence_report(run_paths, &report)?;

    if report.tests_detected {
        next_event(
            "test.detected",
            serde_json::json!({
                "count": report.tests.len(),
                "kinds": report.tests.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(),
            }),
        )?;
        if let Some(t) = report.tests.first() {
            next_event(
                "test.ended",
                serde_json::json!({
                    "result": t.result,
                    "exit_code": t.exit_code,
                }),
            )?;
        }
    }

    next_event(
        "evidence.evaluated",
        serde_json::json!({
            "level": report.level.as_str(),
            "reason": report.reason,
            "tests_detected": report.tests_detected,
        }),
    )?;

    // Polish auth risk notes with real evidence text when available.
    if let Some(ref mut risk) = record.risk {
        for f in &mut risk.findings {
            if f.category == "auth_change" {
                f.evidence_note = Some(format!(
                    "Evidence: {} — {}",
                    report.level.display_title(),
                    report.reason
                ));
            }
        }
        let _ = store::write_risk_report(run_paths, risk);
    }

    record.evidence_level = Some(report.level.as_str().to_string());
    record.evidence = Some(report);
    Ok(())
}

fn apply_trust_report<F>(
    record: &mut RunRecord,
    paths: &LumirixPaths,
    run_paths: &store::RunPaths,
    next_event: &mut F,
) -> Result<(), RunError>
where
    F: FnMut(&str, serde_json::Value) -> Result<(), RunError>,
{
    let trust = report::build_trust_report(record, paths);
    report::write_report_artifacts(run_paths, &trust)?;
    next_event(
        "report.generated",
        serde_json::json!({
            "recommendation_code": trust.verdict.recommendation_code,
            "risk": trust.verdict.risk,
            "evidence": trust.verdict.evidence,
        }),
    )?;
    record.recommendation = Some(trust.verdict.recommendation);
    Ok(())
}

fn append_diff_lines(lines: &mut Vec<String>, summary: Option<&DiffSummary>) {
    if let Some(d) = summary {
        lines.push(format!("Files changed: {}", d.files_changed));
        lines.push(format!("Lines added: {}", d.lines_added));
        lines.push(format!("Lines deleted: {}", d.lines_deleted));
        let rb = match d.rollback_status.as_str() {
            "available" => "available",
            "partial" => "partial",
            "no_changes" => "not needed",
            "no_git" => "unavailable (no git)",
            "disabled" => "disabled",
            _ => "unavailable",
        };
        lines.push(format!("Rollback patch: {rb}"));
    }
}

fn append_risk_lines(lines: &mut Vec<String>, report: Option<&RiskReport>, detailed: bool) {
    if let Some(r) = report {
        lines.push(format!("Risk: {}", r.overall_level.display_title()));
        if detailed {
            for f in &r.findings {
                lines.push(format!(
                    "  [{}] {} — {}",
                    f.severity.as_str(),
                    f.category,
                    f.message
                ));
                if let Some(ref note) = f.evidence_note {
                    lines.push(format!("    {note}"));
                }
            }
        } else if !r.findings.is_empty() {
            lines.push(format!("Risk findings: {}", r.findings.len()));
        }
    }
}

fn append_evidence_lines(lines: &mut Vec<String>, report: Option<&EvidenceReport>, detailed: bool) {
    if let Some(e) = report {
        lines.push(format!("Evidence: {}", e.level.display_title()));
        if detailed {
            lines.push(format!("Reason: {}", e.reason));
            if e.tests.is_empty() {
                lines.push("Tests: (none detected)".to_string());
            } else {
                for t in &e.tests {
                    lines.push(format!("Tests: {} → {}", t.command, t.result));
                }
            }
        }
    }
}

fn join_command(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.contains(char::is_whitespace) {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn now_iso() -> String {
    Local::now().to_rfc3339()
}
