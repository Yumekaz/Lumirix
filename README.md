# Lumirix

**Trust infrastructure for autonomous coding agents.**

Lumirix verifies AI-generated software changes before they are merged, deployed, or trusted.

> Generation is becoming cheap. Trust is becoming expensive.

## Status

**Phase 1 complete (Rust):** CLI skeleton — `init`, `status`, `config show`.

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
cargo run -p lumirix-cli -- init
cargo run -p lumirix-cli -- status
cargo run -p lumirix-cli -- config show
```

## Phase 1 commands

| Command | Description |
|---------|-------------|
| `lumirix init` | Create `.lumirix/` (config, default policies, SQLite, empty runs/) |
| `lumirix init --force` | Reinitialize defaults |
| `lumirix status` | Show init state, Git branch/commit, LLM setting |
| `lumirix config show` | Print `.lumirix/config.toml` |

## Local store

After `init`, Lumirix writes local state under `.lumirix/` (gitignored):

```txt
.lumirix/
  config.toml
  policies/default.toml
  runs/
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
  lumirix-core/   # paths, config, git, init, db
Cargo.toml        # workspace
```

## License

MIT
