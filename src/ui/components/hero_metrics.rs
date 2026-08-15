use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};
use crate::process::types::SystemMemoryInfo;
use crate::ui::animation::AnimationState;
use crate::ui::theme::Theme;

pub struct HeroMetrics;

/// Inner padding of a hero card; also how far the accent bar is pulled back out.
const CARD_MARGIN: f32 = 10.0;

fn gb_to_kb(gb: f32) -> u64 {
    (gb.max(0.0) * 1024.0 * 1024.0) as u64
}

impl HeroMetrics {
    pub fn show(ui: &mut Ui, sys_mem: &SystemMemoryInfo, anim: &AnimationState, total_procs: usize) {
        let total_kb = sys_mem.total_kb;
        let rss_kb = sys_mem.total_rss_sum_kb;
        let swap_used = sys_mem.swap_used_kb;
        let swap_total = sys_mem.swap_total_kb;

        // Display the eased values so the headline figures glide between scans
        // instead of snapping. `AnimationState::update` is driven every frame by
        // the app; reading it here is what makes that work actually visible.
        let pss_kb = gb_to_kb(anim.animated_pss_gb);
        let used_kb = gb_to_kb(anim.animated_used_gb);

        let frac = |part: u64| if total_kb > 0 { (part as f32 / total_kb as f32).clamp(0.0, 1.0) } else { 0.0 };
        let pss_pct = frac(pss_kb);
        let used_pct = frac(used_kb);
        let swap_pct = if swap_total > 0 { (swap_used as f32 / swap_total as f32).clamp(0.0, 1.0) } else { 0.0 };

        // Compare like with like: both sums come from the same scan, and RSS is
        // always >= PSS, so this is the double-counting the tool exists to show.
        let inflation_kb = rss_kb.saturating_sub(sys_mem.total_pss_sum_kb);

        // Any process whose smaps_rollup we could not read contributes RSS in
        // place of PSS. Say so rather than letting it silently inflate the
        // headline number and zero out the inflation figure.
        let mut pss_sub = format!("{:.1}% of {}", pss_pct * 100.0, Theme::format_kb(total_kb));
        if sys_mem.estimated_pss_kb > 0 {
            pss_sub.push_str(&format!(
                " · {} est. from RSS ({} procs)",
                Theme::format_kb(sys_mem.estimated_pss_kb),
                sys_mem.estimated_proc_count
            ));
        }

        ui.columns(4, |cols| {
            Self::card(&mut cols[0], "Real RAM (PSS)", &Theme::format_kb(pss_kb),
                &pss_sub, pss_pct, Theme::ACCENT_BLUE);

            Self::card(&mut cols[1], "System Used", &Theme::format_kb(used_kb),
                &format!("{} available", Theme::format_kb(sys_mem.available_kb)),
                used_pct, Theme::ACCENT_PURPLE);

            Self::card(&mut cols[2], "RSS Inflation", &format!("+{}", Theme::format_kb(inflation_kb)),
                &format!("RSS: {} overcounted", Theme::format_kb(rss_kb)),
                frac(inflation_kb), Theme::ACCENT_ORANGE);

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
            .inner_margin(Margin::same(CARD_MARGIN))
            .show(ui, |ui| {
                let accent_bar = Theme::begin_accent_bar(ui);
                ui.set_min_width(ui.available_width());

                ui.label(RichText::new(title).size(10.5).color(Theme::TEXT_MUTED));
                ui.label(RichText::new(value).size(20.0).color(accent).strong());

                let bar_h = 3.0_f32;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_h), egui::Sense::hover());
                Theme::draw_bar(ui.painter(), rect, progress, accent);

                ui.label(RichText::new(subtitle).size(10.0).color(Theme::TEXT_SECONDARY));

                Theme::end_accent_bar(ui, accent_bar, CARD_MARGIN, accent);
            });
    }
}
