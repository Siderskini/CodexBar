# CodexBar Rust Workspace

Disclaimer: The majority of the work in this fork (including this README) has been done via Codex and there is a scancode toolkit result published in this repository for that work

## Files Accessed by the Rust Extension

For the Rust components in `KDE Plasma/rust`, file access is limited to:

- `codexbar-service` executable path itself (invoked by the widget command you configure).
- `codexbar` executable file in the same directory as `codexbar-service` (if present), otherwise `codexbar` resolved from `PATH`.
- `--input <path>`: reads only the file at `<path>` (optional, when this flag is used).
- `--write-cache <path>`: writes only to `<path>` and may create its parent directory (optional, when this flag is used).

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

## License Scan

A ScanCode license scan of the KDE Plasma folder was run with:

```bash
scancode -clpieu --json-pp codexbar.json /home/sidd/Documents/GitHub/CodexBar/KDE\ Plasma
```

Results are saved in `KDE Plasma/codexbar.json`.
