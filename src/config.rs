use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

/// Types a project can be filed under. Auto-detection picks most of them; "Game" and "Other" are for manual filing.
pub const KINDS: [&str; 10] = [
    "Rust", "Node", "Python", "Roblox", "Godot", "Unity", ".NET", "Web", "Game", "Other",
];

pub const VSCODE: &str = "code.cmd {path}";
pub const EXPLORER: &str = "explorer {path}";
pub const TERMINAL: &str = "wt.exe -d {path}";
pub const PRESETS: [(&str, &str); 3] = [("Code", VSCODE), ("Folder", EXPLORER), ("Terminal", TERMINAL)];

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub kind: Option<String>,
    pub command: Option<String>,
    pub pinned: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Open command per type, e.g. `"Rust": "code.cmd {path}"`.
    pub commands: BTreeMap<String, String>,
    /// Per-project overrides, keyed by folder path.
    pub projects: BTreeMap<String, ProjectConfig>,
    /// Folder paths, most recently opened first.
    pub recent: Vec<String>,
}

fn file() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("ProjectLauncher/config.json"))
}

impl Config {
    pub fn load() -> Self {
        file()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&mut self) -> Result<(), String> {
        self.projects.retain(|_, config| *config != ProjectConfig::default());
        self.commands.retain(|_, command| !command.trim().is_empty());
        let path = file().ok_or("Cannot find the local settings folder.")?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|_| "Could not create the settings folder.")?;
        fs::write(path, serde_json::to_vec_pretty(self).unwrap()).map_err(|_| "Could not save settings.".into())
    }

    /// The project's own command, else its type's command, else VS Code.
    pub fn command_for(&self, kind: &str, path: &str) -> String {
        self.projects
            .get(path)
            .and_then(|project| project.command.clone())
            .or_else(|| self.commands.get(kind).cloned())
            .unwrap_or_else(|| VSCODE.into())
    }

    pub fn default_command(&self, kind: &str) -> String {
        self.commands.get(kind).cloned().unwrap_or_else(|| VSCODE.into())
    }

    pub fn is_pinned(&self, path: &str) -> bool {
        self.projects.get(path).is_some_and(|project| project.pinned)
    }

    pub fn project_mut(&mut self, path: &str) -> &mut ProjectConfig {
        self.projects.entry(path.to_owned()).or_default()
    }

    pub fn touch_recent(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_owned());
        self.recent.truncate(12);
    }
}

/// Splits a command template into a program and arguments, replacing `{path}`. Double quotes group words.
pub fn parse_command(template: &str, path: &str) -> Result<(String, Vec<String>), String> {
    let mut words: Vec<String> = vec![];
    let (mut word, mut quoted, mut started) = (String::new(), false, false);
    for c in template.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    let mut words = words.into_iter().map(|w| w.replace("{path}", path));
    let program = words.next().ok_or("The open command is empty.")?;
    Ok((program, words.collect()))
}

pub fn launch(template: &str, path: &str) -> Result<(), String> {
    let (program, args) = parse_command(template, path)?;
    let mut command = std::process::Command::new(&program);
    command.args(args);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    command
        .spawn()
        .map(drop)
        .map_err(|_| format!("Could not start `{program}`. Check the open command in Settings."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_split_on_spaces_and_quotes() {
        let (program, args) = parse_command(r#""C:\Program Files\Unity\Unity.exe" -projectPath {path} -quit"#, r"D:\My Game").unwrap();
        assert_eq!(program, r"C:\Program Files\Unity\Unity.exe");
        assert_eq!(args, vec!["-projectPath", r"D:\My Game", "-quit"]);
        assert_eq!(parse_command("code.cmd {path}", "x y").unwrap().1, vec!["x y"]);
        assert!(parse_command("   ", "x").is_err());
    }

    #[test]
    fn project_command_beats_type_command_beats_vscode() {
        let mut config = Config::default();
        assert_eq!(config.command_for("Rust", "a"), VSCODE);
        config.commands.insert("Rust".into(), EXPLORER.into());
        assert_eq!(config.command_for("Rust", "a"), EXPLORER);
        config.project_mut("a").command = Some(TERMINAL.into());
        assert_eq!(config.command_for("Rust", "a"), TERMINAL);
        assert_eq!(config.command_for("Rust", "b"), EXPLORER);
    }

    #[test]
    fn recent_is_newest_first_without_repeats() {
        let mut config = Config::default();
        for path in ["a", "b", "a"] {
            config.touch_recent(path);
        }
        assert_eq!(config.recent, vec!["a", "b"]);
    }
}
