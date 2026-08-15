use egui::{Color32, Frame, Margin, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};
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

        Frame::none()
            .fill(Theme::BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_DEFAULT))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Memory Breakdown").size(12.0).strong().color(Theme::TEXT_PRIMARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!(
                            "{} / {} ({:.1}%)",
                            Theme::format_kb(sys_mem.total_pss_sum_kb),
                            Theme::format_kb(total_kb),
                            sys_mem.pss_percentage()
                        )).size(11.0).color(Theme::ACCENT_BLUE));
                    });
                });

                ui.add_space(4.0);

                // ── Segmented Bar ──
                Self::draw_segmented_bar(ui, top_groups, total_kb);

                ui.add_space(4.0);

                // ── Sparkline ──
                Self::draw_sparkline(ui, anim, total_kb);

                ui.add_space(4.0);

                // ── Legend ──
                ui.horizontal_wrapped(|ui| {
                    for group in top_groups.iter().take(6) {
                        if group.total_pss_kb == 0 { continue; }
                        let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
                        let pct = (group.total_pss_kb as f32 / total_kb as f32) * 100.0;
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.5, col);
                        ui.label(egui::RichText::new(format!("{} {:.1}%", group.display_name, pct)).size(10.0).color(Theme::TEXT_SECONDARY));
                        ui.add_space(4.0);
                    }
                });
            });
    }

    fn draw_segmented_bar(ui: &mut Ui, top_groups: &[&AppGroup], total_kb: u64) {
        let bar_h = 12.0_f32;
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_h), Sense::hover());
        let painter = ui.painter();

        painter.rect_filled(bar_rect, Rounding::same(6.0), Color32::from_rgb(22, 27, 34));

        let mut x = bar_rect.min.x;
        for group in top_groups.iter().take(8) {
            if group.total_pss_kb == 0 { continue; }
            let w = (group.total_pss_kb as f32 / total_kb as f32) * bar_rect.width();
            if w < 1.5 { continue; }
            let seg = Rect::from_min_size(egui::pos2(x, bar_rect.min.y), Vec2::new(w, bar_h));
            let col = Color32::from_rgb(group.accent_color[0], group.accent_color[1], group.accent_color[2]);
            painter.rect_filled(seg, Rounding::ZERO, col);
            x += w;
        }
    }

    fn draw_sparkline(ui: &mut Ui, anim: &AnimationState, total_kb: u64) {
        if anim.history.len() < 2 { return; }

        let spark_h = 28.0_f32;
        let (spark_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), spark_h), Sense::hover());
        let painter = ui.painter();
        let max_gb = (total_kb as f32 / (1024.0 * 1024.0)).max(1.0);
        let count = anim.history.len();
        let step = spark_rect.width() / (count - 1).max(1) as f32;

        let pts: Vec<Pos2> = anim.history.iter().enumerate().map(|(i, &(v, _))| {
            let ny = (v / max_gb).clamp(0.0, 1.0);
            Pos2::new(spark_rect.min.x + i as f32 * step, spark_rect.max.y - ny * spark_rect.height())
        }).collect();

        painter.add(egui::Shape::line(pts, Stroke::new(1.5_f32, Theme::ACCENT_BLUE)));
    }
}
