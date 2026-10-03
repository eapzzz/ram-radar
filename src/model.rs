use crate::{
    desktop::DesktopRegistry,
    process::{
        classifier::ProcessClassifier,
        scanner::SystemScanner,
        types::{AppGroup, SystemMemoryInfo},
    },
    telemetry::{Collector, Hardware},
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Default)]
pub struct Snapshot {
    pub memory: SystemMemoryInfo,
    pub groups: Vec<AppGroup>,
    pub hardware: Hardware,
    pub scan_ms: f64,
    pub timestamp: u64,
}
pub struct Feed {
    pub latest: Arc<Mutex<Option<Snapshot>>>,
    pub interval: Arc<AtomicU64>,
    pub paused: Arc<AtomicBool>,
    pub desktop: Arc<DesktopRegistry>,
}
impl Feed {
    pub fn start(ctx: egui::Context, seconds: u64) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let interval = Arc::new(AtomicU64::new(seconds));
        let paused = Arc::new(AtomicBool::new(false));
        let desktop = Arc::new(DesktopRegistry::load());
        let (out, timing, pause, registry) = (
            latest.clone(),
            interval.clone(),
            paused.clone(),
            desktop.clone(),
        );
        std::thread::Builder::new()
            .name("still-sampler".into())
            .spawn(move || {
                let scanner = SystemScanner::new();
                let mut collector = Collector::new();
                loop {
                    if !pause.load(Ordering::Relaxed) {
                        let start = Instant::now();
                        let hardware = collector.sample();
                        let (memory, processes) = scanner.scan_all_processes();
                        let groups = ProcessClassifier::group_processes(processes, &registry);
                        let snapshot = Snapshot {
                            memory,
                            groups,
                            hardware,
                            scan_ms: start.elapsed().as_secs_f64() * 1000.0,
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        };
                        if let Ok(mut slot) = out.lock() {
                            *slot = Some(snapshot)
                        }
                        ctx.request_repaint();
                    }
                    std::thread::sleep(Duration::from_secs(
                        timing.load(Ordering::Relaxed).clamp(1, 10),
                    ));
                }
            })
            .expect("start sampler");
        Self {
            latest,
            interval,
            paused,
            desktop,
        }
    }
}
#[derive(Clone)]
pub struct Point {
    pub gap: bool,
    pub time: u64,
    pub cpu: f32,
    pub memory: f32,
    pub gpu: Option<f32>,
    pub down: f64,
    pub up: f64,
    pub used_kb: u64,
}
#[derive(Default)]
pub struct History {
    pub interrupted: bool,
    pub points: VecDeque<Point>,
}
impl History {
    pub fn push(&mut self, s: &Snapshot) {
        self.points.push_back(Point {
            gap: std::mem::take(&mut self.interrupted),
            time: s.timestamp,
            cpu: s.hardware.cpu,
            memory: s.memory.used_kb() as f32 / s.memory.total_kb.max(1) as f32 * 100.0,
            gpu: s.hardware.gpus.first().and_then(|g| g.busy),
            down: s.hardware.down(),
            up: s.hardware.up(),
            used_kb: s.memory.used_kb(),
        });
        while self
            .points
            .front()
            .is_some_and(|p| s.timestamp.saturating_sub(p.time) > 600)
            || self.points.len() > 601
        {
            self.points.pop_front();
        }
    }
    pub fn csv(&self) -> String {
        let mut out=String::from("unix_seconds,cpu_percent,memory_percent,memory_kib,gpu_percent,download_bytes_per_second,upload_bytes_per_second\n");
        for p in &self.points {
            out.push_str(&format!(
                "{},{:.2},{:.2},{},{},{:.0},{:.0}\n",
                p.time,
                p.cpu,
                p.memory,
                p.used_kb,
                p.gpu.map(|v| format!("{v:.2}")).unwrap_or_default(),
                p.down,
                p.up
            ))
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_is_bounded_and_export_marks_missing_gpu() {
        let mut h = History::default();
        for i in 0..1000 {
            h.push(&Snapshot {
                timestamp: i,
                ..Default::default()
            })
        }
        assert_eq!(h.points.len(), 601);
        assert!(h.csv().contains(",0,,0,0"));
    }
}
