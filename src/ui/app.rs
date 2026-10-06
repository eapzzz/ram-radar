use super::{theme::*, widgets::*};
use crate::{
    model::{Feed, History, Snapshot, MAX_HISTORY_SECONDS},
    process::types::ProcessInfo,
};
use egui::{Color32, RichText};
use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering};
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Page {
    #[default]
    Overview,
    Applications,
    Hardware,
    History,
    Settings,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub interval: u64,
    pub scale: f32,
    pub chart_range: ChartRange,
    pub custom_history_seconds: u64,
}
#[derive(Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum ChartRange {
    FifteenSeconds,
    #[default]
    OneMinute,
    FiveMinutes,
    Custom,
}
impl Preferences {
    pub fn chart_seconds(&self) -> u64 {
        match self.chart_range {
            ChartRange::FifteenSeconds => 15,
            ChartRange::OneMinute => 60,
            ChartRange::FiveMinutes => 300,
            ChartRange::Custom => self.custom_history_seconds.clamp(1, MAX_HISTORY_SECONDS),
        }
    }
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            interval: 2,
            scale: 1.0,
            chart_range: ChartRange::default(),
            custom_history_seconds: 300,
        }
    }
}
fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    PathBuf::from(base).join("still/settings.json")
}
pub struct StillApp {
    pub feed: Feed,
    pub snapshot: Snapshot,
    pub history: History,
    pub page: Page,
    pub search: String,
    pub sort: usize,
    pub ascending: bool,
    pub selected: Option<String>,
    pub confirm: Option<ProcessInfo>,
    pub notice: String,
    pub preferences: Preferences,
    pub paused: bool,
    pub only_apps: bool,
    pub baselines: HashMap<String, u64>,
    pub screenshot: Option<PathBuf>,
    pub samples: usize,
}
impl StillApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        super::theme::setup(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut preferences: Preferences = std::fs::read(config_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        preferences.interval = preferences.interval.clamp(1, 10);
        if !preferences.scale.is_finite() {
            preferences.scale = 1.0
        }
        preferences.scale = preferences.scale.clamp(0.8, 1.5);
        preferences.custom_history_seconds = preferences
            .custom_history_seconds
            .clamp(1, MAX_HISTORY_SECONDS);
        cc.egui_ctx.set_zoom_factor(preferences.scale);
        let args: Vec<_> = std::env::args().collect();
        let page = args
            .windows(2)
            .find(|w| w[0] == "--view")
            .map(|w| match w[1].as_str() {
                "applications" => Page::Applications,
                "hardware" => Page::Hardware,
                "history" => Page::History,
                "settings" => Page::Settings,
                _ => Page::Overview,
            })
            .unwrap_or_default();
        let screenshot = args
            .windows(2)
            .find(|w| w[0] == "--screenshot")
            .map(|w| PathBuf::from(&w[1]));
        Self {
            feed: Feed::start(cc.egui_ctx.clone(), preferences.interval),
            snapshot: Snapshot::default(),
            history: History::default(),
            page,
            search: String::new(),
            sort: 0,
            ascending: false,
            selected: None,
            confirm: None,
            notice: String::new(),
            preferences,
            paused: false,
            only_apps: false,
            baselines: HashMap::new(),
            screenshot,
            samples: 0,
        }
    }
    pub fn save_preferences(&mut self) {
        let p = config_path();
        let result = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(p.parent().unwrap())?;
            let tmp = p.with_extension("tmp");
            std::fs::write(&tmp, serde_json::to_vec_pretty(&self.preferences)?)?;
            std::fs::rename(tmp, p)
        })();
        self.notice = match result {
            Ok(()) => "Preferences saved".into(),
            Err(e) => format!("Could not save preferences: {e}"),
        };
    }
    pub fn export(&mut self) {
        let base = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            format!("{}/.local/share", std::env::var("HOME").unwrap_or_default())
        });
        let dir = PathBuf::from(base).join("still/exports");
        let p = dir.join(format!("still-{}.csv", self.snapshot.timestamp));
        self.notice = match std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&p, self.history.csv()))
        {
            Ok(()) => format!("Saved {}", p.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }
    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(194.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(17, 23, 33))
                    .inner_margin(18.0),
            )
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(28.0, 32.0), egui::Sense::hover());
                    for (i, h) in [12.0, 26.0, 19.0].iter().enumerate() {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(r.left() + i as f32 * 9.0, r.bottom() - h),
                                egui::vec2(5.0, *h),
                            ),
                            2.5,
                            BLUE,
                        );
                    }
                    ui.label(RichText::new("still").size(30.0).strong());
                });
                ui.add_space(4.0);
                ui.label(muted("A little more clarity.").size(11.0));
                let compact = ctx.screen_rect().height() < 650.0;
                ui.add_space(if compact { 8.0 } else { 38.0 });
                for (i, (page, label)) in [
                    (Page::Overview, "Overview"),
                    (Page::Applications, "Applications"),
                    (Page::Hardware, "Hardware"),
                    (Page::History, "History"),
                ]
                .iter()
                .enumerate()
                {
                    self.nav(ui, *page, label, i);
                    ui.add_space(3.0);
                }
                if compact {
                    self.nav(ui, Page::Settings, "Preferences", 4);
                    return;
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(muted("Still 0.2  /  Linux").size(11.0));
                    ui.add_space(8.0);
                    self.nav(ui, Page::Settings, "Preferences", 4);
                    ui.add_space(20.0);
                    ui.label(
                        muted(format!("Up {}", duration(self.snapshot.hardware.uptime))).size(12.0),
                    );
                    ui.label(
                        RichText::new(if self.paused {
                            "Sampling paused"
                        } else {
                            "Local monitoring"
                        })
                        .color(if self.paused { PEACH } else { GREEN })
                        .size(12.0),
                    );
                });
            });
    }
    fn nav(&mut self, ui: &mut egui::Ui, page: Page, label: &str, index: usize) {
        let active = self.page == page;
        let response = egui::Frame::none()
            .fill(if active {
                Color32::from_rgb(35, 49, 69)
            } else {
                Color32::TRANSPARENT
            })
            .rounding(8.0)
            .inner_margin(if ui.ctx().screen_rect().height() < 650.0 {
                6.0
            } else {
                10.0
            })
            .show(ui, |ui| {
                ui.set_width(138.0);
                ui.horizontal(|ui| {
                    nav_icon(ui, index, if active { BLUE } else { MUTED });
                    ui.label(RichText::new(label).color(if active { TEXT } else { MUTED }));
                });
            })
            .response;
        let r = ui.interact(response.rect, ui.id().with(label), egui::Sense::click());
        if r.clicked() {
            self.page = page;
        }
        r.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    fn details(&mut self, ctx: &egui::Context) {
        if let Some(key) = self.selected.clone() {
            let group = self.snapshot.groups.iter().find(|g| g.key == key).cloned();
            let mut open = true;
            egui::Window::new("Application details").id(egui::Id::new("details")).open(&mut open).default_width(740.0).resizable(true).show(ctx,|ui|{if let Some(g)=group{
            ui.horizontal(|ui|{icon(ui,&g.display_name,self.feed.desktop.icon_for(&g.key),40.0);ui.vertical(|ui|{ui.heading(&g.display_name);ui.label(muted(format!("{} processes  /  {} memory",g.processes.len(),kb(g.total_pss_kb))));});});ui.add_space(10.0);ui.label(muted(&g.key));ui.separator();ui.label(muted("Memory is proportional (PSS). ~ means an RSS estimate. CPU: 100% = one logical core.").size(12.0));
            egui::ScrollArea::vertical().max_height(430.0).show(ui,|ui|{for p in &g.processes{egui::CollapsingHeader::new(format!("{}   PID {}    {}{}    {:.1}% CPU",p.name,p.pid,if p.pss_estimated{"~"}else{""},kb(p.pss_kb),p.cpu_usage)).id_salt(p.pid).show(ui,|ui|{keyvalue(ui,"Parent PID",p.ppid.to_string());keyvalue(ui,"Private memory",if p.pss_estimated{"Unavailable".into()}else{kb(p.uss_kb)});keyvalue(ui,"Resident memory",kb(p.rss_kb));keyvalue(ui,"Swap",kb(p.swap_kb));keyvalue(ui,"Threads",p.threads.to_string());ui.label(muted(&p.role_hint));ui.add(egui::Label::new(RichText::new(&p.cmdline).monospace().size(11.0)).wrap());if ui.button("End this process…").clicked(){self.confirm=Some(p.clone());}});}});
        }else{ui.label("This application has exited.");}});
            if !open {
                self.selected = None
            }
        }
        if let Some(p) = self.confirm.clone() {
            egui::Window::new("End process?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Send a termination request to {} (PID {})?",
                        p.name, p.pid
                    ));
                    ui.label(muted("Unsaved work in this process may be lost."));
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm = None
                        }
                        if ui
                            .button(RichText::new("End process").color(PEACH))
                            .clicked()
                        {
                            self.notice = match crate::actions::terminate(p.pid, p.starttime) {
                                Ok(()) => format!("Termination requested for PID {}", p.pid),
                                Err(e) => format!("Could not end process: {e}"),
                            };
                            self.confirm = None;
                        }
                    });
                });
        }
    }
}
impl eframe::App for StillApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let next = self.feed.latest.lock().ok().and_then(|mut l| l.take());
        if let Some(s) = next {
            for g in &s.groups {
                self.baselines
                    .entry(g.key.clone())
                    .or_insert(g.total_pss_kb);
            }
            self.baselines
                .retain(|k, _| s.groups.iter().any(|g| &g.key == k));
            self.history.push(&s);
            self.snapshot = s;
            self.samples += 1;
        }
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(path) = self.screenshot.take() {
                        let bytes: Vec<u8> =
                            image.pixels.iter().flat_map(|p| p.to_array()).collect();
                        self.notice = match image::save_buffer(
                            &path,
                            &bytes,
                            image.width() as u32,
                            image.height() as u32,
                            image::ColorType::Rgba8,
                        ) {
                            Ok(()) => format!("Saved {}", path.display()),
                            Err(e) => e.to_string(),
                        };
                    }
                }
            }
        });
        if self.screenshot.is_some() && self.samples >= 3 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.selected = None;
            self.confirm = None;
        }
        self.sidebar(ctx);
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::none()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(22.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        muted(if self.notice.is_empty() {
                            format!(
                                "{} processes   •   sample {:.0} ms   •   every {} s",
                                self.snapshot
                                    .groups
                                    .iter()
                                    .map(|g| g.processes.len())
                                    .sum::<usize>(),
                                self.snapshot.scan_ms,
                                self.preferences.interval
                            )
                        } else {
                            self.notice.clone()
                        })
                        .size(11.0),
                    );
                    if !self.notice.is_empty() && ui.small_button("Dismiss").clicked() {
                        self.notice.clear();
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(28.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(muted("Your computer, understood.").size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(if self.paused { "Resume" } else { "Pause" })
                            .clicked()
                        {
                            self.paused = !self.paused;
                            self.history.interrupted = true;
                            self.feed.paused.store(self.paused, Ordering::Relaxed);
                        }
                        ui.label(
                            RichText::new(if self.paused { "Paused" } else { "Live" })
                                .size(12.0)
                                .color(if self.paused { PEACH } else { GREEN }),
                        );
                    });
                });
                ui.add_space(18.0);
                if self.samples == 0 {
                    heading(
                        ui,
                        "Getting a clear picture",
                        "Reading system counters and application memory…",
                    );
                    ui.spinner();
                    return;
                }
                egui::ScrollArea::both()
                    .id_salt("page-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(550.0);
                        match self.page {
                            Page::Overview => self.overview(ui),
                            Page::Applications => self.applications(ui),
                            Page::Hardware => self.hardware(ui),
                            Page::History => self.history_page(ui),
                            Page::Settings => self.settings(ui),
                        }
                    });
            });
        self.details(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_retain_custom_chart_range_when_saved_again() {
        let preferences: Preferences =
            serde_json::from_str(r#"{"chart_range":"Custom","custom_history_seconds":3600}"#)
                .unwrap();
        let saved = serde_json::to_value(preferences).unwrap();
        assert_eq!(saved["chart_range"], "Custom");
        assert_eq!(saved["custom_history_seconds"], 3600);
    }

    #[test]
    fn legacy_preferences_get_a_working_chart_range() {
        let preferences: Preferences =
            serde_json::from_str(r#"{"interval":5,"scale":1.2}"#).unwrap();
        assert_eq!(preferences.interval, 5);
        assert_eq!(preferences.chart_seconds(), 60);
    }

    #[test]
    fn custom_window_is_clamped_to_one_second_and_one_hour() {
        let mut preferences = Preferences {
            chart_range: ChartRange::Custom,
            custom_history_seconds: 0,
            ..Default::default()
        };
        assert_eq!(preferences.chart_seconds(), 1);
        preferences.custom_history_seconds = u64::MAX;
        assert_eq!(preferences.chart_seconds(), 3600);
    }
}
