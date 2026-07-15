//! Subprocess spawn with tee of stdout/stderr to console + log files.

use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("failed to spawn command: {0}")]
    Spawn(#[source] io::Error),
    #[error("io error while running command: {0}")]
    Io(#[from] io::Error),
    #[error("command produced no exit status")]
    NoStatus,
}

#[derive(Debug)]
pub struct ProcessResult {
    pub exit_code: i32,
}

/// Run `argv[0]` with `argv[1..]` in `cwd`, tee stdout/stderr to files and console.
pub fn run_teed(
    argv: &[String],
    cwd: &Path,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<ProcessResult, ProcessError> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| ProcessError::Spawn(io::Error::new(io::ErrorKind::InvalidInput, "empty argv")))?;

    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(ProcessError::Spawn)?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_path = stdout_path.to_path_buf();
    let stderr_path = stderr_path.to_path_buf();

    let out_handle = thread::spawn(move || {
        if let Some(mut pipe) = stdout {
            tee_stream(&mut pipe, &stdout_path, false)
        } else {
            Ok(())
        }
    });

    let err_handle = thread::spawn(move || {
        if let Some(mut pipe) = stderr {
            tee_stream(&mut pipe, &stderr_path, true)
        } else {
            Ok(())
        }
    });

    let status = child.wait()?;
    let _ = out_handle.join().map_err(|_| {
        io::Error::new(io::ErrorKind::Other, "stdout tee thread panicked")
    })??;
    let _ = err_handle.join().map_err(|_| {
        io::Error::new(io::ErrorKind::Other, "stderr tee thread panicked")
    })??;

    let exit_code = status.code().unwrap_or(if status.success() { 0 } else { 1 });
    Ok(ProcessResult { exit_code })
}

fn tee_stream(reader: &mut dyn Read, log_path: &Path, is_stderr: bool) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        file.flush()?;
        if is_stderr {
            let mut err = io::stderr().lock();
            err.write_all(&buf[..n])?;
            err.flush()?;
        } else {
            let mut out = io::stdout().lock();
            out.write_all(&buf[..n])?;
            out.flush()?;
        }
    }
    Ok(())
}
