//! Deterministic recommendation rules.

use crate::evidence::EvidenceLevel;
use crate::risk::RiskLevel;
use crate::run::RunStatus;

/// Compute recommendation text + code from risk, evidence, and run status.
pub fn compute_recommendation(
    risk: RiskLevel,
    evidence: EvidenceLevel,
    run_status: RunStatus,
) -> (String, String) {
    if matches!(run_status, RunStatus::Error) {
        return (
            "Do not merge — run ended with an internal error.".into(),
            "run_error".into(),
        );
    }

    let (mut text, mut code) = match risk {
        RiskLevel::Critical => (
            "Do not merge — critical risk findings.".into(),
            "do_not_merge_critical".into(),
        ),
        RiskLevel::High => match evidence {
            EvidenceLevel::Strong => (
                "Review carefully before merge — high risk with strong test evidence.".into(),
                "review_carefully".into(),
            ),
            EvidenceLevel::Failed => (
                "Do not merge yet — high risk and tests failed.".into(),
                "do_not_merge_yet".into(),
            ),
            _ => (
                "Do not merge yet — high risk with insufficient evidence.".into(),
                "do_not_merge_yet".into(),
            ),
        },
        RiskLevel::Medium => (
            "Review before merge — medium risk findings.".into(),
            "review_before_merge".into(),
        ),
        RiskLevel::None | RiskLevel::Low => match evidence {
            EvidenceLevel::Failed => (
                "Do not merge — tests failed.".into(),
                "do_not_merge_tests_failed".into(),
            ),
            EvidenceLevel::Weak => (
                "Review — weak test evidence for the changes.".into(),
                "review_weak_evidence".into(),
            ),
            EvidenceLevel::Medium | EvidenceLevel::Strong | EvidenceLevel::NotNeeded => (
                "No strong block signals in V1 rules — still review before you merge.".into(),
                "likely_safe".into(),
            ),
            EvidenceLevel::None => (
                "Review — evidence not classified.".into(),
                "review".into(),
            ),
        },
    };

    if matches!(run_status, RunStatus::Failed)
        && code != "do_not_merge_tests_failed"
        && code != "do_not_merge_critical"
    {
        text = format!("{text} Command exited non-zero.");
        if code == "likely_safe" {
            code = "review_command_failed".into();
            text = "Review — wrapped command failed.".into();
        }
    }

    (text, code)
}

/// Build a short deterministic summary paragraph.
pub fn build_summary(
    risk: RiskLevel,
    evidence: EvidenceLevel,
    risk_messages: &[String],
    evidence_reason: &str,
    files_changed: u32,
) -> String {
    let mut parts = Vec::new();

    if files_changed == 0 {
        parts.push("No tracked file changes were captured for this run.".to_string());
    } else {
        parts.push(format!(
            "{files_changed} tracked file(s) changed relative to HEAD."
        ));
    }

    if !risk_messages.is_empty() {
        parts.push(format!("Risk signals: {}.", risk_messages.join("; ")));
    } else {
        parts.push("No high-risk path/command signals matched V1 rules.".to_string());
    }

    if !evidence_reason.is_empty() {
        parts.push(format!("Evidence: {evidence_reason}"));
    } else {
        parts.push(format!(
            "Evidence level: {}.",
            evidence.display_title()
        ));
    }

    parts.push(format!("Overall risk level: {}.", risk.display_title()));
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_weak_do_not_merge_yet() {
        let (t, c) = compute_recommendation(
            RiskLevel::High,
            EvidenceLevel::Weak,
            RunStatus::Completed,
        );
        assert!(t.contains("Do not merge yet"));
        assert_eq!(c, "do_not_merge_yet");
    }

    #[test]
    fn critical_blocks() {
        let (t, _) = compute_recommendation(
            RiskLevel::Critical,
            EvidenceLevel::Strong,
            RunStatus::Completed,
        );
        assert!(t.contains("Do not merge"));
    }

    #[test]
    fn low_not_needed_likely_safe() {
        let (t, c) = compute_recommendation(
            RiskLevel::None,
            EvidenceLevel::NotNeeded,
            RunStatus::Completed,
        );
        assert!(t.contains("review before you merge") || t.contains("No strong block"));
        assert_eq!(c, "likely_safe");
    }
}
