//! Path → sensitive areas and keyword relevance heuristics.

use crate::risk::normalize_path;

/// Keywords that mark a path as a sensitive area.
const AUTH_KEYS: &[&str] = &["auth", "middleware", "session", "token", "login", "oauth"];
const MIGRATION_KEYS: &[&str] = &["migration", "migrations"];
const INFRA_KEYS: &[&str] = &["terraform", "k8s", "kubernetes", "dockerfile", "docker-compose"];
const SECRET_KEYS: &[&str] = &[".env", "secrets", "credentials"];

/// Collect sensitive area labels from changed paths.
pub fn sensitive_areas(paths: &[String]) -> Vec<String> {
    let mut areas = Vec::new();
    let mut push = |a: &str| {
        if !areas.iter().any(|x| x == a) {
            areas.push(a.to_string());
        }
    };

    for p in paths {
        let n = normalize_path(p);
        if AUTH_KEYS.iter().any(|k| n.contains(k)) {
            push("auth");
        }
        if MIGRATION_KEYS.iter().any(|k| n.contains(k)) {
            push("migration");
        }
        if INFRA_KEYS.iter().any(|k| n.contains(k)) {
            push("infra");
        }
        if SECRET_KEYS.iter().any(|k| n.contains(k)) {
            push("secret");
        }
    }
    areas
}

/// Keywords derived from paths for command relevance matching.
pub fn keywords_from_paths(paths: &[String]) -> Vec<String> {
    let mut keys = Vec::new();
    let interesting = [
        "auth",
        "middleware",
        "session",
        "token",
        "login",
        "migration",
        "terraform",
        "k8s",
        "secret",
    ];
    for p in paths {
        let n = normalize_path(p);
        for k in interesting {
            if n.contains(k) && !keys.iter().any(|x| x == k) {
                keys.push(k.to_string());
            }
        }
        // file stem
        if let Some(stem) = n.rsplit('/').next() {
            let stem = stem.split('.').next().unwrap_or(stem);
            if stem.len() >= 3
                && stem
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                && !keys.iter().any(|x| x == stem)
            {
                // only keep stems that look like domain words if they match interesting fragments
                if interesting.iter().any(|k| stem.contains(k)) {
                    keys.push(stem.to_string());
                }
            }
        }
    }
    keys
}

/// Whether the command line appears related to changed sensitive keywords.
pub fn command_relevant(command: &str, argv: &[String], keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return false;
    }
    let mut hay = command.to_lowercase();
    for a in argv {
        hay.push(' ');
        hay.push_str(&a.to_lowercase());
    }
    keywords.iter().any(|k| hay.contains(&k.to_lowercase()))
}

/// True if path looks like application code (not pure docs).
pub fn is_code_path(path: &str) -> bool {
    let n = normalize_path(path);
    let exts = [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".java", ".kt", ".cs", ".rb", ".php",
        ".c", ".cpp", ".h", ".sql",
    ];
    exts.iter().any(|e| n.ends_with(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_area_from_path() {
        let areas = sensitive_areas(&["src/auth/session.ts".into()]);
        assert!(areas.iter().any(|a| a == "auth"));
    }

    #[test]
    fn relevant_command() {
        let keys = keywords_from_paths(&["src/auth/session.ts".into()]);
        assert!(command_relevant("npm test -- auth", &[], &keys));
    }
}
