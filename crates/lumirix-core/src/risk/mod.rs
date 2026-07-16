//! Basic risk engine: sensitive paths + dangerous command patterns.

mod model;
mod rules;

use std::collections::BTreeMap;

pub use model::{RiskFinding, RiskLevel, RiskReport};
pub use rules::{match_command, match_path, normalize_path};

/// Evaluate risks for a completed run (pure; no I/O).
///
/// `changed_paths` should include tracked + untracked paths from the run diff.
pub fn evaluate_risks(
    run_id: &str,
    agent_command: &str,
    _agent_argv: &[String],
    changed_paths: &[String],
) -> RiskReport {
    // category -> (hit metadata, paths, commands)
    struct Acc {
        severity: RiskLevel,
        category: String,
        message: String,
        evidence_note: Option<String>,
        paths: Vec<String>,
        commands: Vec<String>,
    }

    let mut by_key: BTreeMap<String, Acc> = BTreeMap::new();

    let mut add_path = |path: &str| {
        if let Some(hit) = match_path(path) {
            let key = format!("path:{}:{}", hit.category, hit.message);
            let entry = by_key.entry(key).or_insert_with(|| Acc {
                severity: hit.severity,
                category: hit.category.to_string(),
                message: hit.message.to_string(),
                evidence_note: hit.evidence_note.map(|s| s.to_string()),
                paths: Vec::new(),
                commands: Vec::new(),
            });
            if !entry.paths.iter().any(|p| p == path) {
                entry.paths.push(path.to_string());
            }
            if hit.severity > entry.severity {
                entry.severity = hit.severity;
            }
        }
    };

    for path in changed_paths {
        add_path(path);
    }

    for hit in match_command(agent_command) {
        let key = format!("cmd:{}:{}", hit.category, hit.message);
        let entry = by_key.entry(key).or_insert_with(|| Acc {
            severity: hit.severity,
            category: hit.category.to_string(),
            message: hit.message.to_string(),
            evidence_note: None,
            paths: Vec::new(),
            commands: Vec::new(),
        });
        if !entry.commands.iter().any(|c| c == agent_command) {
            entry.commands.push(agent_command.to_string());
        }
        if hit.severity > entry.severity {
            entry.severity = hit.severity;
        }
    }

    let mut findings: Vec<RiskFinding> = by_key
        .into_values()
        .enumerate()
        .map(|(i, acc)| RiskFinding {
            id: format!("risk_{:03}", i + 1),
            severity: acc.severity,
            category: acc.category,
            message: acc.message,
            paths: acc.paths,
            commands: acc.commands,
            evidence_note: acc.evidence_note,
        })
        .collect();

    // Sort by severity desc, then category
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.category.cmp(&b.category))
    });
    // Re-number ids after sort
    for (i, f) in findings.iter_mut().enumerate() {
        f.id = format!("risk_{:03}", i + 1);
    }

    let overall_level = findings
        .iter()
        .map(|f| f.severity)
        .max()
        .unwrap_or(RiskLevel::None);

    RiskReport {
        run_id: run_id.to_string(),
        overall_level,
        findings,
    }
}

/// Human-readable `lumirix risks` output.
pub fn format_risks_report(report: &RiskReport) -> String {
    let mut lines = vec![format!(
        "Overall risk: {}",
        report.overall_level.display_title()
    )];

    if report.findings.is_empty() {
        lines.push("Findings: none".to_string());
        return lines.join("\n");
    }

    lines.push(String::new());
    lines.push("Findings:".to_string());
    for (i, f) in report.findings.iter().enumerate() {
        lines.push(format!(
            "{}. [{}] {} — {}",
            i + 1,
            f.severity.as_str(),
            f.category,
            f.message
        ));
        // Success-criteria style one-liner for critical/high
        lines.push(format!(
            "   {} risk: {}",
            f.severity.display_title(),
            f.message
        ));
        if !f.paths.is_empty() {
            lines.push(format!("   paths: {}", f.paths.join(", ")));
        }
        if !f.commands.is_empty() {
            lines.push(format!("   commands: {}", f.commands.join(" | ")));
        }
        if let Some(ref note) = f.evidence_note {
            lines.push(format!("   {note}"));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_file_critical() {
        let paths = vec![".env".to_string()];
        let r = evaluate_risks("run_1", "echo hi", &[], &paths);
        assert_eq!(r.overall_level, RiskLevel::Critical);
        assert!(r.findings.iter().any(|f| f.category == "secret_access"));
    }

    #[test]
    fn auth_high_with_note() {
        let paths = vec!["src/auth/session.ts".to_string()];
        let r = evaluate_risks("run_1", "npm test", &[], &paths);
        assert_eq!(r.overall_level, RiskLevel::High);
        let f = r.findings.iter().find(|f| f.category == "auth_change").unwrap();
        assert_eq!(
            f.evidence_note.as_deref(),
            Some("Evidence not yet evaluated.")
        );
    }

    #[test]
    fn command_risk_without_diff() {
        let r = evaluate_risks("run_1", "terraform destroy -auto-approve", &[], &[]);
        assert_eq!(r.overall_level, RiskLevel::Critical);
    }

    #[test]
    fn clean_run_none() {
        let paths = vec!["README.md".to_string()];
        let r = evaluate_risks("run_1", "git status", &[], &paths);
        assert_eq!(r.overall_level, RiskLevel::None);
        assert!(r.findings.is_empty());
    }
}
