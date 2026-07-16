//! Trust report assembly and rendering.

mod model;
mod next_steps;
mod verdict;

use std::path::Path;

use chrono::Local;

use crate::evidence::EvidenceLevel;
use crate::risk::{match_path, RiskLevel};
use crate::run::{RunPaths, RunRecord, StoreError};
use crate::run::store;

pub use model::{
    ArtifactsSection, ChangedFileRow, RollbackSection, RunSection, TrustReport, Verdict,
};

/// Build a full trust report from a completed run record.
pub fn build_trust_report(record: &RunRecord, paths: &crate::paths::LumirixPaths) -> TrustReport {
    let run_paths = RunPaths::new(&paths.runs_dir, &record.run_id);
    let risk_level = record
        .risk
        .as_ref()
        .map(|r| r.overall_level)
        .or_else(|| parse_risk_level(record.risk_level.as_deref()))
        .unwrap_or(RiskLevel::None);
    let evidence_level = record
        .evidence
        .as_ref()
        .map(|e| e.level)
        .or_else(|| parse_evidence_level(record.evidence_level.as_deref()))
        .unwrap_or(EvidenceLevel::None);

    let (recommendation, recommendation_code) =
        verdict::compute_recommendation(risk_level, evidence_level, record.status);

    let risk_messages: Vec<String> = record
        .risk
        .as_ref()
        .map(|r| r.findings.iter().map(|f| f.message.clone()).collect())
        .unwrap_or_default();
    let evidence_reason = record
        .evidence
        .as_ref()
        .map(|e| e.reason.as_str())
        .unwrap_or("");
    let files_changed = record
        .git_diff
        .as_ref()
        .map(|d| d.files_changed)
        .unwrap_or(0);

    let summary = verdict::build_summary(
        risk_level,
        evidence_level,
        &risk_messages,
        evidence_reason,
        files_changed,
    );

    let changed_files = build_changed_files(record);
    let next_steps = next_steps::suggest_next_steps(
        record.risk.as_ref(),
        record.evidence.as_ref(),
        record.git_diff.as_ref(),
    );

    let rollback_status = record
        .git_diff
        .as_ref()
        .map(|d| d.rollback_status.clone())
        .unwrap_or_else(|| "unknown".into());

    TrustReport {
        run_id: record.run_id.clone(),
        generated_at: Local::now().to_rfc3339(),
        verdict: Verdict {
            risk: risk_level.as_str().to_string(),
            evidence: evidence_level.as_str().to_string(),
            recommendation,
            recommendation_code,
        },
        run: RunSection {
            command: record.agent_command.clone(),
            agent_name: record.agent_name.clone(),
            branch: record.branch.clone(),
            base_commit: record.base_commit.clone(),
            exit_code: record.exit_code,
            status: record.status.as_str().to_string(),
            started_at: record.started_at.clone(),
            ended_at: record.ended_at.clone(),
            task: record.task.clone(),
        },
        summary,
        changed_files,
        risk: record.risk.clone(),
        evidence: record.evidence.clone(),
        rollback: RollbackSection {
            status: rollback_status,
            diff_patch: run_paths.diff_patch.is_file()
                && record
                    .git_diff
                    .as_ref()
                    .map(|d| d.diff_patch)
                    .unwrap_or(false),
            rollback_patch: run_paths.rollback_patch.is_file()
                && record
                    .git_diff
                    .as_ref()
                    .map(|d| d.rollback_patch)
                    .unwrap_or(false),
        },
        next_steps,
        policy: "Not evaluated (policy engine not enabled).".into(),
        artifacts: ArtifactsSection {
            run_dir: run_paths.dir.display().to_string(),
            stdout: run_paths.stdout.display().to_string(),
            stderr: run_paths.stderr.display().to_string(),
            diff_patch: run_paths.diff_patch.display().to_string(),
            rollback_patch: run_paths.rollback_patch.display().to_string(),
            risk_json: run_paths.risk_json.display().to_string(),
            evidence_json: run_paths.evidence_json.display().to_string(),
            report_md: run_paths.report_md.display().to_string(),
            report_json: run_paths.report_json.display().to_string(),
        },
    }
}

/// Write report.md and report.json for a run.
pub fn write_report_artifacts(
    run_paths: &RunPaths,
    report: &TrustReport,
) -> Result<(), StoreError> {
    let json = serde_json::to_string_pretty(report)?;
    store::write_text(&run_paths.report_json, &format!("{json}\n"))?;
    let md = render_markdown(report);
    store::write_text(&run_paths.report_md, &md)?;
    Ok(())
}

