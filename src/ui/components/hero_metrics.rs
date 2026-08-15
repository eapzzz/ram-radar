use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};
use crate::process::types::SystemMemoryInfo;
use crate::ui::animation::AnimationState;
use crate::ui::theme::Theme;

pub struct HeroMetrics;

impl HeroMetrics {
    pub fn show(ui: &mut Ui, sys_mem: &SystemMemoryInfo, _anim: &AnimationState, total_procs: usize) {
        let total_kb = sys_mem.total_kb;
        let pss_kb = sys_mem.total_pss_sum_kb;
        let used_kb = sys_mem.used_kb();
        let avail_kb = sys_mem.available_kb;
        let rss_kb = sys_mem.total_rss_sum_kb;
        let swap_used = sys_mem.swap_used_kb;
        let swap_total = sys_mem.swap_total_kb;

        let pss_pct = if total_kb > 0 { pss_kb as f32 / total_kb as f32 } else { 0.0 };
        let used_pct = if total_kb > 0 { used_kb as f32 / total_kb as f32 } else { 0.0 };
        let swap_pct = if swap_total > 0 { (swap_used as f32 / swap_total as f32).clamp(0.0, 1.0) } else { 0.0 };
        let inflation_kb = rss_kb.saturating_sub(pss_kb);

        // Use egui widgets for reliable layout instead of raw painter drawing
        ui.columns(4, |cols| {
            // Card 1: Real PSS
            Self::card(&mut cols[0], "Real RAM (PSS)", &Theme::format_kb(pss_kb),
                &format!("{:.1}% of {}", pss_pct * 100.0, Theme::format_kb(total_kb)),
                pss_pct, Theme::ACCENT_BLUE);

            // Card 2: System Used
            Self::card(&mut cols[1], "System Used", &Theme::format_kb(used_kb),
                &format!("{} available", Theme::format_kb(avail_kb)),
                used_pct, Theme::ACCENT_PURPLE);

            // Card 3: RSS Inflation
            Self::card(&mut cols[2], "RSS Inflation", &format!("+{}", Theme::format_kb(inflation_kb)),
                &format!("RSS: {} overcounted", Theme::format_kb(rss_kb)),
                1.0_f32, Theme::ACCENT_ORANGE);

            // Card 4: Swap & Procs
            Self::card(&mut cols[3], "Swap & Processes", &Theme::format_kb(swap_used),
                &format!("{} swap · {} procs", Theme::format_kb(swap_total), total_procs),
                swap_pct, Theme::ACCENT_GREEN);
        });
    }

    fn card(ui: &mut Ui, title: &str, value: &str, subtitle: &str, progress: f32, accent: Color32) {
        Frame::none()
            .fill(Theme::BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_DEFAULT))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // Draw left accent bar
                let rect = ui.max_rect();
                let accent_bar = egui::Rect::from_min_size(
                    rect.left_top(),
                    Vec2::new(3.0, rect.height()),
                );
                ui.painter().rect_filled(accent_bar, Rounding::same(2.0), accent);

                ui.label(RichText::new(title).size(10.5).color(Theme::TEXT_MUTED));
                ui.label(RichText::new(value).size(20.0).color(accent).strong());

                // Progress bar
                let bar_h = 3.0_f32;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_h), egui::Sense::hover());
                Theme::draw_bar(ui.painter(), rect, progress, accent);

                ui.label(RichText::new(subtitle).size(10.0).color(Theme::TEXT_SECONDARY));
            });
    }
}
