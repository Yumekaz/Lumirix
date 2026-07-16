//! Detect common top-level test commands.

/// Classify a top-level command as a known test runner kind, if any.
pub fn detect_test_kind(argv: &[String], command_line: &str) -> Option<&'static str> {
    if argv.is_empty() && command_line.trim().is_empty() {
        return None;
    }

    let lower_line = command_line.to_lowercase();
    let tokens: Vec<String> = if argv.is_empty() {
        shellish_tokens(&lower_line)
    } else {
        argv.iter().map(|a| a.to_lowercase()).collect()
    };

    // npm / pnpm / yarn test
    if let Some(kind) = pm_test_kind(&tokens) {
        return Some(kind);
    }

    // python -m pytest
    if tokens.windows(3).any(|w| {
        (w[0] == "python" || w[0] == "python3") && w[1] == "-m" && w[2] == "pytest"
    }) {
        return Some("pytest");
    }

    // bare runners
    if tokens.iter().any(|t| t == "pytest") {
        return Some("pytest");
    }
    if tokens.windows(2).any(|w| w[0] == "go" && w[1] == "test") {
        return Some("go_test");
    }
    if tokens.windows(2).any(|w| w[0] == "cargo" && w[1] == "test") {
        return Some("cargo_test");
    }
    if tokens.windows(2).any(|w| w[0] == "mvn" && w[1] == "test") {
        return Some("mvn_test");
    }
    if tokens.windows(2).any(|w| {
        (w[0] == "gradle" || w[0] == "gradlew" || w[0] == "./gradlew") && w[1] == "test"
    }) {
        return Some("gradle_test");
    }
    if tokens.iter().any(|t| t == "vitest" || t.ends_with("/vitest")) {
        return Some("vitest");
    }
    if tokens.windows(2).any(|w| w[0] == "npx" && w[1] == "vitest") {
        return Some("vitest");
    }
    if tokens.iter().any(|t| t == "jest" || t.ends_with("/jest")) {
        return Some("jest");
    }
    if tokens.windows(2).any(|w| w[0] == "npx" && w[1] == "jest") {
        return Some("jest");
    }

    // Fallback line contains (careful with false positives)
    if lower_line.contains("cargo test") {
        return Some("cargo_test");
    }
    if lower_line.contains("npm test") || lower_line.contains("npm run test") {
        return Some("npm_test");
    }
    if lower_line.contains("pytest") {
        return Some("pytest");
    }

    None
}

fn pm_test_kind(tokens: &[String]) -> Option<&'static str> {
    let pms = [
        ("npm", "npm_test"),
        ("pnpm", "pnpm_test"),
        ("yarn", "yarn_test"),
    ];
    for (pm, kind) in pms {
        // npm test
        if tokens.windows(2).any(|w| w[0] == pm && w[1] == "test") {
            return Some(kind);
        }
        // npm run test / npm run test:unit
        if tokens.windows(3).any(|w| {
            w[0] == pm && w[1] == "run" && (w[2] == "test" || w[2].starts_with("test:"))
        }) {
            return Some(kind);
        }
    }
    None
}

fn shellish_tokens(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(|s| s.trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_npm_test() {
        let argv = vec!["npm".into(), "test".into()];
        assert_eq!(detect_test_kind(&argv, "npm test"), Some("npm_test"));
    }

    #[test]
    fn detects_cargo_test() {
        let argv = vec!["cargo".into(), "test".into(), "-p".into(), "foo".into()];
        assert_eq!(detect_test_kind(&argv, "cargo test -p foo"), Some("cargo_test"));
    }

    #[test]
    fn detects_pytest_module() {
        let argv = vec!["python".into(), "-m".into(), "pytest".into()];
        assert_eq!(detect_test_kind(&argv, "python -m pytest"), Some("pytest"));
    }

    #[test]
    fn ignores_echo() {
        let argv = vec!["cmd".into(), "/C".into(), "echo".into(), "hi".into()];
        assert_eq!(detect_test_kind(&argv, r#"cmd /C "echo hi""#), None);
    }
}
