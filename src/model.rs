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
pub const MAX_HISTORY_SECONDS: u64 = 3600;
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
    pub fn window(&self, seconds: u64) -> impl Iterator<Item = &Point> {
        let start = self
            .points
            .back()
            .map_or(0, |p| p.time.saturating_sub(seconds));
        self.points.iter().filter(move |p| p.time >= start)
    }
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
            .is_some_and(|p| s.timestamp.saturating_sub(p.time) > MAX_HISTORY_SECONDS)
            || self.points.len() > MAX_HISTORY_SECONDS as usize + 1
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
    fn selected_window_filters_by_time_and_keeps_full_history_for_export() {
        let mut h = History::default();
        for time in [0, 2700, 3540, 3585, 3595, 3600] {
            h.push(&Snapshot {
                timestamp: time,
                ..Default::default()
            });
        }
        assert_eq!(
            h.window(15).map(|p| p.time).collect::<Vec<_>>(),
            [3585, 3595, 3600]
        );
        assert_eq!(
            h.window(60).map(|p| p.time).collect::<Vec<_>>(),
            [3540, 3585, 3595, 3600]
        );
        assert_eq!(
            h.window(300).map(|p| p.time).collect::<Vec<_>>(),
            [3540, 3585, 3595, 3600]
        );
        assert_eq!(
            h.window(900).map(|p| p.time).collect::<Vec<_>>(),
            [2700, 3540, 3585, 3595, 3600]
        );
        assert_eq!(h.window(3600).count(), 6);
        assert_eq!(h.csv().lines().count(), 7);
    }

    #[test]
    fn selected_window_is_anchored_to_latest_sample_when_paused() {
        let mut h = History::default();
        for time in [100, 105, 120] {
            h.push(&Snapshot {
                timestamp: time,
                ..Default::default()
            });
        }
        assert_eq!(h.window(15).map(|p| p.time).collect::<Vec<_>>(), [105, 120]);
        assert_eq!(History::default().window(60).count(), 0);
    }
    #[test]
    fn history_is_bounded_and_export_marks_missing_gpu() {
        let mut h = History::default();
        for i in 0..5000 {
            h.push(&Snapshot {
                timestamp: i,
                ..Default::default()
            })
        }
        assert_eq!(h.points.len(), 3601);
        assert_eq!(h.points.front().unwrap().time, 1399);
        assert!(h.csv().contains(",0,,0,0"));
    }
}
