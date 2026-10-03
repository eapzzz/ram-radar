use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::process::types::{ProcessInfo, SystemMemoryInfo};

/// Previous CPU sample for a PID: (process start time in ticks, cumulative
/// utime+stime, when it was taken). The start time lets us detect PID reuse.
type CpuSample = (u64, u64, Instant);

/// Clock ticks per second (USER_HZ). `/proc/[pid]/stat` reports CPU time in
/// these units; it is 100 on most Linux builds but not guaranteed.
fn clk_tck() -> f32 {
    static CACHE: OnceLock<f32> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let v = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        if v > 0 {
            v as f32
        } else {
            100.0
        }
    })
}

/// Number of online CPUs — the ceiling for a legitimate CPU percentage.
fn num_cpus() -> f32 {
    static CACHE: OnceLock<f32> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let v = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
        if v > 0 {
            v as f32
        } else {
            1.0
        }
    })
}

pub struct SystemScanner {
    prev_cpu_times: Mutex<HashMap<u32, CpuSample>>,
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

            for line in reader.lines().map_while(Result::ok) {
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
            .into_iter()
            .filter_map(|pid| self.scan_process(pid, now))
            .collect();

        // Calculate total PSS and RSS sum across all processes
        let mut total_pss = 0u64;
        let mut total_rss = 0u64;
        let mut estimated_pss = 0u64;
        let mut estimated_procs = 0usize;
        for p in &processes {
            total_pss += p.pss_kb;
            total_rss += p.rss_kb;
            if p.pss_estimated {
                estimated_pss += p.pss_kb;
                estimated_procs += 1;
            }
        }
        sys_mem.total_pss_sum_kb = total_pss;
        sys_mem.total_rss_sum_kb = total_rss;
        sys_mem.estimated_pss_kb = estimated_pss;
        sys_mem.estimated_proc_count = estimated_procs;

        // Drop CPU history for PIDs that no longer exist, otherwise the map
        // grows without bound across a long session.
        if let Ok(mut map) = self.prev_cpu_times.lock() {
            let live: HashSet<u32> = processes.iter().map(|p| p.pid).collect();
            map.retain(|pid, _| live.contains(pid));
        }

        (sys_mem, processes)
    }

    fn scan_process(&self, pid: u32, now: Instant) -> Option<ProcessInfo> {
        let proc_path = format!("/proc/{}", pid);
        let path = Path::new(&proc_path);

        // 1. Read /proc/[pid]/stat (also proves the process is still alive)
        let stat_str = fs::read_to_string(path.join("stat")).ok()?;
        let (name, ppid, utime_stime, threads, starttime) = Self::parse_stat(&stat_str)?;

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

        // 4. Read /proc/[pid]/smaps_rollup (Fast PSS, RSS, USS).
        // Unreadable for processes we do not own, in which case we fall back to
        // VmRSS and flag the numbers as an approximation.
        let (pss_kb, rss_kb, uss_kb, swap_kb, pss_estimated) = match Self::read_smaps_rollup(pid) {
            Some((pss, rss, uss, swap)) => (pss, rss, uss, swap, false),
            None => {
                let (rss, swap) = Self::read_status_mem(pid);
                // Kernel threads have no smaps_rollup *and* no VmRSS; with
                // nothing to over-count they are not worth flagging.
                (rss, rss, rss, swap, rss > 0)
            }
        };

        // 5. Calculate CPU usage delta
        let cpu_usage = {
            let mut guard = self.prev_cpu_times.lock().ok();
            if let Some(ref mut map) = guard {
                match map.insert(pid, (starttime, utime_stime, now)) {
                    // Same PID *and* same start time: a genuine previous sample.
                    // A differing start time means the PID was recycled, so the
                    // tick delta would be meaningless.
                    Some((prev_start, prev_ticks, prev_time))
                        if prev_start == starttime && utime_stime >= prev_ticks =>
                    {
                        let elapsed_secs = (now - prev_time).as_secs_f32();
                        if elapsed_secs > 0.05 {
                            let delta_ticks = (utime_stime - prev_ticks) as f32;
                            let usage = (delta_ticks / clk_tck()) / elapsed_secs * 100.0;
                            usage.clamp(0.0, num_cpus() * 100.0)
                        } else {
                            0.0
                        }
                    }
                    _ => 0.0,
                }
            } else {
                0.0
            }
        };

        // 6. Deduce human-friendly role hint
        let role_hint = Self::detect_role(&name, &cmdline, &exe);

        Some(ProcessInfo {
            starttime,
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
            pss_estimated,
        })
    }

