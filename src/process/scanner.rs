use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use rayon::prelude::*;

use crate::process::types::{ProcessInfo, SystemMemoryInfo};

pub struct SystemScanner {
    prev_cpu_times: Mutex<HashMap<u32, (u64, Instant)>>,
}

impl SystemScanner {
    pub fn new() -> Self {
        Self {
            prev_cpu_times: Mutex::new(HashMap::new()),
        }
    }

    pub fn scan_system_memory() -> SystemMemoryInfo {
        let mut info = SystemMemoryInfo::default();
        if let Ok(file) = File::open("/proc/meminfo") {
            let reader = BufReader::new(file);
            let mut buffers = 0u64;
            let mut cached = 0u64;

            for line in reader.lines().flatten() {
                let mut parts = line.split_whitespace();
                if let (Some(key), Some(val_str)) = (parts.next(), parts.next()) {
                    if let Ok(val) = val_str.parse::<u64>() {
                        match key {
                            "MemTotal:" => info.total_kb = val,
                            "MemFree:" => info.free_kb = val,
                            "MemAvailable:" => info.available_kb = val,
                            "Buffers:" => buffers = val,
                            "Cached:" => cached = val,
                            "SwapTotal:" => info.swap_total_kb = val,
                            "SwapFree:" => info.swap_free_kb = val,
                            _ => {}
                        }
                    }
                }
            }
            info.buffers_cache_kb = buffers + cached;
            info.swap_used_kb = info.swap_total_kb.saturating_sub(info.swap_free_kb);
        }
        info
    }

