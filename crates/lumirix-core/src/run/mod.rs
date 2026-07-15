//! Agent run wrapper: execute, capture, list, and show runs.

mod id;
mod model;
mod process;
mod store;

use chrono::Local;
use thiserror::Error;

use crate::db::{self, DbError};
use crate::git;
use crate::paths::LumirixPaths;

pub use model::{RunEvent, RunRecord, RunStatus};
pub use store::{load_all_runs, load_last, load_run, list_run_ids, resolve_last_run_id, StoreError};

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Lumirix is not initialized in this directory. Run `lumirix init` first.")]
    NotInitialized,
    #[error("no command provided; usage: lumirix run -- <command> [args...]")]
    EmptyCommand,
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

/// Execute a command under Lumirix capture.
pub fn execute_run(
    paths: &LumirixPaths,
    argv: Vec<String>,
    task: Option<String>,
) -> Result<ExecuteOutcome, RunError> {
    if !paths.is_initialized() {
        return Err(RunError::NotInitialized);
    }
    if argv.is_empty() {
        return Err(RunError::EmptyCommand);
    }

    let agent_command = join_command(&argv);
    let agent_name = argv[0].clone();
    let git = git::detect_git(&paths.root);

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
        base_commit: git.commit.clone(),
        branch: git.branch.clone(),
        final_commit: None,
        status: RunStatus::Running,
        exit_code: None,
        task,
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
            // Still finish the run record for auditability.
            let msg = e.to_string();
            let _ = next_event(
                "command.ended",
                serde_json::json!({ "error": msg, "exit_code": null }),
            );
            let _ = next_event(
                "run.ended",
                serde_json::json!({ "status": "error" }),
            );
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
    next_event(
        "run.ended",
        serde_json::json!({ "status": status.as_str(), "exit_code": exit_code }),
    )?;

    record.ended_at = Some(ended_at);
    record.status = status;
    record.exit_code = exit_code;
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
    lines.push(format!("Directory: {}", run_dir.display()));
    lines.push(format!("Stdout: {}", run_dir.join("stdout.log").display()));
    lines.push(format!("Stderr: {}", run_dir.join("stderr.log").display()));
    lines.join("\n")
}

/// Minimal report (`lumirix report last`) — not a full trust report.
pub fn format_minimal_report(record: &RunRecord, paths: &LumirixPaths) -> String {
    let run_dir = paths.runs_dir.join(&record.run_id);
    let mut lines = vec![
        format!("Run: {}", record.run_id),
        format!("Status: {}", record.status),
        format!("Command: {}", record.agent_command),
    ];
    match record.exit_code {
        Some(0) => {
            lines.push("Exit code: 0".to_string());
            lines.push("Result: command ran and exited successfully.".to_string());
        }
        Some(code) => {
            lines.push(format!("Exit code: {code}"));
            lines.push("Result: command exited with a non-zero status.".to_string());
        }
        None => {
            lines.push("Exit code: (none)".to_string());
            lines.push("Result: command did not complete normally.".to_string());
        }
    }
    match &record.base_commit {
        Some(c) => lines.push(format!("Base commit: {c}")),
        None => lines.push("Base commit: (none)".to_string()),
    }
    lines.push(format!("Logs: {}", run_dir.join("stdout.log").display()));
    lines.join("\n")
}

/// One-line listing entry.
pub fn format_run_list_line(record: &RunRecord) -> String {
    let exit = record
        .exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{:<22}  {:<10}  exit={:<4}  {}",
        record.run_id,
        record.status.as_str(),
        exit,
        record.agent_command
    )
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
