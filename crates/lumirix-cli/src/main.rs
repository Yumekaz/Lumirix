//! Lumirix CLI — init, status, config, run capture, diffs, risks, evidence, reports, rollback.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use lumirix_core::{
    default_rollback_dest, detect_git, execute_run, export_rollback_patch, format_diff_report,
    format_evidence_for_run, format_init_message, format_risks_for_run, format_run_list_line,
    format_show, format_trust_report_md, format_trust_report_text, generate_trust_report,
    init_project, is_worktree_dirty, load_all_runs, load_last, load_run, resolve_last_run_id,
    Config, LumirixPaths, RunError, RunOptions, StoreError,
};

#[derive(Parser, Debug)]
#[command(
    name = "lumirix",
    version,
    about = "Trust infrastructure for autonomous coding agents",
    long_about = "Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.\n\nWrap agent commands, capture runs/diffs, detect risks and test evidence, and inspect trust reports."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize Lumirix in the current directory
    Init {
        /// Overwrite existing `.lumirix/` defaults
        #[arg(long)]
        force: bool,
    },
    /// Show Lumirix and Git status for the current directory
    Status,
    /// Config-related commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Run a command under Lumirix capture
    Run {
        /// Allow running with a dirty Git worktree
        #[arg(long)]
        allow_dirty: bool,
        /// Optional task description stored with the run
        #[arg(long)]
        task: Option<String>,
        /// Command and arguments (use `--` before the program)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
    /// List captured runs (newest first)
    Runs,
    /// Show metadata for a run (`last` or run id)
    Show {
        /// Run id, or `last` (default: last)
        #[arg(default_value = "last")]
        run: String,
    },
    /// Print a trust report for a run (`last` or run id)
    Report {
        /// Output format: text (default), md, or json
        #[arg(long, default_value = "text")]
        format: String,
        /// Run id, or `last` (default: last)
        #[arg(default_value = "last")]
        run: String,
    },
    /// Show Git diff summary for a run (`last` or run id)
    Diff {
        /// Run id, or `last` (default: last)
        #[arg(default_value = "last")]
        run: String,
    },
    /// Show risk findings for a run (`last` or run id)
    Risks {
        /// Run id, or `last` (default: last)
        #[arg(default_value = "last")]
        run: String,
    },
    /// Show test evidence for a run (`last` or run id)
    Evidence {
        /// Run id, or `last` (default: last)
        #[arg(default_value = "last")]
        run: String,
    },
    /// Export rollback.patch for a run
    Rollback {
        /// Write patch to this path (default: ./rollback.patch)
        #[arg(long, value_name = "PATH", default_value = "rollback.patch")]
        write: PathBuf,
        /// Run id, or `last` (default: last)
        #[arg(default_value = "last")]
        run: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Print the project `.lumirix/config.toml`
    Show,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let cwd = env::current_dir().context("failed to get current directory")?;

    match cli.command {
        Commands::Init { force } => {
            cmd_init(&cwd, force)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Status => {
            cmd_status(&cwd)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Config {
            command: ConfigCommands::Show,
        } => {
            cmd_config_show(&cwd)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Run {
            allow_dirty,
            task,
            args,
        } => cmd_run(&cwd, args, task, allow_dirty),
        Commands::Runs => {
            cmd_runs(&cwd)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Show { run } => {
            cmd_show(&cwd, &run)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Report { format, run } => {
            cmd_report(&cwd, &run, &format)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Diff { run } => {
            cmd_diff(&cwd, &run)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Risks { run } => {
            cmd_risks(&cwd, &run)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Evidence { run } => {
            cmd_evidence(&cwd, &run)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Rollback { write, run } => {
            cmd_rollback(&cwd, &run, write)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn cmd_init(cwd: &PathBuf, force: bool) -> Result<()> {
    let result = init_project(cwd, force).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", format_init_message(&result));
    Ok(())
}

fn cmd_status(cwd: &PathBuf) -> Result<()> {
    let paths = LumirixPaths::new(cwd);

    if !paths.is_initialized() {
        bail!("Lumirix is not initialized in this directory. Run `lumirix init` first.");
    }

    let config = Config::load(&paths).map_err(|e| anyhow::anyhow!("{e}"))?;
    let git = detect_git(cwd);

    println!("Lumirix initialized.");

    if git.is_repo {
        println!("Git repo detected.");
        match &git.branch {
            Some(b) => println!("Current branch: {b}"),
            None => println!("Current branch: (detached HEAD)"),
        }
        match &git.commit {
            Some(c) => println!("Current commit: {c}"),
            None => println!("Current commit: (unknown)"),
        }
        let dirty = is_worktree_dirty(cwd);
        if dirty {
            println!("Worktree: dirty (uncommitted changes)");
            println!("  tip: commit/stash for clean attribution, or use --allow-dirty on run");
        } else {
            println!("Worktree: clean");
        }
    } else {
        println!("Git repo: not detected (limited mode).");
    }

    println!("LLM: {}", config.llm_status_label());
    if config.git.require_clean_worktree_before_run {
        println!("require_clean_worktree_before_run: true");
    }
    Ok(())
}

fn cmd_config_show(cwd: &PathBuf) -> Result<()> {
    let paths = LumirixPaths::new(cwd);

    if !paths.is_initialized() {
        bail!("Lumirix is not initialized in this directory. Run `lumirix init` first.");
    }

    let raw = std::fs::read_to_string(&paths.config)
        .with_context(|| format!("failed to read {}", paths.config.display()))?;
    print!("{raw}");
    if !raw.ends_with('\n') {
        println!();
    }
    Ok(())
}

fn cmd_run(
    cwd: &PathBuf,
    args: Vec<String>,
    task: Option<String>,
    allow_dirty: bool,
) -> Result<ExitCode> {
    let paths = LumirixPaths::new(cwd);
    let outcome = execute_run(
        &paths,
        args,
        RunOptions {
            task,
            allow_dirty,
        },
    )
    .map_err(map_run_error)?;

    let files = outcome
        .record
        .git_diff
        .as_ref()
        .map(|d| d.files_changed)
        .unwrap_or(0);
    let risk = outcome.record.risk_level.as_deref().unwrap_or("-");
    let evidence = outcome.record.evidence_level.as_deref().unwrap_or("-");

    eprintln!(
        "lumirix: run {} {} (exit {}, files={}, risk={}, evidence={})",
        outcome.record.run_id,
        outcome.record.status,
        outcome.child_exit_code,
        files,
        risk,
        evidence
    );
    if let Some(ref rec) = outcome.record.recommendation {
        eprintln!("lumirix: recommendation: {rec}");
    }

    Ok(exit_code_from_i32(outcome.child_exit_code))
}

fn cmd_runs(cwd: &PathBuf) -> Result<()> {
    let paths = require_init(cwd)?;
    let runs = load_all_runs(&paths).map_err(map_store_error)?;
    if runs.is_empty() {
        println!("No runs recorded yet.");
        return Ok(());
    }
    for r in runs {
        println!("{}", format_run_list_line(&r));
    }
    Ok(())
}

fn cmd_show(cwd: &PathBuf, run: &str) -> Result<()> {
    let paths = require_init(cwd)?;
    let record = load_run_ref(&paths, run)?;
    println!("{}", format_show(&record, &paths));
    Ok(())
}

fn cmd_report(cwd: &PathBuf, run: &str, format: &str) -> Result<()> {
    let paths = require_init(cwd)?;
    let mut record = load_run_ref(&paths, run)?;
    let trust = generate_trust_report(&record, &paths).map_err(map_run_error)?;
    record.recommendation = Some(trust.verdict.recommendation.clone());

    match format.to_lowercase().as_str() {
        "md" | "markdown" => print!("{}", format_trust_report_md(&record, &paths)),
        "json" => {
            let json = serde_json::to_string_pretty(&trust)
                .context("failed to serialize trust report")?;
            println!("{json}");
        }
        "text" | "terminal" | _ => print!("{}", format_trust_report_text(&record, &paths)),
    }
    Ok(())
}

fn cmd_diff(cwd: &PathBuf, run: &str) -> Result<()> {
    let paths = require_init(cwd)?;
    let record = load_run_ref(&paths, run)?;
    println!("{}", format_diff_report(&record, &paths));
    Ok(())
}

fn cmd_risks(cwd: &PathBuf, run: &str) -> Result<()> {
    let paths = require_init(cwd)?;
    let record = load_run_ref(&paths, run)?;
    println!("{}", format_risks_for_run(&record));
    Ok(())
}

fn cmd_evidence(cwd: &PathBuf, run: &str) -> Result<()> {
    let paths = require_init(cwd)?;
    let record = load_run_ref(&paths, run)?;
    println!("{}", format_evidence_for_run(&record));
    Ok(())
}

fn cmd_rollback(cwd: &PathBuf, run: &str, write: PathBuf) -> Result<()> {
    let paths = require_init(cwd)?;
    let run_id = if run == "last" {
        resolve_last_run_id(&paths).map_err(map_store_error)?
    } else {
        run.to_string()
    };
    let dest = if write.as_os_str().is_empty() {
        default_rollback_dest(cwd)
    } else if write.is_relative() {
        cwd.join(write)
    } else {
        write
    };

    let exported = export_rollback_patch(&paths, &run_id, &dest).map_err(map_store_error)?;
    println!("Run: {}", exported.run_id);
    println!("Source: {}", exported.source.display());
    println!("Wrote: {} ({} bytes)", exported.dest.display(), exported.bytes);
    if exported.empty {
        println!("Warning: patch is empty.");
    }
    println!("{}", exported.status_note);
    Ok(())
}

fn load_run_ref(paths: &LumirixPaths, run: &str) -> Result<lumirix_core::RunRecord> {
    if run == "last" {
        load_last(paths).map_err(map_store_error)
    } else {
        load_run(paths, run).map_err(map_store_error)
    }
}

fn require_init(cwd: &PathBuf) -> Result<LumirixPaths> {
    let paths = LumirixPaths::new(cwd);
    if !paths.is_initialized() {
        bail!("Lumirix is not initialized in this directory. Run `lumirix init` first.");
    }
    Ok(paths)
}

fn map_run_error(e: RunError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

fn map_store_error(e: StoreError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

fn exit_code_from_i32(code: i32) -> ExitCode {
    if code == 0 {
        ExitCode::SUCCESS
    } else if (1..=255).contains(&code) {
        ExitCode::from(code as u8)
    } else {
        ExitCode::FAILURE
    }
}
