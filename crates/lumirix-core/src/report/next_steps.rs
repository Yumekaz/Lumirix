//! Rule-based suggested next steps.

use crate::evidence::{EvidenceLevel, EvidenceReport};
use crate::risk::{RiskLevel, RiskReport};
use crate::run::DiffSummary;

pub fn suggest_next_steps(
    risk: Option<&RiskReport>,
    evidence: Option<&EvidenceReport>,
    diff: Option<&DiffSummary>,
) -> Vec<String> {
    let mut steps = Vec::new();

    let risk_level = risk.map(|r| r.overall_level).unwrap_or(RiskLevel::None);
    let evidence_level = evidence.map(|e| e.level).unwrap_or(EvidenceLevel::None);

    if let Some(r) = risk {
        for f in &r.findings {
            if f.category == "secret_access" {
                push(
                    &mut steps,
                    "Do not commit secrets; review secret/config diffs and rotate credentials if exposed.",
                );
            }
            if f.category == "auth_change" {
                push(
                    &mut steps,
                    "Add or run auth-related regression tests and manually review auth/middleware diffs.",
                );
            }
            if f.category == "dangerous_command" {
                push(
                    &mut steps,
                    "Confirm the dangerous command was intentional and review command logs.",
                );
            }
            if f.category == "database_migration" {
                push(
                    &mut steps,
                    "Verify migration safety and rollback plan for database changes.",
                );
            }
            if f.category == "infra_change" || f.category == "ci_change" {
                push(
                    &mut steps,
                    "Manually review infrastructure/CI changes before merge.",
                );
            }
        }
    }

    match evidence_level {
        EvidenceLevel::Failed => {
            push(
                &mut steps,
                "Fix failing tests and re-run them under Lumirix.",
            );
        }
        EvidenceLevel::Weak => {
            push(
                &mut steps,
                "Run targeted tests that cover the changed paths, then re-check evidence.",
            );
        }
        EvidenceLevel::Strong | EvidenceLevel::Medium | EvidenceLevel::NotNeeded | EvidenceLevel::None => {}
    }

    if let Some(d) = diff {
        if d.rollback_status == "available" || d.rollback_status == "partial" {
            push(
                &mut steps,
                "If needed, review or apply rollback.patch for tracked changes.",
            );
        }
        if d.files_changed > 0 {
            push(
                &mut steps,
                "Review diff.patch for unintended or unrelated edits.",
            );
        }
    }

    if risk_level >= RiskLevel::High && evidence_level == EvidenceLevel::Strong {
        push(
            &mut steps,
            "Even with strong tests, manually review high-risk paths before merge.",
        );
    }

    if steps.is_empty() {
        steps.push("Review the trust report verdict and re-run targeted tests if unsure.".into());
    }

    steps.truncate(5);
    steps
}

fn push(steps: &mut Vec<String>, s: &str) {
    if !steps.iter().any(|x| x == s) {
        steps.push(s.to_string());
    }
}
