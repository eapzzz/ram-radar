use crate::desktop::DesktopRegistry;
use crate::process::types::{AppGroup, Category, ProcessInfo};
use std::collections::HashMap;
pub struct ProcessClassifier;
impl ProcessClassifier {
    pub fn group_processes(
        processes: Vec<ProcessInfo>,
        desktop: &DesktopRegistry,
    ) -> Vec<AppGroup> {
        let mut groups: HashMap<String, AppGroup> = HashMap::new();
        for p in processes {
            // Full executable identity prevents different programs with equal comm names merging.
            let key = if p.exe.is_empty() {
                format!("comm:{}", p.name)
            } else {
                p.exe.trim_end_matches(" (deleted)").to_owned()
            };
            let metadata = desktop.lookup(&p.exe);
            let key = desktop
                .group_id(&p.exe)
                .map(|id| format!("app:{id}"))
                .unwrap_or(key);
            let display = metadata.map(|d| d.name.clone()).unwrap_or_else(|| {
                std::path::Path::new(&key)
                    .file_name()
                    .map(|n| n.to_string_lossy().trim_start_matches("comm:").to_owned())
                    .unwrap_or_else(|| p.name.clone())
            });
            let category = if metadata.is_some() {
                Category::Apps
            } else if p.ppid == 2 || p.pid == 2 {
                Category::System
            } else {
                Category::Background
            };
            let g = groups
                .entry(key.clone())
                .or_insert_with(|| AppGroup::new(key, display, category));
            g.processes.push(p);
        }
        let mut groups: Vec<_> = groups.into_values().collect();
        for g in &mut groups {
            g.processes.sort_by_key(|p| std::cmp::Reverse(p.pss_kb));
            g.recompute_totals()
        }
        groups.sort_by(|a, b| {
            b.total_pss_kb
                .cmp(&a.total_pss_kb)
                .then_with(|| a.key.cmp(&b.key))
        });
        groups
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn process(exe: &str, pid: u32) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid: 1,
            name: "same".into(),
            cmdline: "irrelevant discord helium text".into(),
            exe: exe.into(),
            pss_kb: 10,
            rss_kb: 20,
            uss_kb: 5,
            swap_kb: 0,
            threads: 1,
            role_hint: String::new(),
            utime_stime: 0,
            cpu_usage: 0.0,
            pss_estimated: false,
            starttime: 1,
        }
    }
    #[test]
    fn executable_identity_and_exact_totals() {
        let groups = ProcessClassifier::group_processes(
            vec![
                process("/one/app", 1),
                process("/two/app", 2),
                process("/one/app", 3),
            ],
            &DesktopRegistry::default(),
        );
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].total_pss_kb, 20);
        assert_eq!(groups[0].processes.len(), 2);
    }
}
