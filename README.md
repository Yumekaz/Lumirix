# Lumirix

**Trust infrastructure for autonomous coding agents.**

Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.

> Generation is becoming cheap. Trust is becoming expensive.

## Status

Rust CLI: project init, status, config, **agent run capture**, and run inspection.

## Requirements

- Rust toolchain (edition 2021)
- **Windows:** Visual Studio Build Tools 2022 with C++ / MSVC + Windows SDK
- **macOS/Linux:** standard system linker (`clang`/`gcc`)
- Git (for full status; init works without Git in limited mode)

## Build

```bash
cargo build -p lumirix-cli
```

### Windows (MSVC)

Use an **x64 Native Tools Command Prompt for VS 2022**, or load the MSVC environment first:

```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build -p lumirix-cli
```

## Install

```bash
cargo install --path crates/lumirix-cli
```

Or run without installing:

```bash
cargo run -p lumirix-cli -- <command>
```

## Commands

| Command | Description |
|---------|-------------|
| `lumirix init` | Create `.lumirix/` (config, default policies, SQLite, empty runs/) |
| `lumirix init --force` | Reinitialize defaults |
| `lumirix status` | Show init state, Git branch/commit, LLM setting |
| `lumirix config show` | Print `.lumirix/config.toml` |
| `lumirix run -- <program> [args...]` | Run a command under capture (tees stdout/stderr) |
| `lumirix runs` | List captured runs (newest first) |
| `lumirix show last` | Show metadata for the last run (or a run id) |
| `lumirix report last` | Minimal run report (exit status + log paths) |

### Examples

```bash
lumirix init
lumirix run -- git --version
lumirix runs
lumirix show last
lumirix report last
```

On Windows, prefer real executables (e.g. `git`, `cmd /C echo hello`) rather than shell builtins alone.

Optional task label:

```bash
lumirix run --task "smoke test" -- git status
```

`lumirix run` exits with the **same code as the wrapped command**.

## Local store

After `init` / `run`, Lumirix writes local state under `.lumirix/` (gitignored):

```txt
.lumirix/
  config.toml
  policies/default.toml
  runs/
    run_YYYY_MM_DD_NNN/
      run.json
      events.jsonl
      stdout.log
      stderr.log
      commands.log
  db/lumirix.sqlite
  cache/
  snapshots/
  artifacts/
```

LLM is **disabled by default**. The deterministic core is the source of truth.

## Layout

```txt
crates/
  lumirix-cli/    # CLI binary (`lumirix`)
  lumirix-core/   # paths, config, git, init, db, run capture
Cargo.toml        # workspace
```

## License

MIT
