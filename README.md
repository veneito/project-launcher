# Project Launcher

A small, low-memory native desktop app (Rust + GPUI) that lists all your project folders and opens any of them in one step. Styled after Sonora: dark, teal-tinted, compact rows, sidebar navigation.

**Status: first working version.**

## Works now

- Scans `Documents\Claude` and lists each subfolder as a project, newest activity first.
- Detects project type from marker files (Rust, Node, Python, Godot, Unity, .NET, Roblox, Web) and shows the git branch.
- Sonora-style layout: sidebar (Home, Search, Your Library by type, Recent, Settings, Pinned), project table, details panel, and a bottom bar.
- Fuzzy search; click a row to select it, double-click or Enter to open it.
- Every type has a default open command (VS Code, File Explorer, Terminal, or any custom command with `{path}`), and each project can override its type and its command.
- Pin projects, and see recently opened ones.
- Settings page: folders to scan, open commands per type, and the look (Solid or Glass, tint strength, color presets or custom hex colors). Saved in `%LOCALAPPDATA%\ProjectLauncher`.

## Planned

- Sortable columns, global hotkey, tray icon.
- Global hotkey, tray icon, per-project run commands.

## Build (Windows)

Requires Rust and the Visual Studio C++ Build Tools.

```powershell
cargo run --release
```

## License

MIT
