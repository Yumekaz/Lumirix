//! Lightweight integration checks for the MVP pipeline (no full agent).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use lumirix_core::{
    evaluate_evidence, evaluate_risks, init_project, EvidenceLevel, RiskLevel,
};

fn temp_dir(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "lumirix-mvp-{}-{}",
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn init_creates_store() {
    let dir = temp_dir("init");
    let result = init_project(&dir, false).expect("init");
    assert!(result.paths.config.is_file());
    assert!(result.paths.db.is_file());
    assert!(result.paths.default_policy.is_file());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn risk_and_evidence_compose_for_auth_without_tests() {
    let paths = vec!["src/auth/session.ts".to_string()];
    let risk = evaluate_risks("run_t", "cmd /C echo x", &[], &paths);
    assert_eq!(risk.overall_level, RiskLevel::High);

    let evidence = evaluate_evidence(
        "run_t",
        "cmd /C echo x",
        &["cmd".into(), "/C".into(), "echo".into(), "x".into()],
        Some(0),
        &paths,
    );
    assert_eq!(evidence.level, EvidenceLevel::Weak);
    assert!(evidence.reason.to_lowercase().contains("auth"));
}

#[test]
fn git_available_for_diff_helpers() {
    let out = Command::new("git").arg("--version").output();
    assert!(out.is_ok(), "git should be available in CI/dev environment");
}
