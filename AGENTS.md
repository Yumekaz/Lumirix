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

**Build phase by phase.** Each phase must be complete and usable before the next starts.

Do **not** jump ahead to: dashboard, cloud, multi-agent adapters, heavy LLM, perfect sandbox replay, enterprise, SaaS.

### Current phase tracker

| Phase | Name | Status |
|-------|------|--------|
| 0 | Research & scope lock | **DONE** |
| 1 | CLI skeleton + `init`/`status`/`config show` | **DONE** (Rust) |
| 2 | Agent run wrapper | **NEXT** |
| 3 | Git diff capture + rollback patch | pending |
| 4 | Basic risk engine | pending |
| 5 | Test evidence capture | pending |
| 6 | Trust report V1 (MVP milestone) | pending |
| 7 | Policy engine V1 | pending |
| 8 | GitHub Actions CI | pending |
| 9 | Replay V1 scripts | pending |
| 10 | Context pack generator | pending |
| 11 | Optional LLM enhancement | pending |
| 12+ | Dashboard, sandbox, corpus, advanced, enterprise | later |

Update the Status column when a phase is finished.

## True MVP (Phases 1–6)

```txt
CLI that wraps an agent run, captures Git diff and commands, detects risky
changes, checks test evidence, generates rollback patch, and writes a trust report.
```

MVP commands:

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

1. Read only the current phase section in the private blueprint; implement that phase only.
2. Verify success criteria before advancing.
3. Prefer simple, working releases over incomplete multi-phase piles.
4. Keep LLM disabled by default forever unless the active phase is 11+.
5. Prefer clean Git worktree for runs (`--allow-dirty` override only when needed).
