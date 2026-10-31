use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub kind: &'static str,
    pub branch: Option<String>,
    pub modified: SystemTime,
}

// shortcut: one hard-coded root, no settings file yet; add a roots setting when you keep projects elsewhere.
pub fn roots() -> Vec<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(|home| vec![PathBuf::from(home).join("Documents").join("Claude")])
        .unwrap_or_default()
}

/// Every direct subfolder of each root is a project, newest activity first.
pub fn scan(roots: &[PathBuf]) -> Vec<Project> {
    let mut found: Vec<Project> = roots
        .iter()
        .filter_map(|root| fs::read_dir(root).ok())
        .flatten()
        .flatten()
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && !entry.file_name().to_string_lossy().starts_with('.')
        })
        .map(|entry| project(&entry.path()))
        .collect();
    found.sort_by(|a, b| b.modified.cmp(&a.modified));
    found
}

fn project(path: &Path) -> Project {
    let mtime = |p: PathBuf| fs::metadata(p).and_then(|m| m.modified()).ok();
    Project {
        name: path.file_name().unwrap_or_default().to_string_lossy().into(),
        path: path.to_path_buf(),
        kind: kind(path),
        branch: fs::read_to_string(path.join(".git/HEAD"))
            .ok()
            .and_then(|head| head.trim().strip_prefix("ref: refs/heads/").map(str::to_owned)),
        modified: mtime(path.join(".git/index"))
            .max(mtime(path.to_path_buf()))
            .unwrap_or(UNIX_EPOCH),
    }
}

fn kind(dir: &Path) -> &'static str {
    let has = |name: &str| dir.join(name).exists();
    let ext = |want: &str| {
        fs::read_dir(dir).is_ok_and(|files| {
            files
                .flatten()
                .any(|file| file.path().extension().is_some_and(|e| e == want))
        })
    };
    if has("Cargo.toml") {
        "Rust"
    } else if has("project.godot") {
        "Godot"
    } else if has("ProjectSettings") {
        "Unity"
    } else if has("package.json") {
        "Node"
    } else if has("pyproject.toml") || has("requirements.txt") || has("app.py") {
        "Python"
    } else if ext("sln") || ext("csproj") {
        ".NET"
    } else if ext("rbxl") || has("default.project.json") {
        "Roblox"
    } else if has("index.html") {
        "Web"
    } else {
        "Other"
    }
}

/// True when the query's letters appear in order in `text`, ignoring case and spaces.
pub fn fuzzy(query: &str, text: &str) -> bool {
    let mut text = text.chars().flat_map(char::to_lowercase);
    query
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| !c.is_whitespace())
        .all(|q| text.any(|c| c == q))
}

pub fn ago(time: SystemTime) -> String {
    let secs = SystemTime::now()
        .duration_since(time)
        .unwrap_or_default()
        .as_secs();
    match secs {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86399 => format!("{}h ago", secs / 3600),
        86400..=2591999 => format!("{}d ago", secs / 86400),
        _ => format!("{}mo ago", secs / 2592000),
    }
}

pub fn open_in_code(path: &Path) -> Result<(), String> {
    let mut command = std::process::Command::new("code.cmd");
    command.arg(path);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    command
        .spawn()
        .map(drop)
        .map_err(|_| "Could not start VS Code. Is `code` on your PATH?".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn fuzzy_matches_letters_in_order() {
        assert!(fuzzy("srvdash", "Server Dashboard"));
        assert!(fuzzy("", "anything"));
        assert!(!fuzzy("dashserver", "Server Dashboard"));
    }

    #[test]
    fn age_buckets() {
        let ago_secs = |s| ago(SystemTime::now() - Duration::from_secs(s));
        assert_eq!(ago_secs(5), "just now");
        assert_eq!(ago_secs(120), "2m ago");
        assert_eq!(ago_secs(7200), "2h ago");
        assert_eq!(ago_secs(172800), "2d ago");
    }

    #[test]
    fn scan_detects_kind_and_branch() {
        let root = std::env::temp_dir().join(format!("pl-test-{}", std::process::id()));
        let rust = root.join("rusty");
        fs::create_dir_all(rust.join(".git")).unwrap();
        fs::write(rust.join("Cargo.toml"), "").unwrap();
        fs::write(rust.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::create_dir_all(root.join("plain")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        let found = scan(&[root.clone()]);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(found.len(), 2);
        let rusty = found.iter().find(|p| p.name == "rusty").unwrap();
        assert_eq!((rusty.kind, rusty.branch.as_deref()), ("Rust", Some("main")));
        assert_eq!(found.iter().find(|p| p.name == "plain").unwrap().kind, "Other");
    }
}
