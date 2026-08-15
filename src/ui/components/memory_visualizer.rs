use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};
use crate::process::types::{AppGroup, SystemMemoryInfo};
use crate::ui::animation::AnimationState;
use crate::ui::theme::Theme;

pub struct MemoryVisualizer;

impl MemoryVisualizer {
    pub fn show(ui: &mut Ui, sys_mem: &SystemMemoryInfo, top_groups: &[&AppGroup], anim: &AnimationState) {
        let total_kb = sys_mem.total_kb;
        if total_kb == 0 {
            return;
        }

        ui.vertical(|ui| {
            // Header Row
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📊 Podział Pamięci RAM (PSS) & Trend w Czasie")
                        .size(13.0)
                        .strong()
                        .color(Theme::TEXT_PRIMARY),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pss_str = Theme::format_kb(sys_mem.total_pss_sum_kb);
                    let tot_str = Theme::format_kb(sys_mem.total_kb);
                    let pct = sys_mem.pss_percentage();
                    ui.label(
                        egui::RichText::new(format!("Aktywne procesy: {} / {} ({:.1}%)", pss_str, tot_str, pct))
                            .size(12.0)
                            .color(Theme::ACCENT_CYAN)
                            .strong(),
                    );
                });
            });

            ui.add_space(4.0);

            // Container frame for Bar + Sparkline
            let avail_width = ui.available_width();
            let container_height = 54.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_width, container_height), Sense::hover());
            let painter = ui.painter();

            // Background glass container
            Theme::draw_glass_card(painter, rect, false, Some(Theme::BORDER_GLASS));

            let inner_rect = rect.shrink2(Vec2::new(10.0, 8.0));

            // 1. Stacked Segmented RAM Bar
            let bar_height = 14.0;
            let bar_rect = Rect::from_min_size(inner_rect.min, Vec2::new(inner_rect.width(), bar_height));
            
            // Bar background
            painter.rect_filled(
                bar_rect,
                Rounding::same(7.0),
                Color32::from_rgb(14, 18, 28),
            );

            let mut current_x = bar_rect.min.x;
            let mut top_rendered_kb = 0u64;

            for group in top_groups.iter().take(8) {
                let group_kb = group.total_pss_kb;
                if group_kb == 0 {
                    continue;
                }
                top_rendered_kb += group_kb;

                let segment_width = ((group_kb as f32 / total_kb as f32) * bar_rect.width()).max(2.0);
                let seg_rect = Rect::from_min_size(
                    egui::pos2(current_x, bar_rect.min.y),
                    Vec2::new(segment_width, bar_height),
                );

                let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
                painter.rect_filled(seg_rect, Rounding::ZERO, col);

                current_x += segment_width;
            }

            // Other processes segment
            let other_kb = sys_mem.total_pss_sum_kb.saturating_sub(top_rendered_kb);
            if other_kb > 0 {
                let segment_width = (other_kb as f32 / total_kb as f32) * bar_rect.width();
                if segment_width >= 2.0 {
                    let seg_rect = Rect::from_min_size(
                        egui::pos2(current_x, bar_rect.min.y),
                        Vec2::new(segment_width, bar_height),
                    );
                    painter.rect_filled(seg_rect, Rounding::ZERO, Color32::from_rgb(80, 95, 120));
                }
            }

            // 2. Live Sparkline Trend
            let spark_y = inner_rect.min.y + 20.0;
            let spark_height = 18.0;
            let spark_rect = Rect::from_min_size(
                egui::pos2(inner_rect.min.x, spark_y),
                Vec2::new(inner_rect.width(), spark_height),
            );

            if anim.history.len() >= 2 {
                let max_gb = (total_kb as f32 / (1024.0 * 1024.0)).max(1.0);
                let count = anim.history.len();
                let step_x = spark_rect.width() / (count - 1).max(1) as f32;

                let mut points: Vec<Pos2> = Vec::with_capacity(count);
                for (idx, &(val_gb, _)) in anim.history.iter().enumerate() {
                    let normalized_y = (val_gb / max_gb).clamp(0.0, 1.0);
                    let x = spark_rect.min.x + idx as f32 * step_x;
                    let y = spark_rect.max.y - (normalized_y * spark_rect.height());
                    points.push(Pos2::new(x, y));
                }

                // Draw smooth gradient wave line
                painter.add(egui::Shape::line(
                    points,
                    Stroke::new(1.5_f32, Theme::ACCENT_CYAN),
                ));
            } else {
                painter.text(
                    spark_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Zbieranie próbek historii zużycia w czasie rzeczywistym...",
                    egui::FontId::proportional(10.0),
                    Theme::TEXT_MUTED,
                );
            }

            ui.add_space(6.0);

            // 3. Compact Legend Pills
            ui.horizontal_wrapped(|ui| {
                for group in top_groups.iter().take(7) {
                    if group.total_pss_kb == 0 {
                        continue;
                    }
                    let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
                    let pct = (group.total_pss_kb as f32 / total_kb as f32) * 100.0;

                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 4.0, col);

                    ui.label(
                        egui::RichText::new(format!("{}: {} ({:.1}%)", group.display_name, Theme::format_kb(group.total_pss_kb), pct))
                            .size(11.0)
                            .color(Theme::TEXT_SECONDARY),
                    );
                    ui.add_space(8.0);
                }
            });
        });
    }
}
