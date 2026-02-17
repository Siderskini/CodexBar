# CodexBar Rust Workspace

Disclaimer: The majority of the work in this fork (including this README) has been done via Codex and there is a scancode toolkit result published in this repository for that work

## Files Accessed by the Rust Extension

For the Rust components in `KDE Plasma/rust`, file access is limited to:

- `codexbar-service` executable path itself (invoked by the widget command you configure).
- `codexbar` executable file in the same directory as `codexbar-service` (if present), otherwise `codexbar` resolved from `PATH`.
- `~/.claude/.credentials.json` (read-only, to load Claude OAuth tokens produced by `claude auth login`).
- `secret-tool` executable from `PATH` (preferred secure store backend for Claude credentials).
- `kwallet-query` executable from `PATH` (KDE Wallet secure store fallback for Claude credentials).
- `--input <path>`: reads only the file at `<path>` (optional, when this flag is used).
- `--write-cache <path>`: writes only to `<path>` and may create its parent directory (optional, when this flag is used).

Credential storage is handled through system keyrings (`secret-tool` or KDE Wallet via `kwallet-query`), not plaintext files.
No other fixed file paths are hardcoded by the Rust code in this repository.  
Note: `codexbar` calls external `codex` and `claude` CLIs; any extra file access from those programs is outside this project.

This workspace is the starting point for a Linux/KDE-native rebuild:

- `crates/codexbar-core`: shared snapshot/domain models.
- `crates/codexbar-cli`: Rust `codexbar` CLI bootstrap (usage output contract).
- `crates/codexbar-service`: CLI service that emits Plasma-friendly JSON snapshots.
- `crates/codexbar-kde-bridge`: IPC/DBus boundary contract for future live transport.

## Local build

```bash
cd "KDE Plasma/rust"
cargo fmt
cargo check
```

## Build CLIs

```bash
cd "KDE Plasma/rust"
cargo build --release -p codexbar-cli -p codexbar-service
```

## Emit a snapshot

Sample snapshot:

```bash
cd "KDE Plasma/rust"
cargo run -p codexbar-service -- snapshot --pretty
```

From installed `codexbar` CLI:

```bash
cd "KDE Plasma/rust"
cargo run -p codexbar-service -- snapshot --from-codexbar-cli --provider all --status --pretty
```

## Claude account setup

Browser-based setup (used by widget "Add Account..."):

```bash
codexbar-service auth --provider claude
```

Direct CLI equivalent:

```bash
codexbar auth --provider claude
```

## License Scan

A ScanCode license scan of the KDE Plasma folder was run with:

```bash
scancode -clpieu --json-pp codexbar.json /home/sidd/Documents/GitHub/CodexBar/KDE\ Plasma
```

An additional License scan for rust was run via:

```bash
cd KDE\ Plasma/rust
cargo license
```

and the current output should be

```
 $ cargo license
(Apache-2.0 OR MIT) AND Unicode-3.0 (1): unicode-ident
Apache-2.0 OR MIT (25): anstream, anstyle, anstyle-parse, anstyle-query, anstyle-wincon, anyhow, clap, clap_builder, clap_derive, clap_lex, colorchoice, heck, is_terminal_polyfill, itoa, once_cell_polyfill, proc-macro2, quote, serde, serde_core, serde_derive, serde_json, syn, utf8parse, windows-link, windows-sys
MIT (6): codexbar-cli, codexbar-core, codexbar-kde-bridge, codexbar-service, strsim, zmij
MIT OR Unlicense (1): memchr
```

Results are saved in `KDE Plasma/codexbar.json`.
