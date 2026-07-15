//! Default policy pack shipped on `lumirix init` (spec §15.2).
//! Rules are stored only in Phase 1 — evaluation begins in Phase 7.

/// Contents of `.lumirix/policies/default.toml`.
pub const DEFAULT_POLICY_TOML: &str = r#"# Lumirix default policies
# These rules are stored on init. Evaluation is not active until Phase 7.

[[rules]]
id = "auth_requires_tests"
description = "Any auth-related change must include or run auth-related tests."
match_paths = ["**/auth/**", "**/middleware/**", "**/session*", "**/token*"]
requires_tests_matching = ["auth", "session", "token", "login"]
severity = "high"
action = "warn"

[[rules]]
id = "migration_requires_rollback"
description = "Database migrations must include rollback or reversible migration evidence."
match_paths = ["**/migrations/**"]
requires_rollback = true
severity = "critical"
action = "fail"

[[rules]]
id = "no_secret_file_change"
description = "Agents should not modify secret/config credential files."
match_paths = [".env", ".env.*", "**/secrets.yml", "**/credentials.json"]
severity = "critical"
action = "fail"

[[rules]]
id = "infra_requires_approval"
description = "Infrastructure changes require explicit human review."
match_paths = ["terraform/**", "k8s/**", ".github/workflows/**", "docker-compose.yml"]
severity = "high"
action = "require_approval"

[[rules]]
id = "dangerous_commands"
description = "Detect dangerous shell commands."
match_commands = ["rm -rf", "drop database", "kubectl delete", "terraform destroy"]
severity = "critical"
action = "block_or_warn"
"#;
