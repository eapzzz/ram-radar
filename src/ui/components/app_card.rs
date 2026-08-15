use std::collections::HashSet;
use egui::{Align, Color32, Frame, Layout, Margin, RichText, Rounding, Stroke, Ui, Vec2, Sense};
use crate::process::types::{AppGroup, ProcessInfo, SystemMemoryInfo};
use crate::ui::app::Toast;
use crate::ui::theme::Theme;

/// Inner margin of a group card; the accent bar has to undo it to sit flush.
const CARD_MARGIN: f32 = 10.0;
/// Horizontal padding of a process sub-row. The table header uses the same
/// value so its labels line up with the values underneath.
const ROW_PAD_X: f32 = 8.0;

// Fixed column widths for the expanded process table. Both the header and the
// data rows allocate exactly these, so the columns actually line up instead of
// drifting with the width of each value.
const COL_ACTION: f32 = 22.0;
const COL_CPU: f32 = 52.0;
const COL_RSS: f32 = 74.0;
const COL_PSS: f32 = 76.0;
const COL_THR: f32 = 32.0;

/// How long an armed "Kill" button waits for the confirming second click.
const KILL_CONFIRM_SECS: f64 = 4.0;

pub struct AppCardView;

impl AppCardView {
    pub fn render_group(
        ui: &mut Ui,
        group: &AppGroup,
        sys_mem: &SystemMemoryInfo,
        expanded_groups: &mut HashSet<String>,
        status_msg: &mut Option<Toast>,
    ) {
        let is_expanded = expanded_groups.contains(&group.key);
        let total_kb = sys_mem.total_kb.max(1);
        let pss_pct = (group.total_pss_kb as f32 / total_kb as f32) * 100.0;
        let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
        let estimated = group.estimated_procs > 0;

        let mut toggle = false;
        let mut kill = false;

        // Two-click confirmation state for this group's Kill button.
        let confirm_id = ui.make_persistent_id(("kill_confirm", &group.key));
        let now = ui.input(|i| i.time);
        let armed = ui
            .data(|d| d.get_temp::<f64>(confirm_id))
            .is_some_and(|t| now - t < KILL_CONFIRM_SECS);

        Frame::none()
            .fill(Theme::BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, if is_expanded { col } else { Theme::BORDER_DEFAULT }))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(CARD_MARGIN))
            .show(ui, |ui| {
                let bar_slot = Theme::begin_accent_bar(ui);

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
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (kill_label, kill_hint) = if armed {
                            ("✓ Confirm", format!("Click again to SIGTERM all {} process(es)", group.processes.len()))
                        } else {
                            ("✕ Kill", format!("SIGTERM all {} process(es) — asks to confirm", group.processes.len()))
                        };
                        let kill_col = if armed { Theme::ACCENT_ORANGE } else { Theme::ACCENT_RED };
                        if ui.button(RichText::new(kill_label).size(10.0).color(kill_col)).on_hover_text(kill_hint).clicked() {
                            if armed {
                                kill = true;
                            } else {
                                ui.data_mut(|d| d.insert_temp(confirm_id, now));
                            }
                        }

                        ui.add_space(6.0);
                        let cpu_col = if group.total_cpu > 10.0 { Theme::ACCENT_ORANGE } else { Theme::TEXT_SECONDARY };
                        ui.label(RichText::new(format!("{:.1}%", group.total_cpu)).size(11.0).color(cpu_col).monospace());

                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("RSS {}", Theme::format_kb(group.total_rss_kb))).size(10.5).color(Theme::TEXT_MUTED).monospace());

                        ui.add_space(6.0);

                        // PSS value + percent badge
                        ui.label(RichText::new(format!("{:.1}%", pss_pct)).size(9.5).color(Color32::BLACK).strong().background_color(col));
                        let pss_label = ui.label(
                            RichText::new(format!("{}{}", if estimated { "~" } else { "" }, Theme::format_kb(group.total_pss_kb)))
                                .size(12.5).color(col).strong().monospace(),
                        );
                        if estimated {
                            pss_label.on_hover_text(estimate_hint(group.estimated_procs));
                        }

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

                    Self::render_table_header(ui);
                    ui.add_space(2.0);

                    for proc in &group.processes {
                        Self::render_proc(ui, proc, status_msg);
                    }
                }

                Theme::end_accent_bar(ui, bar_slot, CARD_MARGIN, if is_expanded { col } else { Theme::BORDER_DEFAULT });
            });

        if toggle {
            if is_expanded { expanded_groups.remove(&group.key); } else { expanded_groups.insert(group.key.clone()); }
        }

        if kill {
            ui.data_mut(|d| d.remove::<f64>(confirm_id));
            let pids: Vec<u32> = group.processes.iter().map(|p| p.pid).collect();
            *status_msg = Some(signal_all(&pids, &group.display_name));
        }
    }

    fn render_table_header(ui: &mut Ui) {
        // Same horizontal padding as a process row so the columns line up.
        Frame::none()
            .inner_margin(Margin::symmetric(ROW_PAD_X, 0.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(ROW_PAD_X);
                    ui.label(RichText::new("PID / ROLE").size(9.5).color(Theme::TEXT_MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        cell(ui, COL_ACTION, RichText::new("").size(9.5));
                        cell(ui, COL_CPU, RichText::new("CPU").size(9.5).color(Theme::TEXT_MUTED));
                        cell(ui, COL_RSS, RichText::new("RSS").size(9.5).color(Theme::TEXT_MUTED));
                        cell(ui, COL_PSS, RichText::new("PSS").size(9.5).color(Theme::ACCENT_BLUE));
                        cell(ui, COL_THR, RichText::new("THR").size(9.5).color(Theme::TEXT_MUTED));
                    });
                });
            });
    }

    fn render_proc(ui: &mut Ui, proc: &ProcessInfo, status_msg: &mut Option<Toast>) {
        let mut kill_pid = None;
        let mut copy_pid = false;

        Frame::none()
            .fill(Theme::BG_SUBROW)
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::symmetric(ROW_PAD_X, 4.0))
            .stroke(Stroke::new(0.5_f32, Theme::BORDER_MUTED))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(ROW_PAD_X);

                    if ui.button(RichText::new(format!("{}", proc.pid)).size(10.0).monospace().color(Theme::TEXT_SECONDARY))
                        .on_hover_text("Copy PID").clicked() {
                        copy_pid = true;
                    }

                    ui.label(RichText::new(&proc.role_hint).size(10.5).color(Theme::ACCENT_PURPLE).strong());
                    ui.label(RichText::new(&proc.name).size(10.0).color(Theme::TEXT_MUTED));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(COL_ACTION, ui.spacing().interact_size.y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.small_button(RichText::new("✕").color(Theme::ACCENT_RED))
                                    .on_hover_text(format!("SIGTERM {}", proc.pid)).clicked() {
                                    kill_pid = Some(proc.pid);
                                }
                            },
                        );

                        let cpu_col = if proc.cpu_usage > 5.0 { Theme::ACCENT_ORANGE } else { Theme::TEXT_MUTED };
                        cell(ui, COL_CPU, RichText::new(format!("{:.1}%", proc.cpu_usage)).size(10.0).color(cpu_col).monospace());
                        cell(ui, COL_RSS, RichText::new(Theme::format_kb(proc.rss_kb)).size(10.0).color(Theme::TEXT_MUTED).monospace());

                        let pss_text = format!(
                            "{}{}",
                            if proc.pss_estimated { "~" } else { "" },
                            Theme::format_kb(proc.pss_kb)
                        );
                        let pss = cell(ui, COL_PSS, RichText::new(pss_text).size(10.0).color(Theme::ACCENT_BLUE).strong().monospace());
                        if proc.pss_estimated {
                            pss.on_hover_text(estimate_hint(1));
                        }

                        cell(ui, COL_THR, RichText::new(format!("{}", proc.threads)).size(10.0).color(Theme::TEXT_MUTED));
                    });
                });

                // Cmdline preview
                if !proc.cmdline.is_empty() {
                    let preview = truncate_chars(&proc.cmdline, 100);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(RichText::new(format!("$ {}", preview)).size(9.0).color(Color32::from_rgb(75, 85, 100)).monospace());
                    });
                }
            });

        ui.add_space(1.0);

        if let Some(pid) = kill_pid {
            *status_msg = Some(match send_sigterm(pid) {
                Ok(()) => Toast::success(format!("SIGTERM → PID {}", pid)),
                Err(e) => Toast::error(format!("PID {}: {}", pid, e)),
            });
        }
        if copy_pid {
            ui.output_mut(|o| o.copied_text = proc.pid.to_string());
            *status_msg = Some(Toast::success(format!("Copied PID {}", proc.pid)));
        }
    }
}

