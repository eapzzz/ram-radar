use super::{
    app::{Page, StillApp},
    theme::*,
    widgets::*,
};
use crate::process::types::Category;
use egui::RichText;
use egui_extras::{Column, TableBuilder};
impl StillApp {
    pub fn overview(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Your system at a glance.",
            "Live resource usage, with the applications behind it.",
        );
        let m = self.snapshot.memory.clone();
        let h = self.snapshot.hardware.clone();
        let memory = m.used_kb() as f32 / m.total_kb.max(1) as f32 * 100.0;
        let cards = if ui.available_width() < 740.0 { 2 } else { 4 };
        ui.columns(cards, |cols| {
            metric(
                &mut cols[0],
                "Memory",
                &kb(m.used_kb()),
                &format!("{} available of {}", kb(m.available_kb), kb(m.total_kb)),
                memory,
                BLUE,
            );
            metric(
                &mut cols[1],
                "Processor",
                &pct(h.cpu),
                &format!("{} logical cores", h.cores.len()),
                h.cpu,
                GREEN,
            );
            metric(
                &mut cols[2 % cards],
                "Graphics",
                &h.gpus
                    .first()
                    .and_then(|g| g.busy)
                    .map(pct)
                    .unwrap_or_else(|| "Unavailable".into()),
                &h.gpus
                    .first()
                    .and_then(|g| g.used)
                    .map(|v| format!("{} video memory", bytes(v as f64)))
                    .unwrap_or_else(|| "Driver does not expose usage".into()),
                h.gpus.first().and_then(|g| g.busy).unwrap_or(0.0),
                PURPLE,
            );
            metric(
                &mut cols[3 % cards],
                "Network",
                &rate(h.down()),
                &format!("{} upload", rate(h.up())),
                0.0,
                PEACH,
            );
        });
        ui.add_space(20.0);
        if ui.available_width() >= 950.0 {
            ui.columns(2, |cols| {
                self.top_consumers(&mut cols[0]);
                self.activity_panel(&mut cols[1]);
            });
        } else {
            self.top_consumers(ui);
            ui.add_space(16.0);
            self.activity_panel(ui);
        }
    }
    fn top_consumers(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Largest consumers").size(18.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("View all").clicked() {
                    self.page = Page::Applications
                }
            });
        });
        self.table(ui, Some(7));
        ui.add_space(8.0);
        ui.label(
            muted(format!(
                "Proportional memory (PSS). {} processes use an RSS estimate (~).",
                self.snapshot.memory.estimated_proc_count
            ))
            .size(11.0),
        );
    }
    fn activity_panel(&mut self, ui: &mut egui::Ui) {
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Activity");
                ui.label(muted("Up to 10 minutes").size(11.0));
            });
            ui.horizontal(|ui| {
                ui.colored_label(BLUE, "Memory");
                ui.colored_label(GREEN, "CPU");
                ui.label(muted("0–100%").size(11.0));
            });
            let mem: Vec<_> = self.history.points.iter().map(|p| p.memory).collect();
            let cpu: Vec<_> = self.history.points.iter().map(|p| p.cpu).collect();
            chart(
                ui,
                &[(&mem, BLUE), (&cpu, GREEN)],
                &self
                    .history
                    .points
                    .iter()
                    .map(|p| (p.time, p.gap))
                    .collect::<Vec<_>>(),
                80.0,
                100.0,
            );
            if ui.small_button("Open history").clicked() {
                self.page = Page::History;
            }
        });
        ui.add_space(14.0);
        panel(ui, |ui| {
            ui.strong("What matters right now");
            let m = &self.snapshot.memory;
            let h = &self.snapshot.hardware;
            let memory = m.used_kb() as f32 / m.total_kb.max(1) as f32 * 100.0;
            let pressure = h.pressure_memory.unwrap_or(0.0);
            let (title, detail, color) = if pressure > 1.0 {
                ("Memory is causing waits",format!("Tasks waited for memory {:.1}% of the last 10 seconds. Review the largest consumers.",pressure),PEACH)
            } else if h.pressure_cpu.unwrap_or(0.0) > 10.0 {
                (
                    "Work is waiting for the CPU",
                    format!(
                        "CPU pressure is {:.1}%. Sort applications by CPU to find busy workers.",
                        h.pressure_cpu.unwrap_or(0.0)
                    ),
                    PEACH,
                )
            } else if memory > 85.0 {
                (
                    "Memory headroom is getting low",
                    format!(
                        "{} remains available. High use alone does not mean the system is stalled.",
                        kb(m.available_kb)
                    ),
                    PEACH,
                )
            } else {
                (
                    "Resources have headroom",
                    format!(
                        "{} of memory is available. {}",
                        kb(m.available_kb),
                        if h.pressure_memory.is_some() {
                            "No significant memory stalls."
                        } else {
                            "Pressure readings are unavailable."
                        }
                    ),
                    GREEN,
                )
            };
            ui.label(RichText::new(title).color(color).size(17.0));
            ui.add(egui::Label::new(muted(detail)).wrap());
            let growth = self
                .snapshot
                .groups
                .iter()
                .filter_map(|g| {
                    let before = *self.baselines.get(&g.key)?;
                    let delta = g.total_pss_kb.saturating_sub(before);
                    (delta > 64 * 1024).then_some((g, delta))
                })
                .max_by_key(|(_, d)| *d);
            if let Some((g, delta)) = growth {
                ui.label(
                    muted(format!(
                        "{} grew by {} since first seen. Growth alone does not prove a leak.",
                        g.display_name,
                        kb(delta)
                    ))
                    .size(12.0),
                );
            } else {
                ui.label(
                    muted(format!(
                        "Swap: {} of {}",
                        kb(m.swap_used_kb),
                        kb(m.swap_total_kb)
                    ))
                    .size(12.0),
                );
            }
        });
    }
    pub fn applications(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Applications",
            "Find a program, compare its footprint, then open it to inspect individual processes.",
        );
        ui.horizontal_wrapped(|ui| {
            let input = egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search name, executable or PID…")
                .desired_width(300.0);
            let response = ui.add(input);
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
                response.request_focus();
            }
            ui.checkbox(&mut self.only_apps, "Desktop apps only");
        });
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(muted("Sort by"));
            for (i, name) in ["Memory", "CPU", "Processes", "Name", "Growth"]
                .iter()
                .enumerate()
            {
                if ui.selectable_label(self.sort == i, *name).clicked() {
                    if self.sort == i {
                        self.ascending = !self.ascending
                    } else {
                        self.sort = i;
                        self.ascending = i == 3;
                    }
                }
            }
            ui.label(
                muted(if self.ascending {
                    "Ascending"
                } else {
                    "Descending"
                })
                .size(12.0),
            );
        });
        ui.add_space(14.0);
        self.table(ui, None);
        ui.add_space(10.0);
        ui.label(muted("Memory: proportional share (PSS), ~ estimated from RSS. CPU: 100% = one core. Growth: since this app was first seen in this session.").size(12.0));
    }
    fn table(&mut self, ui: &mut egui::Ui, limit: Option<usize>) {
        let query = self.search.to_lowercase();
        let mut groups: Vec<_> = self
            .snapshot
            .groups
            .iter()
            .filter(|g| {
                limit.is_some()
                    || ((!self.only_apps || g.category == Category::Apps)
                        && (query.is_empty()
                            || g.display_name.to_lowercase().contains(&query)
                            || g.key.to_lowercase().contains(&query)
                            || g.processes
                                .iter()
                                .any(|p| p.pid.to_string().contains(&query))))
            })
            .collect();
        let growth = |g: &crate::process::types::AppGroup| -> i64 {
            g.total_pss_kb as i64
                - self
                    .baselines
                    .get(&g.key)
                    .copied()
                    .unwrap_or(g.total_pss_kb) as i64
        };
        groups.sort_by(|a, b| {
            let order = match if limit.is_some() { 0 } else { self.sort } {
                1 => b.total_cpu.total_cmp(&a.total_cpu),
                2 => b.processes.len().cmp(&a.processes.len()),
                3 => b
                    .display_name
                    .to_lowercase()
                    .cmp(&a.display_name.to_lowercase()),
                4 => growth(b).cmp(&growth(a)),
                _ => b.total_pss_kb.cmp(&a.total_pss_kb),
            };
            if limit.is_none() && self.ascending {
                order.reverse()
            } else {
                order
            }
            .then_with(|| a.key.cmp(&b.key))
        });
        if let Some(n) = limit {
            groups.truncate(n)
        }
        if groups.is_empty() {
            ui.add_space(20.0);
            ui.label("No applications match this filter.");
            if ui.button("Clear filters").clicked() {
                self.search.clear();
                self.only_apps = false;
            }
            return;
        }
        let full = ui.available_width() > 740.0;
        let mut selected = None;
        let table = TableBuilder::new(ui)
            .id_salt(if limit.is_some() {
                "top-table"
            } else {
                "apps-table"
            })
            .striped(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::remainder().at_least(160.0))
            .column(Column::exact(130.0))
            .column(Column::exact(80.0))
            .column(Column::exact(65.0));
        let table = if full {
            table.column(Column::exact(95.0))
        } else {
            table
        };
        table
            .header(28.0, |mut row| {
                for name in ["Application", "Memory", "CPU", "PIDs"] {
                    row.col(|ui| {
                        ui.label(muted(name).size(12.0));
                    });
                }
                if full {
                    row.col(|ui| {
                        ui.label(muted("Growth").size(12.0));
                    });
                }
            })
            .body(|body| {
                body.rows(44.0, groups.len(), |mut row| {
                    let g = groups[row.index()];
                    row.col(|ui| {
                        icon(
                            ui,
                            &g.display_name,
                            self.feed.desktop.icon_for(&g.key),
                            32.0,
                        );
                        ui.vertical(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(&g.display_name).strong())
                                        .frame(false),
                                )
                                .on_hover_text(&g.key)
                                .clicked()
                            {
                                selected = Some(g.key.clone());
                            }
                            ui.label(muted(g.category.short_title()).size(11.0));
                        });
                    });
                    row.col(|ui| {
                        ui.vertical(|ui| {
                            ui.label(format!(
                                "{}{}",
                                if g.estimated_procs > 0 { "~" } else { "" },
                                kb(g.total_pss_kb)
                            ));
                            meter(
                                ui,
                                g.total_pss_kb as f32 / self.snapshot.memory.total_kb.max(1) as f32
                                    * 100.0,
                                BLUE,
                            );
                        });
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1}%", g.total_cpu));
                    });
                    row.col(|ui| {
                        ui.label(muted(g.processes.len().to_string()));
                    });
                    if full {
                        row.col(|ui| {
                            let delta = growth(g);
                            ui.label(
                                RichText::new(if delta.unsigned_abs() < 1024 {
                                    "—".into()
                                } else {
                                    format!(
                                        "{}{}",
                                        if delta > 0 { "+" } else { "−" },
                                        kb(delta.unsigned_abs())
                                    )
                                })
                                .color(if delta > 64 * 1024 { PEACH } else { MUTED })
                                .size(12.0),
                            );
                        });
                    }
                });
            });
        if selected.is_some() {
            self.selected = selected
        }
    }
    pub fn hardware(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Hardware",
            "Live readings from the hardware and drivers available on this machine.",
        );
        let h = &self.snapshot.hardware;
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(&h.cpu_name);
                ui.label(RichText::new(pct(h.cpu)).color(GREEN));
            });
            ui.label(
                muted(format!(
                    "{} logical cores  •  Load average (1 / 5 / 15 min): {}",
                    h.cores.len(),
                    h.load
                ))
                .size(12.0),
            );
            ui.add_space(8.0);
            let columns = ((ui.available_width() / 110.0) as usize).clamp(2, 12);
            egui::Grid::new("cores")
                .num_columns(columns)
                .spacing(egui::vec2(16.0, 12.0))
                .show(ui, |ui| {
                    for (i, usage) in h.cores.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.set_width(85.0);
                            ui.label(muted(format!("Core {}   {:.0}%", i, usage)).size(11.0));
                            meter(ui, *usage, GREEN);
                        });
                        if (i + 1) % columns == 0 {
                            ui.end_row()
                        }
                    }
                });
        });
        ui.add_space(16.0);
        ui.columns(2,|cols|{
            panel(&mut cols[0],|ui|{ui.strong("Graphics & video memory");if h.gpus.is_empty(){ui.label(muted("No DRM graphics devices detected."));}for g in &h.gpus{ui.label(&g.name);keyvalue(ui,"Driver",&g.driver);keyvalue(ui,"GPU usage",g.busy.map(pct).unwrap_or_else(||"Unavailable".into()));keyvalue(ui,"Video memory",match(g.used,g.total){(Some(u),Some(t))=>format!("{} / {}",bytes(u as f64),bytes(t as f64)),_=>"Not exposed by driver".into()});if let(Some(u),Some(t))=(g.used,g.total){meter(ui,u as f32/t.max(1)as f32*100.0,PURPLE)}ui.add_space(6.0);}ui.label(muted("GPU counters depend on the kernel driver.").size(11.0));});
            panel(&mut cols[1],|ui|{ui.strong("Memory breakdown");let m=&self.snapshot.memory;keyvalue(ui,"In use",kb(m.used_kb()));keyvalue(ui,"Available",kb(m.available_kb));keyvalue(ui,"Free",kb(m.free_kb));keyvalue(ui,"Buffers + cache",kb(m.buffers_cache_kb));keyvalue(ui,"Swap",format!("{} / {}",kb(m.swap_used_kb),kb(m.swap_total_kb)));ui.label(muted("Available includes reclaimable memory. These rows overlap; do not add them together.").size(11.0));});
        });
        ui.add_space(16.0);
        panel(ui, |ui| {
            ui.strong("Storage");
            for m in &h.mounts {
                ui.add_space(8.0);
                keyvalue(
                    ui,
                    &format!("{}  ({})", m.path, m.filesystem),
                    format!(
                        "{} available / {}",
                        bytes(m.available as f64),
                        bytes(m.total as f64)
                    ),
                );
                meter(
                    ui,
                    (m.total.saturating_sub(m.available)) as f32 / m.total.max(1) as f32 * 100.0,
                    PEACH,
                );
            }
            if h.mounts.is_empty() {
                ui.label(muted("No supported local filesystems found."));
            }
            ui.add_space(10.0);
            for d in &h.disks {
                keyvalue(
                    ui,
                    &d.name,
                    format!(
                        "Read {}    Write {}    Busy {:.0}%",
                        rate(d.read),
                        rate(d.write),
                        d.busy
                    ),
                );
            }
        });
        ui.add_space(16.0);
        ui.columns(2, |cols| {
            panel(&mut cols[0], |ui| {
                ui.strong("Network interfaces");
                for n in &h.networks {
                    ui.add_space(8.0);
                    ui.label(&n.name);
                    keyvalue(
                        ui,
                        "Download / upload",
                        format!("{} / {}", rate(n.down), rate(n.up)),
                    );
                    ui.label(
                        muted(format!(
                            "Received {}  •  Sent {} since interface reset",
                            bytes(n.received as f64),
                            bytes(n.sent as f64)
                        ))
                        .size(11.0),
                    );
                }
                if h.networks.is_empty() {
                    ui.label(muted("No network interfaces found."));
                }
                ui.label(muted("Virtual interfaces can count the same traffic twice.").size(11.0));
            });
            panel(&mut cols[1], |ui| {
                ui.strong("Sensors");
                for s in &h.sensors {
                    keyvalue(ui, &s.name, format!("{:.1} {}", s.value, s.unit));
                }
                if h.sensors.is_empty() {
                    ui.label(muted("No readable hwmon sensors."));
                }
                if let Some((capacity, status)) = &h.battery {
                    keyvalue(ui, "Battery", format!("{capacity:.0}% / {status}"));
                }
            });
        });
        ui.add_space(16.0);
        panel(ui, |ui| {
            ui.strong("Resource pressure");
            ui.label(
                muted("Percentage of time tasks were stalled, averaged over the last 10 seconds.")
                    .size(12.0),
            );
            for (name, value) in [
                ("CPU", h.pressure_cpu),
                ("Memory", h.pressure_memory),
                ("I/O", h.pressure_io),
            ] {
                keyvalue(
                    ui,
                    name,
                    value.map(pct).unwrap_or_else(|| "Unavailable".into()),
                );
            }
        });
    }
    pub fn history_page(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Session history",
            "Up to 10 minutes of local history. Data stays in memory until you export it.",
        );
        ui.horizontal(|ui| {
            if ui.button("Export CSV").clicked() {
                self.export()
            }
            ui.label(muted(format!(
                "{} samples recorded",
                self.history.points.len()
            )));
        });
        ui.add_space(16.0);
        let cpu: Vec<_> = self.history.points.iter().map(|p| p.cpu).collect();
        let mem: Vec<_> = self.history.points.iter().map(|p| p.memory).collect();
        let down: Vec<_> = self.history.points.iter().map(|p| p.down as f32).collect();
        let up: Vec<_> = self.history.points.iter().map(|p| p.up as f32).collect();
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(GREEN, "CPU");
                ui.colored_label(BLUE, "Memory");
                ui.label(muted("0–100%").size(12.0));
            });
            chart(
                ui,
                &[(&cpu, GREEN), (&mem, BLUE)],
                &self
                    .history
                    .points
                    .iter()
                    .map(|p| (p.time, p.gap))
                    .collect::<Vec<_>>(),
                190.0,
                100.0,
            );
            ui.horizontal(|ui| {
                ui.label(muted(format!(
                    "CPU peak {:.1}%",
                    cpu.iter().copied().fold(0.0, f32::max)
                )));
                ui.label(muted(format!(
                    "Memory peak {:.1}%",
                    mem.iter().copied().fold(0.0, f32::max)
                )));
            });
        });
        ui.add_space(16.0);
        panel(ui, |ui| {
            let max = down.iter().chain(&up).copied().fold(1024.0, f32::max);
            ui.horizontal(|ui| {
                ui.colored_label(PEACH, "Download");
                ui.colored_label(PURPLE, "Upload");
                ui.label(muted(format!("Scale: {}", rate(max as f64))).size(12.0));
            });
            chart(
                ui,
                &[(&down, PEACH), (&up, PURPLE)],
                &self
                    .history
                    .points
                    .iter()
                    .map(|p| (p.time, p.gap))
                    .collect::<Vec<_>>(),
                150.0,
                max,
            );
        });
        ui.add_space(12.0);
        ui.label(muted("Hover over a chart to inspect values. Pausing stops collection; history is retained for this session.").size(12.0));
    }
    pub fn settings(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Preferences",
            "Small controls for a monitor that stays out of the way.",
        );
        panel(ui, |ui| {
            ui.strong("Sampling");
            ui.label(muted(
                "Slower sampling reduces the cost of scanning proportional process memory.",
            ));
            ui.horizontal(|ui| {
                for seconds in [1, 2, 5, 10] {
                    if ui
                        .selectable_label(
                            self.preferences.interval == seconds,
                            format!("{seconds} seconds"),
                        )
                        .clicked()
                    {
                        self.preferences.interval = seconds;
                        self.feed
                            .interval
                            .store(seconds, std::sync::atomic::Ordering::Relaxed);
                        self.save_preferences();
                    }
                }
            });
            ui.add_space(14.0);
            ui.strong("Interface size");
            if ui
                .add(
                    egui::Slider::new(&mut self.preferences.scale, 0.8..=1.5)
                        .step_by(0.1)
                        .suffix("×"),
                )
                .changed()
            {
                ui.ctx().set_zoom_factor(self.preferences.scale);
                self.save_preferences();
            }
        });
        ui.add_space(16.0);
        panel(ui, |ui| {
            ui.strong("Honest measurements");
            ui.label("PSS divides shared pages between the processes using them. When access is denied, Still uses an explicitly marked RSS estimate. System memory uses MemTotal − MemAvailable, not the sum of applications.");
            ui.add_space(8.0);
            ui.label("Installed applications include helpers from their installation directory. Other processes are grouped by executable; each process remains available in the details. GPU support depends on what your driver exposes.");
            ui.add_space(8.0);
            ui.label("Still uses no cloud services, telemetry, elevated privileges or always-on daemon. Closing the window ends monitoring.");
            ui.add_space(8.0);
            ui.hyperlink_to(
                "Source code & issue tracker",
                "https://github.com/eapzzz/still",
            );
        });
    }
}