    pub fn scan_all_processes(&self) -> (SystemMemoryInfo, Vec<ProcessInfo>) {
        let mut sys_mem = Self::scan_system_memory();

        let entries = match fs::read_dir("/proc") {
            Ok(entries) => entries,
            Err(_) => return (sys_mem, Vec::new()),
        };

        let pids: Vec<u32> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let file_name = e.file_name();
                file_name.to_str()?.parse::<u32>().ok()
            })
            .collect();

        let now = Instant::now();

        let processes: Vec<ProcessInfo> = pids
            .into_par_iter()
            .filter_map(|pid| self.scan_process(pid, now))
            .collect();

        // Calculate total PSS and RSS sum across all processes
        let mut total_pss = 0u64;
        let mut total_rss = 0u64;
        for p in &processes {
            total_pss += p.pss_kb;
            total_rss += p.rss_kb;
        }
        sys_mem.total_pss_sum_kb = total_pss;
        sys_mem.total_rss_sum_kb = total_rss;

        (sys_mem, processes)
    }

    fn scan_process(&self, pid: u32, now: Instant) -> Option<ProcessInfo> {
        let proc_path = format!("/proc/{}", pid);
        let path = Path::new(&proc_path);
        if !path.exists() {
            return None;
        }

        // 1. Read /proc/[pid]/stat
        let stat_str = fs::read_to_string(path.join("stat")).ok()?;
        let (name, ppid, utime_stime, threads) = Self::parse_stat(&stat_str)?;

        // Filter out idle kernel workers if they have 0 memory and ppid 2 (optional, but keep clean)
        // We'll keep them in System category if they have memory

        // 2. Read /proc/[pid]/cmdline
        let cmdline = match fs::read(path.join("cmdline")) {
            Ok(bytes) => {
                let s = bytes
                    .split(|&b| b == 0)
                    .filter(|part| !part.is_empty())
                    .map(|part| String::from_utf8_lossy(part).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                if s.trim().is_empty() {
                    name.clone()
                } else {
                    s
                }
            }
            Err(_) => name.clone(),
        };

        // 3. Read /proc/[pid]/exe (symlink)
        let exe = fs::read_link(path.join("exe"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // 4. Read /proc/[pid]/smaps_rollup (Super fast PSS, RSS, USS)
        let (pss_kb, rss_kb, uss_kb, swap_kb) = Self::read_smaps_rollup(pid).unwrap_or_else(|| {
            // Fallback to /proc/[pid]/status if smaps_rollup is unavailable
            Self::read_status_mem(pid)
        });

        // 5. Calculate CPU usage delta
        let cpu_usage = {
            let mut guard = self.prev_cpu_times.lock().ok();
            if let Some(ref mut map) = guard {
                if let Some((prev_ticks, prev_time)) = map.insert(pid, (utime_stime, now)) {
                    let elapsed_secs = (now - prev_time).as_secs_f32();
                    if elapsed_secs > 0.05 && utime_stime >= prev_ticks {
                        // In Linux, 100 ticks = 1 sec usually (sysconf(_SC_CLK_TCK))
                        let delta_ticks = (utime_stime - prev_ticks) as f32;
                        let usage = (delta_ticks / 100.0) / elapsed_secs * 100.0;
                        usage.clamp(0.0, 3200.0)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            }
        };

        // 6. Deduce human-friendly role hint
        let role_hint = Self::detect_role(&name, &cmdline, &exe);

        Some(ProcessInfo {
            pid,
            ppid,
            name,
            cmdline,
            exe,
            pss_kb,
            rss_kb,
            uss_kb,
            swap_kb,
            threads,
            role_hint,
            utime_stime,
            cpu_usage,
        })
    }

    fn parse_stat(stat_str: &str) -> Option<(String, u32, u64, u32)> {
        // Format: pid (comm with possible spaces) state ppid ...
        let start_paren = stat_str.find('(')?;
        let end_paren = stat_str.rfind(')')?;
        let name = stat_str[start_paren + 1..end_paren].to_string();

        let remainder = &stat_str[end_paren + 1..].trim();
        let fields: Vec<&str> = remainder.split_whitespace().collect();
        // fields[0] is state (e.g. 'S')
        // fields[1] is ppid
        // fields[11] is utime (14 in 1-based)
        // fields[12] is stime (15 in 1-based)
        // fields[17] is num_threads (20 in 1-based)

        let ppid = fields.get(1)?.parse::<u32>().ok()?;
        let utime = fields.get(11)?.parse::<u64>().unwrap_or(0);
        let stime = fields.get(12)?.parse::<u64>().unwrap_or(0);
        let threads = fields.get(17)?.parse::<u32>().unwrap_or(1);

        Some((name, ppid, utime + stime, threads))
    }

    fn read_smaps_rollup(pid: u32) -> Option<(u64, u64, u64, u64)> {
        let path = format!("/proc/{}/smaps_rollup", pid);
        let mut file = File::open(path).ok()?;
        let mut content = String::with_capacity(1024);
        file.read_to_string(&mut content).ok()?;

        let mut pss_kb = 0u64;
        let mut rss_kb = 0u64;
        let mut priv_clean = 0u64;
        let mut priv_dirty = 0u64;
        let mut swap_kb = 0u64;

        for line in content.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(key), Some(val_str)) = (parts.next(), parts.next()) {
                if let Ok(val) = val_str.parse::<u64>() {
                    match key {
                        "Pss:" => pss_kb = val,
                        "Rss:" => rss_kb = val,
                        "Private_Clean:" => priv_clean = val,
                        "Private_Dirty:" => priv_dirty = val,
                        "Swap:" | "SwapPss:" => {
                            if swap_kb == 0 {
                                swap_kb = val;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let uss_kb = priv_clean + priv_dirty;
        Some((pss_kb, rss_kb, uss_kb, swap_kb))
    }

    fn read_status_mem(pid: u32) -> (u64, u64, u64, u64) {
        let path = format!("/proc/{}/status", pid);
        let mut rss = 0u64;
        let mut swap = 0u64;

        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                if line.starts_with("VmRSS:") {
                    if let Some(val) = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok()) {
                        rss = val;
                    }
                } else if line.starts_with("VmSwap:") {
                    if let Some(val) = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok()) {
                        swap = val;
                    }
                }
            }
        }
        (rss, rss, rss, swap)
    }

    fn detect_role(name: &str, cmdline: &str, exe: &str) -> String {
        let cmd_lower = cmdline.to_lowercase();
        let exe_lower = exe.to_lowercase();

        if cmd_lower.contains("--type=gpu-process") {
            "GPU Process (Render engine / OpenGL)".to_string()
        } else if cmd_lower.contains("--type=renderer") {
            if cmd_lower.contains("--extension-process") {
                "Renderer (Browser Extension)".to_string()
            } else {
                "Renderer (Web Page / Tab Content)".to_string()
            }
        } else if cmd_lower.contains("--type=zygote") {
            "Zygote (Process Fork Template)".to_string()
        } else if cmd_lower.contains("--type=utility") {
            if cmd_lower.contains("network") {
                "Utility (Network Service)".to_string()
            } else if cmd_lower.contains("audio") {
                "Utility (Audio Service)".to_string()
            } else if cmd_lower.contains("storage") {
                "Utility (Storage Service)".to_string()
            } else {
                "Utility Process".to_string()
            }
        } else if cmd_lower.contains("crashpad-handler") {
            "Crash Reporter / Handler".to_string()
        } else if cmd_lower.contains("language_server") || cmd_lower.contains("--enable_lsp") {
            "Language Server (LSP)".to_string()
        } else if cmd_lower.contains("chrome-devtools-mcp") {
            "Chrome DevTools MCP Worker".to_string()
        } else if cmd_lower.contains("extensionhost") || cmd_lower.contains("extension-host") {
            "IDE Extension Host".to_string()
        } else if cmd_lower.contains("steamwebhelper") {
            "Steam Web Helper Worker".to_string()
        } else if cmd_lower.contains("gpu-screen-recorder") {
            "Replay / Screen Recorder Worker".to_string()
        } else if exe_lower.contains("discord") && !cmd_lower.contains("--type=") {
            "Discord Main Client".to_string()
        } else if exe_lower.contains("helium") && !cmd_lower.contains("--type=") {
            "Helium Browser Main Process".to_string()
        } else if exe_lower.contains("steam") && !cmd_lower.contains("steamwebhelper") {
            "Steam Main Client".to_string()
        } else if exe_lower.contains("dolphin") {
            "Dolphin File Manager".to_string()
        } else if name == "Hyprland" {
            "Hyprland Compositor Main".to_string()
        } else {
            "Main / Worker Process".to_string()
        }
    }
}