/// Terminal trust report (verdict first).
pub fn render_text(report: &TrustReport) -> String {
    let mut lines = vec![
        "=== Lumirix Trust Report ===".to_string(),
        format!("Run: {}", report.run_id),
        String::new(),
        "## Verdict".to_string(),
        format!("Risk: {}", title_case_level(&report.verdict.risk)),
        format!("Evidence: {}", title_case_level(&report.verdict.evidence)),
        format!("Recommendation: {}", report.verdict.recommendation),
        String::new(),
        "## Summary".to_string(),
        report.summary.clone(),
        String::new(),
        "## Run".to_string(),
        format!("Command: {}", report.run.command),
        format!("Agent: {}", report.run.agent_name),
        format!(
            "Branch: {}",
            report.run.branch.as_deref().unwrap_or("(none)")
        ),
        format!(
            "Base commit: {}",
            report.run.base_commit.as_deref().unwrap_or("(none)")
        ),
        format!(
            "Exit code: {}",
            report
                .exit_code_display()
        ),
        format!("Status: {}", report.run.status),
    ];

    lines.push(String::new());
    lines.push("## Changed Files".to_string());
    if report.changed_files.is_empty() {
        lines.push("(none tracked)".to_string());
    } else {
        for f in &report.changed_files {
            let tag = f.risk_tag.as_deref().unwrap_or("info");
            lines.push(format!(
                "- {} (+{} -{}) [{}]",
                f.path, f.added, f.deleted, tag
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Risk Findings".to_string());
    if let Some(ref r) = report.risk {
        if r.findings.is_empty() {
            lines.push("(none)".to_string());
        } else {
            for (i, f) in r.findings.iter().enumerate() {
                lines.push(format!(
                    "{}. [{}] {} — {}",
                    i + 1,
                    f.severity.as_str(),
                    f.category,
                    f.message
                ));
            }
        }
    } else {
        lines.push("(not captured)".to_string());
    }

    lines.push(String::new());
    lines.push("## Evidence".to_string());
    if let Some(ref e) = report.evidence {
        lines.push(format!("Level: {}", e.level.display_title()));
        lines.push(format!("Reason: {}", e.reason));
        if e.tests.is_empty() {
            lines.push("Tests: (none detected)".to_string());
        } else {
            for t in &e.tests {
                lines.push(format!("- {} → {}", t.command, t.result));
            }
        }
    } else {
        lines.push("(not captured)".to_string());
    }

    lines.push(String::new());
    lines.push("## Rollback".to_string());
    lines.push(format!("Status: {}", report.rollback.status));
    lines.push(format!("diff.patch: {}", report.rollback.diff_patch));
    lines.push(format!(
        "rollback.patch: {}",
        report.rollback.rollback_patch
    ));

    lines.push(String::new());
    lines.push("## Policy".to_string());
    lines.push(report.policy.clone());

    lines.push(String::new());
    lines.push("## Suggested Next Steps".to_string());
    for (i, s) in report.next_steps.iter().enumerate() {
        lines.push(format!("{}. {s}", i + 1));
    }

    lines.push(String::new());
    lines.push("## Artifacts".to_string());
    lines.push(format!("Directory: {}", report.artifacts.run_dir));
    lines.push(format!("report.md: {}", report.artifacts.report_md));
    lines.push(format!("report.json: {}", report.artifacts.report_json));

    lines.join("\n") + "\n"
}

/// Markdown trust report body.
pub fn render_markdown(report: &TrustReport) -> String {
    let mut md = String::new();
    md.push_str("# Lumirix Trust Report\n\n");
    md.push_str(&format!("Run: `{}`  \n", report.run_id));
    md.push_str(&format!("Generated: {}\n\n", report.generated_at));

    md.push_str("## Verdict\n\n");
    md.push_str(&format!(
        "Risk: **{}**  \n",
        title_case_level(&report.verdict.risk)
    ));
    md.push_str(&format!(
        "Evidence: **{}**  \n",
        title_case_level(&report.verdict.evidence)
    ));
    md.push_str(&format!(
        "Recommendation: **{}**\n\n",
        report.verdict.recommendation
    ));

    md.push_str("## Summary\n\n");
    md.push_str(&report.summary);
    md.push_str("\n\n");

    md.push_str("## Run\n\n");
    md.push_str(&format!("- Command: `{}`\n", report.run.command));
    md.push_str(&format!("- Agent: `{}`\n", report.run.agent_name));
    md.push_str(&format!(
        "- Branch: {}\n",
        report.run.branch.as_deref().unwrap_or("(none)")
    ));
    md.push_str(&format!(
        "- Base commit: {}\n",
        report.run.base_commit.as_deref().unwrap_or("(none)")
    ));
    md.push_str(&format!("- Exit code: {}\n", report.exit_code_display()));
    md.push_str(&format!("- Status: {}\n\n", report.run.status));

    md.push_str("## Changed Files\n\n");
    if report.changed_files.is_empty() {
        md.push_str("_None tracked._\n\n");
    } else {
        md.push_str("| File | + | - | Risk |\n|---|---:|---:|---|\n");
        for f in &report.changed_files {
            md.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                f.path,
                f.added,
                f.deleted,
                f.risk_tag.as_deref().unwrap_or("-")
            ));
        }
        md.push('\n');
    }

    md.push_str("## Risk Findings\n\n");
    if let Some(ref r) = report.risk {
        if r.findings.is_empty() {
            md.push_str("_None._\n\n");
        } else {
            for (i, f) in r.findings.iter().enumerate() {
                md.push_str(&format!(
                    "{}. **[{}] {}** — {}\n",
                    i + 1,
                    f.severity.as_str(),
                    f.category,
                    f.message
                ));
                if !f.paths.is_empty() {
                    md.push_str(&format!("   - paths: `{}`\n", f.paths.join("`, `")));
                }
            }
            md.push('\n');
        }
    } else {
        md.push_str("_Not captured._\n\n");
    }

    md.push_str("## Evidence\n\n");
    if let Some(ref e) = report.evidence {
        md.push_str(&format!("Level: **{}**  \n", e.level.display_title()));
        md.push_str(&format!("Reason: {}\n\n", e.reason));
        if e.tests.is_empty() {
            md.push_str("Tests: _none detected_\n\n");
        } else {
            md.push_str("| Command | Result |\n|---|---|\n");
            for t in &e.tests {
                md.push_str(&format!("| `{}` | {} |\n", t.command, t.result));
            }
            md.push('\n');
        }
    } else {
        md.push_str("_Not captured._\n\n");
    }

    md.push_str("## Rollback\n\n");
    md.push_str(&format!("- Status: `{}`\n", report.rollback.status));
    md.push_str(&format!(
        "- diff.patch present: {}\n",
        report.rollback.diff_patch
    ));
    md.push_str(&format!(
        "- rollback.patch present: {}\n\n",
        report.rollback.rollback_patch
    ));

    md.push_str("## Policy\n\n");
    md.push_str(&format!("{}\n\n", report.policy));

    md.push_str("## Suggested Next Steps\n\n");
    for (i, s) in report.next_steps.iter().enumerate() {
        md.push_str(&format!("{}. {}\n", i + 1, s));
    }
    md.push('\n');

    md.push_str("## Artifacts\n\n");
    md.push_str(&format!("- Run dir: `{}`\n", report.artifacts.run_dir));
    md.push_str(&format!("- report.md: `{}`\n", report.artifacts.report_md));
    md.push_str(&format!(
        "- report.json: `{}`\n",
        report.artifacts.report_json
    ));

    md
}

impl TrustReport {
    fn exit_code_display(&self) -> String {
        self.run
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "(none)".into())
    }
}

fn build_changed_files(record: &RunRecord) -> Vec<ChangedFileRow> {
    let mut rows = Vec::new();
    if let Some(ref d) = record.git_diff {
        for f in &d.files {
            let risk_tag = match_path(&f.path).map(|h| h.category.to_string());
            rows.push(ChangedFileRow {
                path: f.path.clone(),
                added: f.added,
                deleted: f.deleted,
                risk_tag,
            });
        }
        for u in &d.untracked {
            let risk_tag = match_path(u).map(|h| h.category.to_string());
            rows.push(ChangedFileRow {
                path: format!("{u} (untracked)"),
                added: 0,
                deleted: 0,
                risk_tag,
            });
        }
    }
    rows
}

fn parse_risk_level(s: Option<&str>) -> Option<RiskLevel> {
    match s? {
        "none" => Some(RiskLevel::None),
        "low" => Some(RiskLevel::Low),
        "medium" => Some(RiskLevel::Medium),
        "high" => Some(RiskLevel::High),
        "critical" => Some(RiskLevel::Critical),
        _ => None,
    }
}

fn parse_evidence_level(s: Option<&str>) -> Option<EvidenceLevel> {
    match s? {
        "none" => Some(EvidenceLevel::None),
        "not_needed" => Some(EvidenceLevel::NotNeeded),
        "weak" => Some(EvidenceLevel::Weak),
        "failed" => Some(EvidenceLevel::Failed),
        "medium" => Some(EvidenceLevel::Medium),
        "strong" => Some(EvidenceLevel::Strong),
        _ => None,
    }
}

fn title_case_level(s: &str) -> String {
    match s {
        "not_needed" => "Not needed".into(),
        "none" => "None".into(),
        "low" => "Low".into(),
        "medium" => "Medium".into(),
        "high" => "High".into(),
        "critical" => "Critical".into(),
        "weak" => "Weak".into(),
        "failed" => "Failed".into(),
        "strong" => "Strong".into(),
        other => other.to_string(),
    }
}

/// Helper used by CLI when only a path is needed.
pub fn report_paths_for(runs_dir: &Path, run_id: &str) -> RunPaths {
    RunPaths::new(runs_dir, run_id)
}
