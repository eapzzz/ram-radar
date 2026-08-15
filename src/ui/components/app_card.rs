use std::collections::HashSet;
use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2, Sense};
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

        let mut toggle = false;
        let mut kill = false;

        Frame::none()
            .fill(Theme::BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, if is_expanded { col } else { Theme::BORDER_DEFAULT }))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                // Draw left accent bar
                let rect = ui.max_rect();
                let accent_bar = egui::Rect::from_min_size(
                    rect.left_top(),
                    Vec2::new(3.0, rect.height()),
                );
                ui.painter().rect_filled(accent_bar, Rounding::same(2.0), if is_expanded { col } else { Theme::BORDER_DEFAULT });

                ui.horizontal(|ui| {
                    // Expand chevron
                    let chevron = if is_expanded { "▾" } else { "▸" };
                    if ui.button(RichText::new(chevron).size(14.0).color(col)).clicked() {
                        toggle = true;
                    }

                    // Icon
                    ui.label(RichText::new(group.icon).size(16.0));

                    // Name + badges
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&group.display_name).size(13.5).color(Color32::WHITE).strong());
                            ui.label(RichText::new(group.category.short_title()).size(9.5).color(Theme::TEXT_MUTED).background_color(Theme::BG_BADGE));
                            ui.label(RichText::new(format!("{} procs", group.processes.len())).size(9.5).color(Theme::TEXT_SECONDARY).background_color(Theme::BG_BADGE));
                        });
                    });

                    // Right metrics
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("✕ Kill").size(10.0).color(Theme::ACCENT_RED)).on_hover_text("SIGTERM all").clicked() {
                            kill = true;
                        }

                        ui.add_space(6.0);
                        let cpu_col = if group.total_cpu > 10.0 { Theme::ACCENT_ORANGE } else { Theme::TEXT_SECONDARY };
                        ui.label(RichText::new(format!("{:.1}%", group.total_cpu)).size(11.0).color(cpu_col).monospace());

                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("RSS {}", Theme::format_kb(group.total_rss_kb))).size(10.5).color(Theme::TEXT_MUTED).monospace());

                        ui.add_space(6.0);

                        // PSS value + percent badge
                        ui.label(RichText::new(format!("{:.1}%", pss_pct)).size(9.5).color(Color32::BLACK).strong().background_color(col));
                        ui.label(RichText::new(Theme::format_kb(group.total_pss_kb)).size(12.5).color(col).strong().monospace());

                        // Mini progress bar
                        let bar_w = 40.0_f32;
                        let bar_h = 3.0_f32;
                        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, bar_h), Sense::hover());
                        Theme::draw_bar(ui.painter(), bar_rect, pss_pct / 100.0, col);
                    });
                });

                // Expanded processes
                if is_expanded && !group.processes.is_empty() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Table header
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(RichText::new("PID / ROLE").size(9.5).color(Theme::TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("ACTION").size(9.5).color(Theme::TEXT_MUTED));
                            ui.add_space(8.0);
                            ui.label(RichText::new("CPU").size(9.5).color(Theme::TEXT_MUTED));
                            ui.add_space(12.0);
                            ui.label(RichText::new("RSS").size(9.5).color(Theme::TEXT_MUTED));
                            ui.add_space(12.0);
                            ui.label(RichText::new("PSS").size(9.5).color(Theme::ACCENT_BLUE));
                            ui.add_space(12.0);
                            ui.label(RichText::new("THR").size(9.5).color(Theme::TEXT_MUTED));
                        });
                    });
                    ui.add_space(2.0);

                    for proc in &group.processes {
                        Self::render_proc(ui, proc, status_msg);
                    }
                }
            });

        if toggle {
            if is_expanded { expanded_groups.remove(&group.key); } else { expanded_groups.insert(group.key.clone()); }
        }

        if kill {
            let count = group.processes.len();
            for p in &group.processes {
                unsafe { libc::kill(p.pid as i32, libc::SIGTERM); }
            }
            *status_msg = Some((format!("Terminated {} ({} procs)", group.display_name, count), std::time::Instant::now()));
        }
    }

    fn render_proc(ui: &mut Ui, proc: &ProcessInfo, status_msg: &mut Option<(String, std::time::Instant)>) {
        let mut kill_pid = None;
        let mut copy_pid = false;

        Frame::none()
            .fill(Theme::BG_SUBROW)
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::symmetric(8.0, 4.0))
            .stroke(Stroke::new(0.5_f32, Theme::BORDER_MUTED))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);

                    if ui.button(RichText::new(format!("{}", proc.pid)).size(10.0).monospace().color(Theme::TEXT_SECONDARY))
                        .on_hover_text("Copy PID").clicked() {
                        copy_pid = true;
                    }

                    ui.label(RichText::new(&proc.role_hint).size(10.5).color(Theme::ACCENT_PURPLE).strong());
                    ui.label(RichText::new(&proc.name).size(10.0).color(Theme::TEXT_MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(RichText::new("✕").color(Theme::ACCENT_RED)).on_hover_text(format!("Kill {}", proc.pid)).clicked() {
                            kill_pid = Some(proc.pid);
                        }
                        ui.add_space(6.0);
                        let cpu_col = if proc.cpu_usage > 5.0 { Theme::ACCENT_ORANGE } else { Theme::TEXT_MUTED };
                        ui.label(RichText::new(format!("{:.1}%", proc.cpu_usage)).size(10.0).color(cpu_col).monospace());
                        ui.add_space(10.0);
                        ui.label(RichText::new(Theme::format_kb(proc.rss_kb)).size(10.0).color(Theme::TEXT_MUTED).monospace());
                        ui.add_space(10.0);
                        ui.label(RichText::new(Theme::format_kb(proc.pss_kb)).size(10.0).color(Theme::ACCENT_BLUE).strong().monospace());
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("{}", proc.threads)).size(10.0).color(Theme::TEXT_MUTED));
                    });
                });

                // Cmdline preview
                if !proc.cmdline.is_empty() {
                    let preview = if proc.cmdline.len() > 100 { format!("{}…", &proc.cmdline[..100]) } else { proc.cmdline.clone() };
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(RichText::new(format!("$ {}", preview)).size(9.0).color(Color32::from_rgb(75, 85, 100)).monospace());
                    });
                }
            });

        ui.add_space(1.0);

        if let Some(pid) = kill_pid {
            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
            *status_msg = Some((format!("SIGTERM → PID {}", pid), std::time::Instant::now()));
        }
        if copy_pid {
            ui.output_mut(|o| o.copied_text = proc.pid.to_string());
            *status_msg = Some((format!("Copied PID {}", proc.pid), std::time::Instant::now()));
        }
    }
}
