//! Test evidence capture and strength scoring.

mod detect;
mod model;
mod relevance;

pub use model::{EvidenceLevel, EvidenceReport, TestRecord, TestsFile};

/// Evaluate tests + evidence for a run (pure; no I/O).
pub fn evaluate_evidence(
    run_id: &str,
    agent_command: &str,
    agent_argv: &[String],
    exit_code: Option<i32>,
    changed_paths: &[String],
) -> EvidenceReport {
    let kind = detect::detect_test_kind(agent_argv, agent_command);
    let tests = if let Some(kind) = kind {
        let result = match exit_code {
            Some(0) => "passed",
            Some(_) => "failed",
            None => "unknown",
        };
        vec![TestRecord {
            id: "test_001".into(),
            command: agent_command.to_string(),
            argv: agent_argv.to_vec(),
            kind: kind.to_string(),
            exit_code,
            result: result.to_string(),
            stdout_path: "stdout.log".into(),
            stderr_path: "stderr.log".into(),
            detected: true,
        }]
    } else {
        Vec::new()
    };

    let tests_detected = !tests.is_empty();
    let tests_passed = tests.iter().any(|t| t.result == "passed");
    let tests_failed = tests.iter().any(|t| t.result == "failed");

    let areas = relevance::sensitive_areas(changed_paths);
    let keywords = relevance::keywords_from_paths(changed_paths);
    let relevant_tests =
        tests_detected && relevance::command_relevant(agent_command, agent_argv, &keywords);
    let any_code = changed_paths.iter().any(|p| relevance::is_code_path(p));
    let has_auth = areas.iter().any(|a| a == "auth");

    let (level, reason) = if tests_failed {
        (
            EvidenceLevel::Failed,
            "Test command exited with a non-zero status.".to_string(),
        )
    } else if !areas.is_empty() {
        if !tests_detected {
            if has_auth {
                (
                    EvidenceLevel::Weak,
                    "auth-sensitive files changed but no auth-related tests were detected."
                        .to_string(),
                )
            } else {
                (
                    EvidenceLevel::Weak,
                    format!(
                        "sensitive areas changed ({}) but no test command was detected.",
                        areas.join(", ")
                    ),
                )
            }
        } else if tests_passed && relevant_tests {
            (
                EvidenceLevel::Strong,
                "Relevant tests appear to have run and passed (name heuristic).".to_string(),
            )
        } else if tests_passed {
            (
                EvidenceLevel::Weak,
                "Tests passed but do not appear related to the sensitive changed paths (name heuristic)."
                    .to_string(),
            )
        } else {
            (
                EvidenceLevel::Weak,
                "Sensitive paths changed; test evidence incomplete.".to_string(),
            )
        }
    } else if !tests_detected {
        if any_code {
            (
                EvidenceLevel::Weak,
                "Code files changed but no test command was detected.".to_string(),
            )
        } else {
            (
                EvidenceLevel::NotNeeded,
                "No sensitive paths changed; tests not required for V1.".to_string(),
            )
        }
    } else if tests_passed {
        (
            EvidenceLevel::Medium,
            "Tests ran and passed; no sensitive-path keyword match required.".to_string(),
        )
    } else {
        (
            EvidenceLevel::None,
            "No conclusive evidence classification.".to_string(),
        )
    };

    EvidenceReport {
        run_id: run_id.to_string(),
        level,
        reason,
        tests_detected,
        tests_passed,
        relevant_tests,
        sensitive_areas: areas,
        matched_keywords: keywords,
        tests,
    }
}

/// Human-readable `lumirix evidence` output.
pub fn format_evidence_report(report: &EvidenceReport) -> String {
    let mut lines = vec![
        format!("Evidence: {}", report.level.display_title()),
        format!("Reason: {}", report.reason),
    ];
    if !report.sensitive_areas.is_empty() {
        lines.push(format!(
            "Sensitive areas: {}",
            report.sensitive_areas.join(", ")
        ));
    }
    lines.push(String::new());
    lines.push("Tests:".to_string());
    if report.tests.is_empty() {
        lines.push("  (none detected)".to_string());
    } else {
        for t in &report.tests {
            let code = t
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".into());
            lines.push(format!(
                "  - {} ({}) → {} (exit {})",
                t.command, t.kind, t.result, code
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_change_no_tests_weak() {
        let r = evaluate_evidence(
            "run_1",
            r#"cmd /C "echo x""#,
            &["cmd".into(), "/C".into(), "echo".into(), "x".into()],
            Some(0),
            &["src/auth/session.ts".into()],
        );
        assert_eq!(r.level, EvidenceLevel::Weak);
        assert!(r.reason.contains("auth-sensitive"));
        assert!(!r.tests_detected);
    }

    #[test]
    fn cargo_test_pass_medium_without_sensitive() {
        let r = evaluate_evidence(
            "run_1",
            "cargo test",
            &["cargo".into(), "test".into()],
            Some(0),
            &["README.md".into()],
        );
        assert_eq!(r.level, EvidenceLevel::Medium);
        assert!(r.tests_detected);
        assert!(r.tests_passed);
    }

    #[test]
    fn test_failed() {
        let r = evaluate_evidence(
            "run_1",
            "cargo test",
            &["cargo".into(), "test".into()],
            Some(1),
            &["src/lib.rs".into()],
        );
        assert_eq!(r.level, EvidenceLevel::Failed);
    }

    #[test]
    fn relevant_auth_test_strong() {
        let r = evaluate_evidence(
            "run_1",
            "npm test -- auth",
            &["npm".into(), "test".into(), "--".into(), "auth".into()],
            Some(0),
            &["src/auth/session.ts".into()],
        );
        assert_eq!(r.level, EvidenceLevel::Strong);
        assert!(r.relevant_tests);
    }
}