    /// Returns (name, ppid, utime+stime, threads, starttime).
    /// `fields` is indexed from the field after the closing paren, i.e.
    /// `fields[i]` is proc(5) field number `i + 3`.
    fn parse_stat(stat_str: &str) -> Option<(String, u32, u64, u32, u64)> {
        let start_paren = stat_str.find('(')?;
        let end_paren = stat_str.rfind(')')?;
        let name = stat_str[start_paren + 1..end_paren].to_string();

        let remainder = &stat_str[end_paren + 1..].trim();
        let fields: Vec<&str> = remainder.split_whitespace().collect();

        let ppid = fields.get(1)?.parse::<u32>().ok()?;
        let utime = fields.get(11)?.parse::<u64>().unwrap_or(0);
        let stime = fields.get(12)?.parse::<u64>().unwrap_or(0);
        let threads = fields.get(17)?.parse::<u32>().unwrap_or(1);
        let starttime = fields
            .get(19)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Some((name, ppid, utime + stime, threads, starttime))
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
        let mut swap_pss_kb = None;

        for line in content.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(key), Some(val_str)) = (parts.next(), parts.next()) {
                if let Ok(val) = val_str.parse::<u64>() {
                    match key {
                        "Pss:" => pss_kb = val,
                        "Rss:" => rss_kb = val,
                        "Private_Clean:" => priv_clean = val,
                        "Private_Dirty:" => priv_dirty = val,
                        "Swap:" => swap_kb = val,
                        // Proportional swap; preferred when the kernel exposes it.
                        "SwapPss:" => swap_pss_kb = Some(val),
                        _ => {}
                    }
                }
            }
        }

        let uss_kb = priv_clean + priv_dirty;
        Some((pss_kb, rss_kb, uss_kb, swap_pss_kb.unwrap_or(swap_kb)))
    }

    /// Fallback when `smaps_rollup` is unreadable (typically EACCES for
    /// processes owned by another user). Returns (VmRSS, VmSwap) — there is no
    /// PSS/USS available here, so callers must treat it as an approximation.
    fn read_status_mem(pid: u32) -> (u64, u64) {
        let path = format!("/proc/{}/status", pid);
        let mut rss = 0u64;
        let mut swap = 0u64;

        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            for line in reader.lines().map_while(Result::ok) {
                if line.starts_with("VmRSS:") {
                    if let Some(val) = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        rss = val;
                    }
                } else if line.starts_with("VmSwap:") {
                    if let Some(val) = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        swap = val;
                    }
                }
            }
        }
        (rss, swap)
    }

    fn detect_role(name: &str, cmdline: &str, exe: &str) -> String {
        let cmd_lower = cmdline.to_lowercase();
        let exe_lower = exe.to_lowercase();

        if cmd_lower.contains("--type=gpu-process") {
            "GPU Process (Render Engine / OpenGL)".to_string()
        } else if cmd_lower.contains("--type=renderer") {
            if cmd_lower.contains("--extension-process") {
                "Renderer (Browser Extension)".to_string()
            } else {
                "Renderer (Web Tab Content)".to_string()
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
                "Utility Worker".to_string()
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
            "Steam WebHelper Worker".to_string()
        } else if cmd_lower.contains("gpu-screen-recorder") {
            "Replay / Screen Buffer Worker".to_string()
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

#[cfg(test)]
mod tests {
    use super::SystemScanner;

    /// A real `/proc/<pid>/stat` line (from `cat`), trimmed of nothing.
    const STAT: &str = "188537 (cat) R 188535 188535 188535 0 -1 4194304 129 0 0 0 7 11 0 0 16 -4 1 0 1961384 9265152 1517 18446744073709551615 94370030219264 94370030259713 140721592824064 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0 94370030275056 94370030276900 94371029528576 140721592832503 140721592832523 140721592832523 140721592836074 0\n";

    #[test]
    fn parses_stat_fields_including_starttime() {
        let (name, ppid, cpu_ticks, threads, starttime) = SystemScanner::parse_stat(STAT).unwrap();
        assert_eq!(name, "cat");
        assert_eq!(ppid, 188535);
        assert_eq!(cpu_ticks, 18); // utime 7 + stime 11
        assert_eq!(threads, 1);
        // starttime identifies the process across PID reuse.
        assert_eq!(starttime, 1961384);
    }

    #[test]
    fn parses_comm_containing_spaces_and_parens() {
        let line = STAT.replace("(cat)", "(weird (name) here)");
        let (name, ppid, _, _, starttime) = SystemScanner::parse_stat(&line).unwrap();
        assert_eq!(name, "weird (name) here");
        assert_eq!(ppid, 188535);
        assert_eq!(starttime, 1961384);
    }

    #[test]
    fn rejects_malformed_stat() {
        assert!(SystemScanner::parse_stat("nonsense without parens").is_none());
    }
}
