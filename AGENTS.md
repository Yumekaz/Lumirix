# Lumirix — Agent Build Rules

Source of truth: private local product blueprint (never push that file; never mention it in public README/docs).

## One-line product

Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.

**Thesis:** Generation is cheap. Trust is expensive.

## Category

Trust infrastructure for autonomous coding agents — **not** a coding agent, chatbot, memory app, RAG, LLM wrapper, PR summarizer, or generic observability dashboard.

## Non-negotiable architecture

1. **Deterministic core first** — source of truth is: `trace + git diff + command logs + test output + policy rules + replay artifacts`
2. **LLM optional** — default off; may explain/summarize; never required for what changed, pass/fail, policy, rollback, or audit
3. **Local-first** — store under `.lumirix/`; no cloud for V1
4. **Git-native** — diffs, patches, commits, branches as baseline
5. **Agent-agnostic** — `lumirix run -- <any command>`
6. **Evidence over vibes** — never “appears safe”; always cite files, tests, policy, rollback
7. **CLI-first** — no dashboard/UI until after MVP (Phase 12+)
8. **Fail closed** for high-risk when used as a gate

## Stack (serious path)

- **Rust** workspace: `crates/lumirix-cli` + `crates/lumirix-core`
- SQLite + Markdown/JSON reports (later phases)
- Windows builds need VS Build Tools (MSVC) via `vcvars64.bat`

## Build mode (mandatory)

Build **one milestone at a time**. Finish and verify before starting the next.

Do **not** jump ahead to: dashboard, cloud, multi-agent adapters, heavy LLM, perfect sandbox replay, enterprise, SaaS.

### Progress (internal tracker — do not paste phase numbers into README or commit messages)

| Milestone | Status |
|-----------|--------|
| Scope lock | DONE |
| CLI `init` / `status` / `config show` | DONE (Rust, on origin) |
| Agent run wrapper | **DONE** (Rust) |
| Git diff + rollback patch | **DONE** (Rust) |
| Basic risk engine | **NEXT** |
| Test evidence capture | pending |
| Trust report V1 (MVP) | pending |
| Policy engine | pending |
| CI integration | pending |
| Replay scripts | pending |
| Context pack | pending |
| Optional LLM | pending |
| Dashboard / sandbox / enterprise | later |

**Public wording rule:** never say “Phase 1/2/3…” in README, commits, or marketing. Describe features only. Phases are private (you + agent / memory).

## MVP target

```txt
CLI that wraps an agent run, captures Git diff and commands, detects risky
changes, checks test evidence, generates rollback patch, and writes a trust report.
```

Target commands:

```bash
lumirix init
lumirix run -- <agent command>
lumirix report last
lumirix risks last
lumirix evidence last
lumirix rollback last --write rollback.patch
```

## Secrets

- Never commit or push the private product blueprint markdown file.
- Never mention that filename in public docs/README.

## When implementing

1. Read only the **current** private-blueprint section; implement that milestone only.
2. Verify success criteria before advancing.
3. Prefer simple, working releases over incomplete multi-milestone piles.
4. Keep LLM disabled by default unless the active work is optional-LLM enhancement.
5. Prefer clean Git worktree for runs (`--allow-dirty` override only when needed).
6. Commit messages: feature-focused, **no phase numbers**.
