//! Lumirix CLI — init, status, config, run capture, diffs, and inspection.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use lumirix_core::{
    detect_git, execute_run, format_diff_report, format_init_message, format_minimal_report,
    format_risks_for_run, format_run_list_line, format_show, init_project, load_all_runs,
    load_last, load_run, Config, LumirixPaths, RunError, RunOptions, StoreError,
};

#[derive(Parser, Debug)]
#[command(
    name = "lumirix",
    version,
    about = "Trust infrastructure for autonomous coding agents",
    long_about = "Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.\n\nWrap agent commands, capture runs and Git diffs, detect basic risks, and inspect results."
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
    /// Print a minimal report for a run (`last` or run id)
    Report {
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
        Commands::Report { run } => {
            cmd_report(&cwd, &run)?;
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
    } else {
        println!("Git repo: not detected (limited mode).");
    }

    println!("LLM: {}", config.llm_status_label());
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

    eprintln!(
        "lumirix: run {} {} (exit {}, files changed {})",
        outcome.record.run_id, outcome.record.status, outcome.child_exit_code, files
    );

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

fn cmd_report(cwd: &PathBuf, run: &str) -> Result<()> {
    let paths = require_init(cwd)?;
    let record = load_run_ref(&paths, run)?;
    println!("{}", format_minimal_report(&record, &paths));
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
