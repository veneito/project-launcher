# Project Launcher

A small, low-memory native desktop app (Rust + GPUI) that lists all your project folders and opens any of them in one step. Styled after Sonora: dark, teal-tinted, compact rows, sidebar navigation.

**Status: early scaffold.** The UI is not built yet.

## Plan

- Scan chosen root folders and detect project type from marker files (`Cargo.toml`, `package.json`, `.unity`, `.blend`).
- Fuzzy search, recents, pinned projects.
- Open the selected project in VS Code.
- Later: global hotkey, tray icon, per-project run commands.

## Build (Windows)

Requires Rust and the Visual Studio C++ Build Tools.

```powershell
cargo run --release
```

## License

MIT
