use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};

use crate::process::classifier::ProcessClassifier;
use crate::process::scanner::SystemScanner;
use crate::process::types::{AppGroup, Category, SortColumn, SortDirection, SystemMemoryInfo};
use crate::ui::animation::AnimationState;
use crate::ui::components::{AppCardView, HeaderBar, HeroMetrics, MemoryVisualizer};
use crate::ui::theme::Theme;

struct ScanResult {
    sys_mem: SystemMemoryInfo,
    app_groups: Vec<AppGroup>,
}

pub struct RamRadarApp {
    scanner: Arc<SystemScanner>,
    scan_rx: Receiver<ScanResult>,
    scan_tx: Sender<ScanResult>,
    is_scanning: Arc<Mutex<bool>>,

    sys_mem: SystemMemoryInfo,
    app_groups: Vec<AppGroup>,

    anim: AnimationState,
    expanded: HashSet<String>,
    active_cat: Option<Category>,
    filter: String,
    sort_col: SortColumn,
    sort_dir: SortDirection,
    refresh_interval: f32,
    last_refresh: Instant,
    paused: bool,
    toast: Option<(String, Instant)>,
}

impl RamRadarApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        Theme::apply_to_ctx(&cc.egui_ctx);

        let scanner = Arc::new(SystemScanner::new());
        let (tx, rx) = channel();

        let mut expanded = HashSet::new();
        expanded.insert("antigravity_ide".to_string());
        expanded.insert("discord".to_string());
        expanded.insert("helium_browser".to_string());

        let app = Self {
            scanner,
            scan_rx: rx,
            scan_tx: tx,
            is_scanning: Arc::new(Mutex::new(false)),
            sys_mem: SystemMemoryInfo::default(),
            app_groups: Vec::new(),
            anim: AnimationState::new(),
            expanded,
            active_cat: None,
            filter: String::new(),
            sort_col: SortColumn::PssRealisticRam,
            sort_dir: SortDirection::Descending,
            refresh_interval: 1.0,
            last_refresh: Instant::now() - Duration::from_secs(10),
            paused: false,
            toast: None,
        };

        app.trigger_scan();
        app
    }

    fn trigger_scan(&self) {
        let flag = Arc::clone(&self.is_scanning);
        {
            let mut g = flag.lock().unwrap();
            if *g { return; }
            *g = true;
        }
        let scanner = Arc::clone(&self.scanner);
        let tx = self.scan_tx.clone();
        thread::spawn(move || {
            let (sys_mem, procs) = scanner.scan_all_processes();
            let app_groups = ProcessClassifier::group_processes(procs);
            let _ = tx.send(ScanResult { sys_mem, app_groups });
            if let Ok(mut g) = flag.lock() { *g = false; }
        });
    }
}

impl eframe::App for RamRadarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process scan results
        while let Ok(result) = self.scan_rx.try_recv() {
            self.sys_mem = result.sys_mem;
            self.app_groups = result.app_groups;
            let pss_gb = self.sys_mem.total_pss_sum_kb as f32 / (1024.0 * 1024.0);
            self.anim.push_sample(pss_gb);
        }

        // Smooth animations
        let pss_gb = self.sys_mem.total_pss_sum_kb as f32 / (1024.0 * 1024.0);
        let used_gb = self.sys_mem.used_kb() as f32 / (1024.0 * 1024.0);
        self.anim.update(pss_gb, used_gb);

        // Auto refresh
        if !self.paused && self.last_refresh.elapsed().as_secs_f32() >= self.refresh_interval {
            self.trigger_scan();
            self.last_refresh = Instant::now();
        }

        ctx.request_repaint_after(Duration::from_millis(100));

        egui::CentralPanel::default()
            .frame(Frame::none().fill(Theme::BG_BASE).inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                // Header
                let mut manual_refresh = false;
                HeaderBar::show(
                    ui,
                    &mut self.active_cat,
                    &mut self.filter,
                    &mut self.sort_col,
                    &mut self.sort_dir,
                    &mut self.refresh_interval,
                    &mut self.paused,
                    || manual_refresh = true,
                );
                if manual_refresh {
                    self.trigger_scan();
                    self.last_refresh = Instant::now();
                }

                ui.add_space(8.0);

                // Toast
                if let Some((msg, time)) = &self.toast {
                    if time.elapsed().as_secs() < 3 {
                        Frame::none()
                            .fill(Color32::from_rgb(17, 38, 28))
                            .stroke(Stroke::new(1.0_f32, Theme::ACCENT_GREEN))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::symmetric(10.0, 5.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("✓ {}", msg)).color(Theme::ACCENT_GREEN));
                            });
                        ui.add_space(4.0);
                    }
                }

                // Hero Metrics
                let total_procs: usize = self.app_groups.iter().map(|g| g.processes.len()).sum();
                HeroMetrics::show(ui, &self.sys_mem, &self.anim, total_procs);
                ui.add_space(8.0);

                // Memory Visualizer
                let refs: Vec<&AppGroup> = self.app_groups.iter().collect();
                MemoryVisualizer::show(ui, &self.sys_mem, &refs, &self.anim);
                ui.add_space(8.0);

                // Process List
                self.render_list(ui);
            });
    }
}

impl RamRadarApp {
    fn render_list(&mut self, ui: &mut Ui) {
        let query = self.filter.trim().to_lowercase();

        let mut filtered: Vec<&AppGroup> = self.app_groups.iter()
            .filter(|g| {
                if let Some(cat) = self.active_cat { if g.category != cat { return false; } }
                if query.is_empty() { return true; }
                g.display_name.to_lowercase().contains(&query)
                    || g.key.contains(&query)
                    || g.processes.iter().any(|p|
                        p.name.to_lowercase().contains(&query)
                        || p.pid.to_string().contains(&query)
                        || p.role_hint.to_lowercase().contains(&query))
            })
            .collect();

        filtered.sort_by(|a, b| {
            let ord = match self.sort_col {
                SortColumn::PssRealisticRam => a.total_pss_kb.cmp(&b.total_pss_kb),
                SortColumn::RssStandardRam => a.total_rss_kb.cmp(&b.total_rss_kb),
                SortColumn::UssPrivateRam => a.total_uss_kb.cmp(&b.total_uss_kb),
                SortColumn::Cpu => a.total_cpu.partial_cmp(&b.total_cpu).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::ProcessCount => a.processes.len().cmp(&b.processes.len()),
                SortColumn::Name => a.display_name.cmp(&b.display_name),
            };
            if self.sort_dir == SortDirection::Descending { ord.reverse() } else { ord }
        });

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if filtered.is_empty() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No processes match filter").size(13.0).color(Theme::TEXT_MUTED));
                });
            } else {
                for group in filtered {
                    AppCardView::render_group(ui, group, &self.sys_mem, &mut self.expanded, &mut self.toast);
                    ui.add_space(4.0);
                }
            }
        });
    }
}
