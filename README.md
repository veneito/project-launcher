# Project Launcher

A small, low-memory native desktop app (Rust + GPUI) that lists all your project folders and opens any of them in one step. Styled after Sonora: dark, teal-tinted, compact rows, sidebar navigation.

**Status: first working version.**

## Works now

- Scans `Documents\Claude` and lists each subfolder as a project, newest activity first.
- Detects project type from marker files (Rust, Node, Python, Godot, Unity, .NET, Roblox, Web) and shows the git branch.
- Fuzzy search; Enter opens the first match, clicking a row opens it in VS Code.
- Type filters in the sidebar, Sonora-style glass window (reused from my Server Dashboard app).

## Planned

- Configurable root folders, pinned projects.
- Global hotkey, tray icon, per-project run commands.

## Build (Windows)

Requires Rust and the Visual Studio C++ Build Tools.

```powershell
cargo run --release
```

## License

MIT
