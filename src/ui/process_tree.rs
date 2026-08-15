use std::collections::HashSet;
use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};
use crate::process::types::{AppGroup, Category, ProcessInfo, SortColumn, SortDirection, SystemMemoryInfo};
use crate::ui::theme::Theme;

pub struct ProcessTreeView {
    pub expanded_groups: HashSet<String>,
    pub selected_pid: Option<u32>,
    pub status_message: Option<(String, std::time::Instant)>,
}

impl ProcessTreeView {
    pub fn new() -> Self {
        let mut expanded = HashSet::new();
        // Expand major multi-process apps by default so user sees sub-processes immediately
        expanded.insert("helium_browser".to_string());
        expanded.insert("discord".to_string());
        expanded.insert("antigravity_ide".to_string());
        expanded.insert("steam".to_string());

        Self {
            expanded_groups: expanded,
            selected_pid: None,
            status_message: None,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        groups: &[AppGroup],
        sys_mem: &SystemMemoryInfo,
        active_category: Option<Category>,
        filter_query: &str,
        sort_column: SortColumn,
        sort_direction: SortDirection,
    ) {
        // Status message notification if any
        if let Some((msg, time)) = &self.status_message {
            if time.elapsed().as_secs() < 4 {
                Frame::none()
                    .fill(Color32::from_rgb(20, 83, 45))
                    .stroke(Stroke::new(1.0_f32, Theme::ACCENT_EMERALD))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✓").color(Theme::ACCENT_EMERALD).strong());
                            ui.label(RichText::new(msg).color(Color32::WHITE));
                        });
                    });
                ui.add_space(6.0);
            }
        }

        // Table Header
        self.render_header(ui);
        ui.add_space(4.0);

        // Filter and sort groups
        let query = filter_query.trim().to_lowercase();
        let mut filtered: Vec<&AppGroup> = groups
            .iter()
            .filter(|g| {
                if let Some(cat) = active_category {
                    if g.category != cat {
                        return false;
                    }
                }
                if !query.is_empty() {
                    let matches_group = g.display_name.to_lowercase().contains(&query) || g.key.contains(&query);
                    let matches_proc = g.processes.iter().any(|p| {
                        p.name.to_lowercase().contains(&query)
                            || p.cmdline.to_lowercase().contains(&query)
                            || p.pid.to_string().contains(&query)
                            || p.role_hint.to_lowercase().contains(&query)
                    });
                    matches_group || matches_proc
                } else {
                    true
                }
            })
            .collect();

        // Sort groups
        filtered.sort_by(|a, b| {
            let ordering = match sort_column {
                SortColumn::PssRealisticRam => a.total_pss_kb.cmp(&b.total_pss_kb),
                SortColumn::RssStandardRam => a.total_rss_kb.cmp(&b.total_rss_kb),
                SortColumn::UssPrivateRam => a.total_uss_kb.cmp(&b.total_uss_kb),
                SortColumn::Cpu => a.total_cpu.partial_cmp(&b.total_cpu).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::ProcessCount => a.processes.len().cmp(&b.processes.len()),
                SortColumn::Name => a.display_name.cmp(&b.display_name),
            };
            if sort_direction == SortDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        });

        // Group rows container
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for group in filtered {
                    self.render_group_row(ui, group, sys_mem, &query);
                    ui.add_space(3.0);
                }
            });
    }

    fn render_header(&self, ui: &mut Ui) {
        Frame::none()
            .fill(Theme::BG_HEADER)
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(10.0, 8.0))
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_DARK))
            .show(ui, |ui| {
                ui.columns(5, |cols| {
                    cols[0].label(RichText::new("Aplikacja / Proces").color(Theme::TEXT_SECONDARY).strong());
                    cols[1].label(RichText::new("Podprocesy").color(Theme::TEXT_SECONDARY).strong());
                    cols[2].label(RichText::new("Realny RAM (PSS)").color(Theme::ACCENT_CYAN).strong());
                    cols[3].label(RichText::new("Tradycyjny RSS").color(Theme::TEXT_MUTED).strong());
                    cols[4].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("CPU / Akcja").color(Theme::TEXT_SECONDARY).strong());
                    });
                });
            });
    }

    fn render_group_row(&mut self, ui: &mut Ui, group: &AppGroup, sys_mem: &SystemMemoryInfo, query: &str) {
        let is_expanded = self.expanded_groups.contains(&group.key) || !query.is_empty();
        let total_kb = sys_mem.total_kb.max(1);
        let pss_pct = (group.total_pss_kb as f32 / total_kb as f32) * 100.0;
        let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);

        let mut toggle_clicked = false;
        let mut kill_group_requested = false;

        Frame::none()
            .fill(Theme::BG_CARD)
            .stroke(Stroke::new(1.0_f32, if is_expanded { col } else { Theme::BORDER_DARK }))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Expand/collapse button
                    let arrow = if is_expanded { "▼" } else { "▶" };
                    if ui.button(RichText::new(arrow).color(col).size(13.0)).clicked() {
                        toggle_clicked = true;
                    }

                    // Icon & Name
                    ui.label(RichText::new(group.icon).size(16.0));
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&group.display_name).color(Theme::TEXT_PRIMARY).strong().size(14.0));
                            // Category Badge
                            ui.label(
                                RichText::new(format!(" {} ", group.category.title()))
                                    .size(10.0)
                                    .color(Theme::TEXT_MUTED)
                                    .background_color(Color32::from_rgb(30, 36, 52)),
                            );
                        });
                    });

                    // Layout columns
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Kill Group Button
                        if ui.button(RichText::new("✖ Zakończ").color(Theme::ACCENT_ROSE).size(11.0)).on_hover_text("Zakończ wszystkie procesy tej aplikacji (SIGTERM)").clicked() {
                            kill_group_requested = true;
                        }

                        // CPU %
                        ui.add_space(8.0);
                        let cpu_col = if group.total_cpu > 10.0 { Theme::ACCENT_AMBER } else { Theme::TEXT_SECONDARY };
                        ui.label(RichText::new(format!("{:>4.1}% CPU", group.total_cpu)).color(cpu_col).monospace());

                        // Traditional RSS
                        ui.add_space(16.0);
                        let rss_str = Theme::format_kb(group.total_rss_kb);
                        ui.label(RichText::new(rss_str).color(Theme::TEXT_MUTED).monospace());

                        // Realistic PSS + Percentage Badge
                        ui.add_space(16.0);
                        let pss_str = Theme::format_kb(group.total_pss_kb);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(pss_str).color(Theme::ACCENT_CYAN).strong().monospace());
                            ui.label(
                                RichText::new(format!(" ({:.1}%) ", pss_pct))
                                    .color(Color32::BLACK)
                                    .size(10.0)
                                    .strong()
                                    .background_color(Theme::ACCENT_CYAN),
                            );
                        });

                        // Process count badge
                        ui.add_space(16.0);
                        let count_str = format!("{} procesów", group.processes.len());
                        ui.label(RichText::new(count_str).color(Theme::TEXT_SECONDARY).size(12.0));
                    });
                });

                // Expanded Sub-processes Table
                if is_expanded && !group.processes.is_empty() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    for proc in &group.processes {
                        self.render_subprocess_row(ui, proc);
                    }
                }
            });

        if toggle_clicked {
            if self.expanded_groups.contains(&group.key) {
                self.expanded_groups.remove(&group.key);
            } else {
                self.expanded_groups.insert(group.key.clone());
            }
        }

        if kill_group_requested {
            self.terminate_group(group);
        }
    }

    fn render_subprocess_row(&mut self, ui: &mut Ui, proc: &ProcessInfo) {
        let mut kill_pid = None;

        Frame::none()
            .fill(Theme::BG_SUB_ROW)
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::symmetric(8.0, 4.0))
            .stroke(Stroke::new(0.5_f32, Theme::BORDER_DARK))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Indent + PID
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(format!("PID {}", proc.pid))
                            .monospace()
                            .size(11.0)
                            .color(Theme::TEXT_MUTED)
                            .background_color(Color32::from_rgb(28, 33, 46)),
                    );

                    // Role Badge
                    ui.label(
                        RichText::new(&proc.role_hint)
                            .color(Theme::ACCENT_PURPLE)
                            .size(12.0)
                            .strong(),
                    );

                    // Process name
                    ui.label(RichText::new(&proc.name).color(Theme::TEXT_SECONDARY).size(11.0));

                    // Columns right to left
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Kill single button
                        if ui.small_button(RichText::new("✖").color(Theme::ACCENT_ROSE)).on_hover_text(format!("Zakończ PID {}", proc.pid)).clicked() {
                            kill_pid = Some(proc.pid);
                        }

                        // CPU
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("{:.1}%", proc.cpu_usage)).color(Theme::TEXT_MUTED).size(11.0).monospace());

                        // RSS
                        ui.add_space(16.0);
                        ui.label(RichText::new(Theme::format_kb(proc.rss_kb)).color(Theme::TEXT_MUTED).size(11.0).monospace());

                        // PSS
                        ui.add_space(16.0);
                        ui.label(RichText::new(Theme::format_kb(proc.pss_kb)).color(Theme::ACCENT_CYAN).size(11.0).strong().monospace());

                        // Threads
                        ui.add_space(12.0);
                        ui.label(RichText::new(format!("{} thr", proc.threads)).color(Theme::TEXT_MUTED).size(10.0));
                    });
                });

                // Small command line details on hover
                if ui.is_rect_visible(ui.min_rect()) {
                    let cmd_preview = if proc.cmdline.len() > 140 {
                        format!("{}...", &proc.cmdline[..140])
                    } else {
                        proc.cmdline.clone()
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new(format!("$ {}", cmd_preview))
                                .size(10.0)
                                .color(Color32::from_rgb(80, 95, 120))
                                .monospace(),
                        );
                    });
                }
            });

        if let Some(pid) = kill_pid {
            self.terminate_process(pid);
        }
    }

    fn terminate_process(&mut self, pid: u32) {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
        self.status_message = Some((
            format!("Wysłano sygnał SIGTERM do procesu PID {}", pid),
            std::time::Instant::now(),
        ));
    }

    fn terminate_group(&mut self, group: &AppGroup) {
        let count = group.processes.len();
        for p in &group.processes {
            unsafe {
                libc::kill(p.pid as i32, libc::SIGTERM);
            }
        }
        self.status_message = Some((
            format!("Zakończono aplikację {} ({} procesów)", group.display_name, count),
            std::time::Instant::now(),
        ));
    }
}
