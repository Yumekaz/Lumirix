//! Run ID generation: `run_YYYY_MM_DD_NNN`.

use std::fs;
use std::path::Path;

use chrono::Local;

/// Allocate the next run id for the current local date under `runs_dir`.
pub fn next_run_id(runs_dir: &Path) -> std::io::Result<String> {
    let date = Local::now().format("%Y_%m_%d");
    let prefix = format!("run_{date}_");

    let mut max_seq: u32 = 0;
    if runs_dir.is_dir() {
        for entry in fs::read_dir(runs_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if let Some(rest) = name.strip_prefix(&prefix) {
                if let Ok(n) = rest.parse::<u32>() {
                    max_seq = max_seq.max(n);
                }
            }
        }
    }

    Ok(format!("{prefix}{:03}", max_seq + 1))
}