/// A fixed-width, right-aligned table cell. Header and data rows use identical
/// widths so the columns line up.
fn cell(ui: &mut Ui, width: f32, text: RichText) -> egui::Response {
    ui.allocate_ui_with_layout(
        Vec2::new(width, ui.spacing().interact_size.y),
        Layout::right_to_left(Align::Center),
        |ui| ui.label(text),
    )
    .inner
}

fn estimate_hint(count: usize) -> String {
    format!(
        "{} process(es) here are not owned by this user, so /proc/<pid>/smaps_rollup is unreadable \
         and RSS is shown in place of PSS (an over-estimate). Run as root for exact figures.",
        count
    )
}

/// Truncate on a character boundary. Slicing raw bytes panics as soon as a
/// cmdline contains a multi-byte character — including the U+FFFD that
/// `from_utf8_lossy` inserts for daemons that rewrite their argv.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}…", &s[..idx]),
        None => s.to_string(),
    }
}

/// SIGTERM a single PID, reporting the OS error instead of assuming success.
fn send_sigterm(pid: u32) -> Result<(), std::io::Error> {
    // SAFETY: kill(2) with a plain PID and a valid signal number; no memory is
    // shared with the kernel here.
    if unsafe { libc::kill(pid as i32, libc::SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// SIGTERM every PID in a group and summarise what actually happened. PIDs come
/// from the last scan, so some are expected to be gone already (ESRCH).
fn signal_all(pids: &[u32], group_name: &str) -> Toast {
    let mut sent = 0usize;
    let mut gone = 0usize;
    let mut denied = 0usize;
    let mut other: Option<std::io::Error> = None;

    for &pid in pids {
        match send_sigterm(pid) {
            Ok(()) => sent += 1,
            Err(e) => match e.raw_os_error() {
                Some(libc::ESRCH) => gone += 1,
                Some(libc::EPERM) => denied += 1,
                _ => other = Some(e),
            },
        }
    }

    if sent == 0 && denied > 0 {
        Toast::error(format!("{}: permission denied for all {} process(es)", group_name, denied))
    } else if let (0, Some(e)) = (sent, other) {
        Toast::error(format!("{}: {}", group_name, e))
    } else {
        let mut msg = format!("SIGTERM → {} ({}/{} procs)", group_name, sent, pids.len());
        if denied > 0 {
            msg.push_str(&format!(", {} denied", denied));
        }
        if gone > 0 {
            msg.push_str(&format!(", {} already gone", gone));
        }
        if denied > 0 { Toast::error(msg) } else { Toast::success(msg) }
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_chars;

    #[test]
    fn truncates_on_a_char_boundary() {
        // A 99-byte ASCII prefix followed by a multi-byte char puts byte index
        // 100 inside that char; slicing bytes here used to panic.
        let s = format!("{}ż_tail", "a".repeat(99));
        assert_eq!(truncate_chars(&s, 100), format!("{}ż…", "a".repeat(99)));

        // U+FFFD is what from_utf8_lossy emits for a rewritten argv.
        let lossy = format!("{}\u{FFFD}x", "b".repeat(99));
        assert_eq!(truncate_chars(&lossy, 100), format!("{}\u{FFFD}…", "b".repeat(99)));
    }

    #[test]
    fn leaves_short_strings_alone() {
        assert_eq!(truncate_chars("żółć", 100), "żółć");
        assert_eq!(truncate_chars("", 100), "");
    }
}
