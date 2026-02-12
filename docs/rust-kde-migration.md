---
summary: "Rust + KDE Plasma migration plan and initial implementation layout."
read_when:
  - Porting CodexBar from macOS SwiftUI to Linux/KDE
  - Extending the Rust service and Plasma widget
---

# Rust + KDE migration

This repository now includes a first Rust/KDE migration scaffold:

- `KDE Plasma/rust/crates/codexbar-core`: snapshot/domain contract used by UI and service layers.
- `KDE Plasma/rust/crates/codexbar-service`: command that emits panel-ready JSON.
- `KDE Plasma/rust/crates/codexbar-kde-bridge`: IPC envelope and DBus naming constants.
- `KDE Plasma/org.codexbar.widget`: Plasma 6 applet package that polls `codexbar-service`.

## Migration strategy

1. Keep the current Swift app as behavioral reference.
2. Port provider logic into Rust incrementally, one provider at a time.
3. Keep Plasma UI thin: render the Rust snapshot contract only.
4. Switch transport from command polling to DBus once service behavior stabilizes.

## Why this split

- Plasma widgets are QML-based, so a pure Rust widget is not practical.
- Rust is best used for fetch/parsing/business logic and IPC.
- A stable snapshot schema allows independent iteration on service and UI.

## Immediate next steps

1. Implement provider modules in `codexbar-core` mirroring Swift strategies (`cli`, `web`, `oauth`, `api`).
2. Add async refresh loop + cache in `codexbar-service`.
3. Move widget from command polling (`executable` engine) to DBus subscription.
4. Add integration fixtures for real `codexbar --format json` payloads.
5. Add packaging flow for:
   - Rust binaries (`cargo build --release`)
   - Plasma applet (`kpackagetool6` package/install)
