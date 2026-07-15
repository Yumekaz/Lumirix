//! Lumirix CLI — Phase 1: init, status, config show.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use lumirix_core::{
    detect_git, format_init_message, init_project, Config, LumirixPaths,
};

#[derive(Parser, Debug)]
#[command(
    name = "lumirix",
    version,
    about = "Trust infrastructure for autonomous coding agents",
    long_about = "Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.\n\nPhase 1: project init, status, and config."
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
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Print the project `.lumirix/config.toml`
    Show,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir().context("failed to get current directory")?;

    match cli.command {
        Commands::Init { force } => cmd_init(&cwd, force),
        Commands::Status => cmd_status(&cwd),
        Commands::Config {
            command: ConfigCommands::Show,
        } => cmd_config_show(&cwd),
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
