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

    // Current State
    sys_mem: SystemMemoryInfo,
    app_groups: Vec<AppGroup>,

    // UI & Animations
    anim_state: AnimationState,
    expanded_groups: HashSet<String>,
    active_category: Option<Category>,
    filter_query: String,
    sort_column: SortColumn,
    sort_direction: SortDirection,
    refresh_interval_secs: f32,
    last_refresh: Instant,
    is_paused: bool,

    // Toasts
    status_message: Option<(String, Instant)>,
}

impl RamRadarApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        Theme::apply_to_ctx(&cc.egui_ctx);

        let scanner = Arc::new(SystemScanner::new());
        let (scan_tx, scan_rx) = channel();

        let mut expanded = HashSet::new();
        // Expand prominent multi-process apps by default
        expanded.insert("antigravity_ide".to_string());
        expanded.insert("discord".to_string());
        expanded.insert("helium_browser".to_string());
        expanded.insert("steam".to_string());

        let mut app = Self {
            scanner,
            scan_rx,
            scan_tx,
            is_scanning: Arc::new(Mutex::new(false)),
            sys_mem: SystemMemoryInfo::default(),
            app_groups: Vec::new(),
            anim_state: AnimationState::new(),
            expanded_groups: expanded,
            active_category: None,
            filter_query: String::new(),
            sort_column: SortColumn::PssRealisticRam,
            sort_direction: SortDirection::Descending,
            refresh_interval_secs: 1.0,
            last_refresh: Instant::now() - Duration::from_secs(10),
            is_paused: false,
            status_message: None,
        };

        app.trigger_background_scan();
        app
    }

    fn trigger_background_scan(&self) {
        let is_scanning = Arc::clone(&self.is_scanning);
        {
            let mut guard = is_scanning.lock().unwrap();
            if *guard {
                return;
            }
            *guard = true;
        }

        let scanner = Arc::clone(&self.scanner);
        let tx = self.scan_tx.clone();

        thread::spawn(move || {
            let (sys_mem, processes) = scanner.scan_all_processes();
            let app_groups = ProcessClassifier::group_processes(processes);

            let _ = tx.send(ScanResult {
                sys_mem,
                app_groups,
            });

            if let Ok(mut guard) = is_scanning.lock() {
                *guard = false;
            }
        });
    }
}

impl eframe::App for RamRadarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process background scan results
        while let Ok(result) = self.scan_rx.try_recv() {
            self.sys_mem = result.sys_mem;
            self.app_groups = result.app_groups;

            let pss_gb = self.sys_mem.total_pss_sum_kb as f32 / (1024.0 * 1024.0);
            self.anim_state.push_sample(pss_gb);
        }

        // 2. Update animations
        let pss_gb = self.sys_mem.total_pss_sum_kb as f32 / (1024.0 * 1024.0);
        let used_gb = self.sys_mem.used_kb() as f32 / (1024.0 * 1024.0);
        self.anim_state.update_animations(pss_gb, used_gb);

        // 3. Periodic refresh
        if !self.is_paused && self.last_refresh.elapsed().as_secs_f32() >= self.refresh_interval_secs {
            self.trigger_background_scan();
            self.last_refresh = Instant::now();
        }

        // Request continuous repaint for smooth animations
        ctx.request_repaint_after(Duration::from_millis(16)); // ~60 FPS

        // 4. Central Panel UI Layout
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header Bar
            let mut manual_refresh = false;
            HeaderBar::show(
                ui,
                &mut self.active_category,
                &mut self.filter_query,
                &mut self.sort_column,
                &mut self.sort_direction,
                &mut self.refresh_interval_secs,
                &mut self.is_paused,
                || manual_refresh = true,
            );

            if manual_refresh {
                self.trigger_background_scan();
                self.last_refresh = Instant::now();
            }

            ui.add_space(8.0);

            // Toast message if active
            if let Some((msg, time)) = &self.status_message {
                if time.elapsed().as_secs() < 3 {
                    Frame::none()
                        .fill(Color32::from_rgb(16, 50, 30))
                        .stroke(Stroke::new(1.0_f32, Theme::ACCENT_EMERALD))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(12.0, 6.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("✓").color(Theme::ACCENT_EMERALD).strong());
                                ui.label(RichText::new(msg).color(Color32::WHITE));
                            });
                        });
                    ui.add_space(6.0);
                }
            }

            // Hero Metric Cards
            let total_procs: usize = self.app_groups.iter().map(|g| g.processes.len()).sum();
            HeroMetrics::show(ui, &self.sys_mem, &self.anim_state, total_procs);
            ui.add_space(10.0);

            // Memory Breakdown Visualizer + Sparkline
            let top_refs: Vec<&AppGroup> = self.app_groups.iter().collect();
            MemoryVisualizer::show(ui, &self.sys_mem, &top_refs, &self.anim_state);
            ui.add_space(10.0);

            // Process App Cards List
            self.render_app_list(ui);
        });
    }
}

impl RamRadarApp {
    fn render_app_list(&mut self, ui: &mut Ui) {
        let query = self.filter_query.trim().to_lowercase();

        // 1. Filter
        let mut filtered: Vec<&AppGroup> = self.app_groups
            .iter()
            .filter(|g| {
                if let Some(cat) = self.active_category {
                    if g.category != cat {
                        return false;
                    }
                }
                if !query.is_empty() {
                    let matches_name = g.display_name.to_lowercase().contains(&query) || g.key.contains(&query);
                    let matches_child = g.processes.iter().any(|p| {
                        p.name.to_lowercase().contains(&query)
                            || p.cmdline.to_lowercase().contains(&query)
                            || p.pid.to_string().contains(&query)
                            || p.role_hint.to_lowercase().contains(&query)
                    });
                    matches_name || matches_child
                } else {
                    true
                }
            })
            .collect();

        // 2. Sort
        filtered.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::PssRealisticRam => a.total_pss_kb.cmp(&b.total_pss_kb),
                SortColumn::RssStandardRam => a.total_rss_kb.cmp(&b.total_rss_kb),
                SortColumn::UssPrivateRam => a.total_uss_kb.cmp(&b.total_uss_kb),
                SortColumn::Cpu => a.total_cpu.partial_cmp(&b.total_cpu).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::ProcessCount => a.processes.len().cmp(&b.processes.len()),
                SortColumn::Name => a.display_name.cmp(&b.display_name),
            };
            if self.sort_direction == SortDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        });

        // 3. Scrollable List of App Cards
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("🔍 Brak procesów pasujących do filtra").size(15.0).color(Theme::TEXT_MUTED));
                    });
                } else {
                    for group in filtered {
                        AppCardView::render_group(
                            ui,
                            group,
                            &self.sys_mem,
                            &mut self.expanded_groups,
                            &mut self.status_message,
                        );
                        ui.add_space(6.0);
                    }
                }
            });
    }
}
