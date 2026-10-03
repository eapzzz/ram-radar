//! Freedesktop application metadata. No program-specific classification rules.
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Default)]
pub struct DesktopApp {
    pub id: String,
    pub name: String,
    pub icon: Option<PathBuf>,
}
#[derive(Default)]
pub struct DesktopRegistry {
    apps: HashMap<String, DesktopApp>,
    install_dirs: Vec<(PathBuf, DesktopApp)>,
    icons: HashMap<String, PathBuf>,
}
fn executable(command: &str) -> Option<String> {
    let command = command.trim();
    let word = if let Some(s) = command.strip_prefix('"') {
        s.split('"').next()?
    } else {
        command.split_whitespace().next()?
    };
    if word.is_empty() || matches!(word, "env" | "sh" | "bash" | "flatpak") {
        return None;
    }
    Some(Path::new(word).file_name()?.to_string_lossy().into_owned())
}
fn parse_entry(text: &str) -> Option<(String, String, String)> {
    let mut section = false;
    let mut values = HashMap::new();
    for line in text.lines() {
        if line.starts_with('[') {
            section = line == "[Desktop Entry]";
            continue;
        }
        if section {
            if let Some((k, v)) = line.split_once('=') {
                values.insert(k.trim(), v.trim());
            }
        }
    }
    if values.get("Type") != Some(&"Application") || values.get("Hidden") == Some(&"true") {
        return None;
    }
    let key = values
        .get("TryExec")
        .and_then(|v| executable(v))
        .or_else(|| values.get("Exec").and_then(|v| executable(v)))?;
    Some((
        key,
        values.get("Name")?.to_string(),
        values.get("Icon").unwrap_or(&"").to_string(),
    ))
}
fn index_icons(dir: &Path, depth: usize, out: &mut HashMap<String, PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            index_icons(&p, depth - 1, out)
        } else if matches!(p.extension().and_then(|s| s.to_str()), Some("png" | "svg")) {
            if let Some(name) = p.file_stem().and_then(|s| s.to_str()) {
                out.entry(name.into()).or_insert(p);
            }
        }
    }
}
impl DesktopRegistry {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let data =
            std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));
        let mut roots = vec![PathBuf::from(data)];
        roots.extend(
            std::env::var("XDG_DATA_DIRS")
                .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        );
        roots.push(PathBuf::from(format!(
            "{home}/.local/share/flatpak/exports/share"
        )));
        roots.push(PathBuf::from("/var/lib/flatpak/exports/share"));
        let mut icons = HashMap::new();
        for root in &roots {
            for theme in [
                "hicolor/48x48/apps",
                "hicolor/64x64/apps",
                "hicolor/128x128/apps",
                "hicolor/scalable/apps",
                "breeze/apps/48",
                "hicolor",
                "breeze/apps",
            ] {
                index_icons(&root.join("icons").join(theme), 5, &mut icons);
            }
            index_icons(&root.join("pixmaps"), 2, &mut icons);
        }
        let mut registry = Self::default();
        for root in roots {
            let Ok(entries) = fs::read_dir(root.join("applications")) else {
                continue;
            };
            let mut files: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
            files.sort();
            for path in files {
                if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else {
                    continue;
                };
                if let Some((key, name, icon)) = parse_entry(&text) {
                    let icon = if Path::new(&icon).is_absolute() && Path::new(&icon).is_file() {
                        Some(PathBuf::from(icon))
                    } else {
                        icons.get(&icon).cloned()
                    };
                    let app = DesktopApp {
                        id: key.to_lowercase(),
                        name,
                        icon,
                    };
                    registry
                        .apps
                        .entry(key.to_lowercase())
                        .or_insert(app.clone());
                    // A launcher symlink identifies its installation directory without executing it.
                    let launcher = std::env::var_os("PATH")
                        .into_iter()
                        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
                        .map(|p| p.join(&key))
                        .find(|p| p.is_file());
                    if let Some(parent) = launcher
                        .and_then(|p| fs::canonicalize(p).ok())
                        .and_then(|p| p.parent().map(Path::to_path_buf))
                    {
                        if parent.starts_with("/opt")
                            || (parent.starts_with("/usr/lib") && parent != Path::new("/usr/lib"))
                        {
                            registry.install_dirs.push((parent, app));
                        }
                    }
                }
            }
        }
        registry.icons = icons;
        registry
    }
    pub fn icon_for(&self, exe: &str) -> Option<&Path> {
        if let Some(icon) = self.lookup(exe).and_then(|app| app.icon.as_deref()) {
            return Some(icon);
        }
        let name = Path::new(exe).file_name()?.to_str()?.to_lowercase();
        let base = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-');
        self.icons
            .get(&name)
            .or_else(|| self.icons.get(base))
            .map(PathBuf::as_path)
    }
    pub fn group_id(&self, exe: &str) -> Option<&str> {
        let path = Path::new(exe.trim_end_matches(" (deleted)"));
        self.install_dirs
            .iter()
            .filter(|(dir, _)| path.starts_with(dir))
            .max_by_key(|(dir, _)| dir.components().count())
            .map(|(_, app)| app.id.as_str())
    }
    pub fn lookup(&self, exe: &str) -> Option<&DesktopApp> {
        if let Some(id) = exe.strip_prefix("app:") {
            return self.apps.get(id);
        }
        let path = Path::new(exe.trim_end_matches(" (deleted)"));
        self.apps
            .get(&path.file_name()?.to_str()?.to_lowercase())
            .or_else(|| {
                self.install_dirs
                    .iter()
                    .filter(|(dir, _)| path.starts_with(dir))
                    .max_by_key(|(dir, _)| dir.components().count())
                    .map(|(_, app)| app)
            })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basename_metadata_never_changes_executable_group_identity() {
        let mut registry = DesktopRegistry::default();
        let app = DesktopApp {
            id: "app".into(),
            name: "App".into(),
            icon: None,
        };
        registry.apps.insert("app".into(), app.clone());
        registry.install_dirs.push((PathBuf::from("/opt/app"), app));
        assert!(registry.lookup("/one/app").is_some());
        assert!(registry.group_id("/one/app").is_none());
        assert!(registry.group_id("/two/app").is_none());
        assert_eq!(registry.group_id("/opt/app/helper"), Some("app"));
        assert!(registry.group_id("/opt/application/helper").is_none());
        let process = |exe: &str, pid: u32| crate::process::types::ProcessInfo {
            pid,
            ppid: 1,
            name: "app".into(),
            cmdline: String::new(),
            exe: exe.into(),
            pss_kb: 10,
            rss_kb: 10,
            uss_kb: 10,
            swap_kb: 0,
            threads: 1,
            role_hint: String::new(),
            utime_stime: 0,
            cpu_usage: 0.0,
            pss_estimated: false,
            starttime: 1,
        };
        let groups = crate::process::classifier::ProcessClassifier::group_processes(
            vec![process("/one/app", 1), process("/two/app", 2)],
            &registry,
        );
        assert_eq!(groups.len(), 2);
        let groups = crate::process::classifier::ProcessClassifier::group_processes(
            vec![process("/opt/app/app", 1), process("/opt/app/helper", 2)],
            &registry,
        );
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].total_pss_kb, 20);
    }
    #[test]
    fn desktop_actions_do_not_override_app() {
        let e=parse_entry("[Desktop Entry]\nType=Application\nName=Browser\nExec=\"/opt/browser app/bin/browser\" %U\nIcon=browser\n[Desktop Action New]\nName=Wrong\nExec=wrong").unwrap();
        assert_eq!(e.0, "browser");
        assert_eq!(e.1, "Browser");
    }
    #[test]
    fn wrappers_are_not_assigned_to_every_process() {
        assert!(parse_entry("[Desktop Entry]\nType=Application\nName=X\nExec=sh -c x").is_none());
    }
    #[test]
    fn hidden_entry_is_ignored() {
        assert!(
            parse_entry("[Desktop Entry]\nType=Application\nHidden=true\nName=X\nExec=x").is_none()
        );
    }
}
