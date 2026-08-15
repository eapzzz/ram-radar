use egui::{Color32, Rect, Ui, Vec2};
use crate::process::types::SystemMemoryInfo;
use crate::ui::animation::AnimationState;
use crate::ui::theme::Theme;

pub struct HeroMetrics;

impl HeroMetrics {
    pub fn show(ui: &mut Ui, sys_mem: &SystemMemoryInfo, _anim: &AnimationState, total_procs: usize) {
        let total_kb = sys_mem.total_kb;
        let pss_sum_kb = sys_mem.total_pss_sum_kb;
        let used_kb = sys_mem.used_kb();
        let available_kb = sys_mem.available_kb;
        let rss_sum_kb = sys_mem.total_rss_sum_kb;
        let swap_used_kb = sys_mem.swap_used_kb;
        let swap_total_kb = sys_mem.swap_total_kb;

        let pss_pct = if total_kb > 0 { (pss_sum_kb as f32 / total_kb as f32) * 100.0 } else { 0.0 };
        let used_pct = if total_kb > 0 { (used_kb as f32 / total_kb as f32) * 100.0 } else { 0.0 };

        let avail_width = ui.available_width();
        let gap = 10.0;
        let card_width = ((avail_width - gap * 3.0) / 4.0).max(180.0);
        let card_height = 86.0;

        ui.horizontal(|ui| {
            // Card 1: Real PSS RAM
            Self::render_card(
                ui,
                card_width,
                card_height,
                "⚡ REALNY RAM (PSS)",
                &Theme::format_kb(pss_sum_kb),
                &format!("z {} całkowitego ({:.1}%)", Theme::format_kb(total_kb), pss_pct),
                pss_pct / 100.0,
                Theme::ACCENT_CYAN,
                true,
            );
            ui.add_space(gap);

            // Card 2: System Used / Available
            Self::render_card(
                ui,
                card_width,
                card_height,
                "📊 CAŁKOWICIE ZAJĘTY (SYSTEM)",
                &Theme::format_kb(used_kb),
                &format!("{} wolnej pamięci", Theme::format_kb(available_kb)),
                used_pct / 100.0,
                Theme::ACCENT_PURPLE,
                false,
            );
            ui.add_space(gap);

            // Card 3: RSS Overestimation Warning
            let over_kb = rss_sum_kb.saturating_sub(pss_sum_kb);
            Self::render_card(
                ui,
                card_width,
                card_height,
                "⚠️ TRADYCYJNY RSS (ZAWYŻONY)",
                &Theme::format_kb(rss_sum_kb),
                &format!("+{} sztucznie zliczanego", Theme::format_kb(over_kb)),
                1.0,
                Theme::ACCENT_AMBER,
                false,
            );
            ui.add_space(gap);

            // Card 4: Swap & Processes
            let swap_pct = if swap_total_kb > 0 { (swap_used_kb as f32 / swap_total_kb as f32).clamp(0.0, 1.0) } else { 0.0 };
            Self::render_card(
                ui,
                card_width,
                card_height,
                "🔄 SWAP & PROCESY",
                &Theme::format_kb(swap_used_kb),
                &format!("Swap: {} • {} proc.", Theme::format_kb(swap_total_kb), total_procs),
                swap_pct,
                Theme::ACCENT_EMERALD,
                false,
            );
        });
    }

    fn render_card(
        ui: &mut Ui,
        width: f32,
        height: f32,
        title: &str,
        main_val: &str,
        sub_val: &str,
        progress: f32,
        accent: Color32,
        is_hero: bool,
    ) {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
        let painter = ui.painter();
        let is_hovered = response.hovered();

        // Draw card body with subtle glowing border
        Theme::draw_glass_card(painter, rect, is_hovered, Some(accent));

        // Draw content inside
        let inner_rect = rect.shrink2(Vec2::new(12.0, 10.0));
        
        // Title row
        let title_pos = inner_rect.min;
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            title,
            egui::FontId::proportional(10.5),
            Theme::TEXT_MUTED,
        );

        // Main Value
        let main_pos = egui::pos2(inner_rect.min.x, inner_rect.min.y + 16.0);
        let font_size = if is_hero { 22.0 } else { 19.0 };
        painter.text(
            main_pos,
            egui::Align2::LEFT_TOP,
            main_val,
            egui::FontId::proportional(font_size),
            if is_hero { Color32::WHITE } else { accent },
        );

        // Mini progress bar
        let bar_y = inner_rect.min.y + 44.0;
        let bar_rect = Rect::from_min_size(
            egui::pos2(inner_rect.min.x, bar_y),
            Vec2::new(inner_rect.width(), 4.0),
        );
        Theme::draw_progress_bar(painter, bar_rect, progress, accent);

        // Subtitle / info text
        let sub_pos = egui::pos2(inner_rect.min.x, inner_rect.min.y + 53.0);
        painter.text(
            sub_pos,
            egui::Align2::LEFT_TOP,
            sub_val,
            egui::FontId::proportional(11.0),
            Theme::TEXT_SECONDARY,
        );
    }
}
