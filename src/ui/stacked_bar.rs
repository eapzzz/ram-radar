use egui::{Color32, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};
use crate::process::types::{AppGroup, SystemMemoryInfo};
use crate::ui::theme::Theme;

pub struct StackedRamBar;

impl StackedRamBar {
    pub fn show(ui: &mut Ui, sys_mem: &SystemMemoryInfo, top_groups: &[&AppGroup]) {
        let total_kb = sys_mem.total_kb;
        if total_kb == 0 {
            return;
        }

        ui.vertical(|ui| {
            // Header stats
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📊 Podział Pamięci RAM (Realne PSS):")
                        .color(Theme::TEXT_PRIMARY)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pss_str = Theme::format_kb(sys_mem.total_pss_sum_kb);
                    let tot_str = Theme::format_kb(sys_mem.total_kb);
                    let pct = sys_mem.pss_percentage();
                    ui.label(
                        egui::RichText::new(format!("Suma PSS procesów: {} / {} ({:.1}%)", pss_str, tot_str, pct))
                            .color(Theme::ACCENT_CYAN)
                            .strong(),
                    );
                });
            });

            ui.add_space(4.0);

            // Multi-segment Bar
            let available_width = ui.available_width();
            let bar_height = 20.0;
            let (rect, _response) = ui.allocate_exact_size(Vec2::new(available_width, bar_height), Sense::hover());
            let painter = ui.painter();

            // Background of whole bar
            painter.rect_filled(
                rect,
                Rounding::same(6.0),
                Color32::from_rgb(28, 33, 48),
            );
            painter.rect_stroke(
                rect,
                Rounding::same(6.0),
                Stroke::new(1.0_f32, Theme::BORDER_DARK),
            );

            let mut current_x = rect.min.x;

            // Render top apps
            let mut top_rendered_kb = 0u64;
            for group in top_groups.iter().take(8) {
                let group_kb = group.total_pss_kb;
                if group_kb == 0 {
                    continue;
                }
                top_rendered_kb += group_kb;

                let segment_width = (group_kb as f32 / total_kb as f32) * rect.width();
                if segment_width < 2.0 {
                    continue;
                }

                let seg_rect = Rect::from_min_size(
                    egui::pos2(current_x, rect.min.y),
                    Vec2::new(segment_width, bar_height),
                );

                let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
                painter.rect_filled(seg_rect, Rounding::ZERO, col);

                current_x += segment_width;
            }

            // Other processes segment
            let other_kb = sys_mem.total_pss_sum_kb.saturating_sub(top_rendered_kb);
            if other_kb > 0 {
                let segment_width = (other_kb as f32 / total_kb as f32) * rect.width();
                if segment_width >= 2.0 {
                    let seg_rect = Rect::from_min_size(
                        egui::pos2(current_x, rect.min.y),
                        Vec2::new(segment_width, bar_height),
                    );
                    painter.rect_filled(seg_rect, Rounding::ZERO, Color32::from_rgb(100, 116, 139));
                }
            }

            ui.add_space(4.0);

            // Legend
            ui.horizontal_wrapped(|ui| {
                for group in top_groups.iter().take(7) {
                    if group.total_pss_kb == 0 {
                        continue;
                    }
                    let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
                    
                    // Small color box
                    let (box_rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                    ui.painter().rect_filled(box_rect, Rounding::same(2.0), col);

                    let pct = (group.total_pss_kb as f32 / total_kb as f32) * 100.0;
                    ui.label(
                        egui::RichText::new(format!("{}: {} ({:.1}%)", group.display_name, Theme::format_kb(group.total_pss_kb), pct))
                            .size(11.0)
                            .color(Theme::TEXT_SECONDARY),
                    );
                    ui.add_space(6.0);
                }
            });
        });
    }
}
