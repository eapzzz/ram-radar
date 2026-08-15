use std::collections::HashSet;
use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};
use crate::process::types::{AppGroup, ProcessInfo, SystemMemoryInfo};
use crate::ui::theme::Theme;

pub struct AppCardView;

impl AppCardView {
    pub fn render_group(
        ui: &mut Ui,
        group: &AppGroup,
        sys_mem: &SystemMemoryInfo,
        expanded_groups: &mut HashSet<String>,
        status_msg: &mut Option<(String, std::time::Instant)>,
    ) {
        let is_expanded = expanded_groups.contains(&group.key);
        let total_kb = sys_mem.total_kb.max(1);
        let pss_pct = (group.total_pss_kb as f32 / total_kb as f32) * 100.0;
        let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);

        let mut toggle_clicked = false;
        let mut kill_group = false;

        // Custom Card Container
        Frame::none()
            .fill(Theme::BG_CARD)
            .stroke(Stroke::new(1.0_f32, if is_expanded { col } else { Theme::BORDER_GLASS }))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                // 1. Top Main Row
                ui.horizontal(|ui| {
                    // Expand/Collapse Chevron Button
                    let arrow = if is_expanded { "▼" } else { "▶" };
                    if ui.button(RichText::new(arrow).color(col).size(13.0).strong()).clicked() {
                        toggle_clicked = true;
                    }

                    // Large App Icon
                    ui.label(RichText::new(group.icon).size(18.0));

                    // App Title & Category Badge
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&group.display_name)
                                    .color(Color32::WHITE)
                                    .size(15.0)
                                    .strong(),
                            );

                            // Category Tag Pill
                            ui.label(
                                RichText::new(format!(" {} ", group.category.title()))
                                    .size(10.0)
                                    .color(Theme::TEXT_MUTED)
                                    .background_color(Theme::BG_PILL),
                            );

