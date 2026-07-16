# Lumirix

**Trust infrastructure for autonomous coding agents.**

Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.

> Generation is becoming cheap. Trust is becoming expensive.

## Status

Rust CLI MVP: wrap a command, capture logs + Git diff, score risk and test evidence, export rollback patch, and print a **verdict-first trust report**.

## Requirements

- Rust toolchain (edition 2021)
- **Windows:** Visual Studio Build Tools 2022 with C++ / MSVC + Windows SDK
- **macOS/Linux:** standard system linker (`clang`/`gcc`)
- Git (for full status and diff capture; limited mode without Git)

## Build

### Windows (recommended)

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build.ps1
```

Or manually:

```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build -p lumirix-cli
```

### Other platforms

```bash
cargo build -p lumirix-cli
```

Install:

```bash
cargo install --path crates/lumirix-cli
```

## Try the MVP (5 minutes)

From the repo root, after building:

```powershell
# optional scripted walkthrough
powershell -ExecutionPolicy Bypass -File scripts\demo.ps1
```

Manual walkthrough:

```bat
target\debug\lumirix.exe init
target\debug\lumirix.exe status

rem Happy path
target\debug\lumirix.exe run --allow-dirty -- git --version
target\debug\lumirix.exe report last

rem Critical risk demo
target\debug\lumirix.exe run --allow-dirty -- cmd /C "echo SECRET=demo>> .env"
target\debug\lumirix.exe risks last
target\debug\lumirix.exe report last
del .env

rem Export rollback patch for last run that changed tracked files
target\debug\lumirix.exe rollback last --write rollback.patch
```

Inspect artifacts under `.lumirix\runs\run_*\` (`report.md`, `risk.json`, `diff.patch`, `rollback.patch`, …).

### Important MVP limits

- **`run` wants a clean Git worktree** so diffs are attributable. Use `--allow-dirty` when demos leave the tree dirty. `status` shows dirty/clean.
- **Test evidence V1** only inspects the **top-level** wrapped command (`cargo test`, `npm test`, …). Tests launched *inside* an agent are not seen yet.
- **Rollback** reverses **tracked** Git changes via `rollback.patch`. Untracked files may remain (partial rollback).
- Recommendations are **heuristic**, never absolute (“definitely safe”).

## Commands

| Command | Description |
|---------|-------------|
| `lumirix init` | Create `.lumirix/` |
| `lumirix status` | Init + Git + dirty/clean + LLM setting |
| `lumirix config show` | Print config |
| `lumirix run -- <program> [args...]` | Capture run (logs, diff, risk, evidence, report) |
| `lumirix run --allow-dirty -- …` | Allow dirty worktree |
| `lumirix runs` | List runs (risk + evidence columns) |
| `lumirix show last` | Run metadata |
| `lumirix report last` | Trust report (terminal) |
| `lumirix report last --format md` | Markdown trust report |
| `lumirix report last --format json` | JSON trust report |
| `lumirix diff last` | Diff summary |
| `lumirix risks last` | Risk findings |
| `lumirix evidence last` | Evidence strength |
| `lumirix rollback last --write rollback.patch` | Export reverse patch for the run |

`lumirix run` exits with the **same code as the wrapped command**.

## Local store

```txt
.lumirix/
  config.toml
  policies/default.toml
  runs/run_YYYY_MM_DD_NNN/
    run.json
    events.jsonl
    stdout.log / stderr.log
    commands.log
    diff.patch / rollback.patch / diff_summary.json
    risk.json / tests.json / evidence.json
    report.md / report.json
  db/lumirix.sqlite
```

LLM is **disabled by default**. The deterministic core is the source of truth.

## Layout

```txt
crates/
  lumirix-cli/
  lumirix-core/
scripts/
  build.ps1
  demo.ps1
```

## License

MIT
