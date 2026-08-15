use std::collections::HashMap;
use crate::process::types::{AppGroup, Category, ProcessInfo};

pub struct ProcessClassifier;

impl ProcessClassifier {
    pub fn group_processes(processes: Vec<ProcessInfo>) -> Vec<AppGroup> {
        let mut app_groups: HashMap<String, AppGroup> = HashMap::new();

        // 1. First pass: classify each process into a designated group key
        for p in processes {
            let (key, display_name, category, icon, color) = Self::classify_process(&p);
            
            let group = app_groups.entry(key.clone()).or_insert_with(|| {
                AppGroup::new(key, display_name, category, icon, color)
            });

            group.processes.push(p);
        }

        // 2. Second pass: recompute totals and sort internal processes
        let mut result: Vec<AppGroup> = app_groups.into_values().collect();
        for group in &mut result {
            // Sort processes inside group: largest PSS first
            group.processes.sort_by(|a, b| b.pss_kb.cmp(&a.pss_kb));
            group.recompute_totals();
        }

        // 3. Sort groups by PSS descending by default
        result.sort_by(|a, b| b.total_pss_kb.cmp(&a.total_pss_kb));

        result
    }

    fn classify_process(p: &ProcessInfo) -> (String, String, Category, &'static str, [u8; 3]) {
        let cmd = p.cmdline.to_lowercase();
        let exe = p.exe.to_lowercase();
        let name = p.name.to_lowercase();

        // 1. Antigravity IDE & AI Suite
        if exe.contains("antigravity-ide") || cmd.contains("antigravity-ide") {
            return (
                "antigravity_ide".to_string(),
                "Antigravity IDE".to_string(),
                Category::Apps,
                "⚡",
                [168, 85, 247], // Purple
            );
        }
        if exe.contains("antigravity") || cmd.contains("/opt/antigravity/") || cmd.contains("language_server") || cmd.contains("chrome-devtools-mcp") {
            return (
                "antigravity_core".to_string(),
                "Antigravity Agent & LSP Hub".to_string(),
                Category::Apps,
                "✨",
                [147, 51, 234], // Deep Purple
            );
        }

        // 2. Helium Browser
        if exe.contains("helium") || cmd.contains("helium") || name.contains("helium") {
            return (
                "helium_browser".to_string(),
                "Helium Browser".to_string(),
                Category::Apps,
                "🌐",
                [56, 189, 248], // Sky Blue
            );
        }

        // 3. Discord
        if exe.contains("discord") || cmd.contains("discord") || name.contains("discord") {
            return (
                "discord".to_string(),
                "Discord".to_string(),
                Category::Apps,
                "💬",
                [88, 101, 242], // Discord Blurple
            );
        }

        // 4. Claude Desktop & Claude Code
        if exe.contains("claude") || cmd.contains("claude-desktop") || cmd.contains("claude-code") || name.contains("claude") {
            return (
                "claude".to_string(),
                "Claude Desktop / Code".to_string(),
                Category::Apps,
                "🤖",
                [249, 115, 22], // Orange
            );
        }

        // 5. Steam & Gaming
        if exe.contains("steam") || cmd.contains("steamwebhelper") || name.contains("steam") || cmd.contains("gamescope") {
            return (
                "steam".to_string(),
                "Steam Client & WebHelper".to_string(),
                Category::Apps,
                "🎮",
                [30, 90, 160], // Steam Blue
            );
        }

        // 6. Dolphin File Manager
        if exe.contains("dolphin") || name.contains("dolphin") || cmd.contains("dolphin") {
            return (
                "dolphin".to_string(),
                "Dolphin File Manager".to_string(),
                Category::Apps,
                "📁",
                [14, 165, 233], // Cyan
            );
        }

        // 7. GPU Screen Recorder & Media
        if name.contains("gpu-screen-rec") || cmd.contains("gpu-screen-recorder") {
            return (
                "gpu_screen_recorder".to_string(),
                "GPU Screen Recorder (RAM Buffer)".to_string(),
                Category::Desktop,
                "🎥",
                [239, 68, 68], // Red
            );
        }

        // 8. Hyprland & Wayland Desktop Stack
        if name == "hyprland" || cmd.contains("hyprland") || name.starts_with("hypr") 
            || name == "waybar" || name == "swaybg" || name == "swaync" 
            || name == "rofi" || name == "wofi" || name == "mako" || name == "dunst"
            || name.contains("wireplumber") || name.contains("pipewire")
            || name.contains("xdg-desktop-por")
            || name.contains("polkit-kde") || name == "xwayland" {
            return (
                "hyprland_desktop".to_string(),
                "Hyprland Compositor & Shell".to_string(),
                Category::Desktop,
                "🪟",
                [16, 185, 129], // Emerald
            );
        }

        // 9. Other Web Browsers & Media
        if exe.contains("firefox") || name.contains("firefox") {
            return ("firefox".to_string(), "Firefox Browser".to_string(), Category::Apps, "🦊", [249, 115, 22]);
        }
        if exe.contains("chrome") || name.contains("chrome") {
            return ("chrome".to_string(), "Google Chrome".to_string(), Category::Apps, "🌐", [234, 179, 8]);
        }
        if exe.contains("brave") || name.contains("brave") {
            return ("brave".to_string(), "Brave Browser".to_string(), Category::Apps, "🦁", [249, 115, 22]);
        }
        if exe.contains("spotify") || name.contains("spotify") {
            return ("spotify".to_string(), "Spotify Music".to_string(), Category::Apps, "🎵", [34, 197, 94]);
        }

        // 10. Terminals & Shells
        if name == "kitty" || name == "alacritty" || name == "foot" || name == "wezterm" 
            || name == "zsh" || name == "bash" || name == "fish" || name == "tmux" {
            return (
                "terminal_shell".to_string(),
                "Terminals & Shells (CLI)".to_string(),
                Category::Development,
                "💻",
                [245, 158, 11], // Amber
            );
        }

        // 11. Compilers & Build Tools
        if name.starts_with("rustc") || name.starts_with("cargo") || name == "gcc" || name == "clang" || name == "cc1" || name == "ld" {
            return (
                "build_tools".to_string(),
                "Compilers & Build Tools (Rust/GCC/LLVM)".to_string(),
                Category::Development,
                "🔨",
                [234, 88, 12],
            );
        }
        if name == "node" || name.starts_with("python") {
            return (
                format!("runtime_{}", name),
                format!("Runtime Environment ({})", name),
                Category::Development,
                "📦",
                [59, 130, 246],
            );
        }

        // 12. System & Kernel Daemons
        if p.ppid == 2 || name.starts_with("kworker") || name.starts_with("ksoftirqd") || name.starts_with("rcu") {
            return (
                "kernel_threads".to_string(),
                "Linux Kernel (kworkers & system threads)".to_string(),
                Category::System,
                "🐧",
                [100, 116, 139], // Slate
            );
        }

        if name.starts_with("systemd") || name == "dbus-broker" || name == "dbus-daemon" 
            || name == "networkmanager" || name == "polkitd" || name == "udevd" || name == "upowerd"
            || name == "accounts-daemon" || name == "avahi-daemon" || name == "rtkit-daemon" {
            return (
                "system_services".to_string(),
                "System Core Services (systemd / D-Bus / Network)".to_string(),
                Category::System,
                "🛡️",
                [148, 163, 184],
            );
        }

        // 13. Fallback: Group by executable base name or process name
        let clean_name = if !p.name.is_empty() {
            p.name.clone()
        } else if !p.exe.is_empty() {
            p.exe.rsplit('/').next().unwrap_or("Process").to_string()
        } else {
            format!("PID_{}", p.pid)
        };

        let is_user_app = !p.exe.is_empty() && (p.exe.starts_with("/home/") || p.exe.starts_with("/opt/") || p.exe.starts_with("/usr/bin/"));
        let cat = if is_user_app {
            Category::Background
        } else {
            Category::System
        };

        (
            format!("proc_{}", clean_name),
            clean_name,
            cat,
            if is_user_app { "⚙️" } else { "🔧" },
            [156, 163, 175],
        )
    }
}