                            // Process Count Pill
                            ui.label(
                                RichText::new(format!(" {} procesów ", group.processes.len()))
                                    .size(10.5)
                                    .color(Theme::TEXT_SECONDARY)
                                    .background_color(Color32::from_rgb(25, 33, 50)),
                            );
                        });
                    });

                    // Right Side Metrics (Direct fixed layout, rock solid)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Kill Group Button
                        if ui.button(
                            RichText::new("✖ Zakończ")
                                .color(Theme::ACCENT_ROSE)
                                .size(11.0)
                                .strong(),
                        ).on_hover_text("Zakończ wszystkie procesy tej aplikacji (SIGTERM)").clicked() {
                            kill_group = true;
                        }

                        // CPU Meter
                        ui.add_space(6.0);
                        let cpu_color = if group.total_cpu > 10.0 { Theme::ACCENT_AMBER } else { Theme::TEXT_SECONDARY };
                        ui.label(
                            RichText::new(format!("{:>4.1}% CPU", group.total_cpu))
                                .color(cpu_color)
                                .monospace()
                                .size(12.0),
                        );

                        // Traditional RSS Pill
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!("RSS: {}", Theme::format_kb(group.total_rss_kb)))
                                .color(Theme::TEXT_MUTED)
                                .size(11.5)
                                .monospace(),
                        );

                        // Real PSS Metric with Glow Badge
                        ui.add_space(10.0);
                        let pss_str = Theme::format_kb(group.total_pss_kb);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(pss_str)
                                    .color(Theme::ACCENT_CYAN)
                                    .size(14.0)
                                    .strong()
                                    .monospace(),
                            );
                            ui.label(
                                RichText::new(format!(" {:.1}% ", pss_pct))
                                    .color(Color32::BLACK)
                                    .size(10.5)
                                    .strong()
                                    .background_color(Theme::ACCENT_CYAN),
                            );
                        });
                    });
                });

                // 2. Expanded Subprocess List
                if is_expanded && !group.processes.is_empty() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Subprocess Table Header
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("PID & ROLA PROCESU").size(10.5).color(Theme::TEXT_MUTED).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("AKCJA").size(10.5).color(Theme::TEXT_MUTED).strong());
                            ui.add_space(12.0);
                            ui.label(RichText::new("CPU").size(10.5).color(Theme::TEXT_MUTED).strong());
                            ui.add_space(16.0);
                            ui.label(RichText::new("RSS").size(10.5).color(Theme::TEXT_MUTED).strong());
                            ui.add_space(16.0);
                            ui.label(RichText::new("REALNY RAM (PSS)").size(10.5).color(Theme::ACCENT_CYAN).strong());
                            ui.add_space(16.0);
                            ui.label(RichText::new("WĄTKI").size(10.5).color(Theme::TEXT_MUTED).strong());
                        });
                    });

                    ui.add_space(4.0);

                    for proc in &group.processes {
                        Self::render_subprocess_row(ui, proc, status_msg);
                    }
                }
            });

        if toggle_clicked {
            if is_expanded {
                expanded_groups.remove(&group.key);
            } else {
                expanded_groups.insert(group.key.clone());
            }
        }

        if kill_group {
            let count = group.processes.len();
            for p in &group.processes {
                unsafe {
                    libc::kill(p.pid as i32, libc::SIGTERM);
                }
            }
            *status_msg = Some((
                format!("Zakończono aplikację {} ({} procesów)", group.display_name, count),
                std::time::Instant::now(),
            ));
        }
    }

    fn render_subprocess_row(
        ui: &mut Ui,
        proc: &ProcessInfo,
        status_msg: &mut Option<(String, std::time::Instant)>,
    ) {
        let mut kill_pid = None;
        let mut copy_pid = false;

        Frame::none()
            .fill(Theme::BG_SUBROW)
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(10.0, 6.0))
            .stroke(Stroke::new(0.5_f32, Theme::BORDER_GLASS))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(12.0);

                    // PID Button (Click to Copy)
                    if ui.button(
                        RichText::new(format!("PID {}", proc.pid))
                            .size(10.5)
                            .monospace()
                            .color(Theme::TEXT_SECONDARY),
                    ).on_hover_text("Kliknij, aby skopiować PID").clicked() {
                        copy_pid = true;
                    }

                    // Role Badge
                    ui.label(
                        RichText::new(&proc.role_hint)
                            .color(Theme::ACCENT_VIOLET)
                            .size(12.0)
                            .strong(),
                    );

                    // Process Executable Name
                    ui.label(
                        RichText::new(&proc.name)
                            .color(Theme::TEXT_MUTED)
                            .size(11.0),
                    );

                    // Right Side Subprocess Metrics
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Kill Button
                        if ui.small_button(RichText::new("✖").color(Theme::ACCENT_ROSE)).on_hover_text(format!("Zakończ PID {}", proc.pid)).clicked() {
                            kill_pid = Some(proc.pid);
                        }

                        // CPU
                        ui.add_space(10.0);
                        let cpu_color = if proc.cpu_usage > 5.0 { Theme::ACCENT_AMBER } else { Theme::TEXT_MUTED };
                        ui.label(RichText::new(format!("{:>4.1}%", proc.cpu_usage)).color(cpu_color).size(11.0).monospace());

                        // RSS
                        ui.add_space(14.0);
                        ui.label(RichText::new(Theme::format_kb(proc.rss_kb)).color(Theme::TEXT_MUTED).size(11.0).monospace());

                        // PSS (Real RAM)
                        ui.add_space(14.0);
                        ui.label(RichText::new(Theme::format_kb(proc.pss_kb)).color(Theme::ACCENT_CYAN).size(11.5).strong().monospace());

                        // Threads
                        ui.add_space(14.0);
                        ui.label(RichText::new(format!("{} thr", proc.threads)).color(Theme::TEXT_MUTED).size(10.5));
                    });
                });

                // Command line preview snippet
                if !proc.cmdline.is_empty() {
                    let preview = if proc.cmdline.len() > 120 {
                        format!("{}...", &proc.cmdline[..120])
                    } else {
                        proc.cmdline.clone()
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label(
                            RichText::new(format!("$ {}", preview))
                                .size(9.5)
                                .color(Color32::from_rgb(90, 105, 135))
                                .monospace(),
                        );
                    });
                }
            });

        ui.add_space(2.0);

        if let Some(pid) = kill_pid {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
            *status_msg = Some((
                format!("Wysłano sygnał SIGTERM do PID {}", pid),
                std::time::Instant::now(),
            ));
        }

        if copy_pid {
            ui.output_mut(|o| o.copied_text = proc.pid.to_string());
            *status_msg = Some((
                format!("Skopiowano PID {} do schowka", proc.pid),
                std::time::Instant::now(),
            ));
        }
    }
}
