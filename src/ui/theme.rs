use egui::{Color32, RichText};
pub const BG: Color32 = Color32::from_rgb(21, 27, 38);
pub const PANEL: Color32 = Color32::from_rgb(29, 37, 50);
pub const LINE: Color32 = Color32::from_rgb(48, 60, 77);
pub const TEXT: Color32 = Color32::from_rgb(237, 241, 247);
pub const MUTED: Color32 = Color32::from_rgb(158, 174, 195);
pub const BLUE: Color32 = Color32::from_rgb(140, 183, 245);
pub const PEACH: Color32 = Color32::from_rgb(241, 185, 149);
pub const GREEN: Color32 = Color32::from_rgb(134, 202, 181);
pub const PURPLE: Color32 = Color32::from_rgb(189, 167, 228);
pub fn setup(ctx: &egui::Context) {
    let mut s = (*ctx.style()).clone();
    s.visuals = egui::Visuals::dark();
    s.visuals.panel_fill = BG;
    s.visuals.window_fill = PANEL;
    s.visuals.extreme_bg_color = BG;
    s.visuals.faint_bg_color = PANEL;
    s.visuals.override_text_color = Some(TEXT);
    s.visuals.selection.bg_fill = Color32::from_rgb(48, 72, 104);
    s.visuals.selection.stroke = egui::Stroke::new(1.0_f32, BLUE);
    s.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, LINE);
    s.visuals.widgets.inactive.weak_bg_fill = PANEL;
    s.visuals.widgets.hovered.weak_bg_fill = LINE;
    s.spacing.item_spacing = egui::vec2(12.0, 10.0);
    s.spacing.button_padding = egui::vec2(12.0, 8.0);
    s.spacing.interact_size.y = 32.0;
    s.text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    s.text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    s.text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    ctx.set_style(s);
}
pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(MUTED)
}
pub fn bytes(n: f64) -> String {
    if n >= 1073741824.0 {
        format!("{:.1} GiB", n / 1073741824.0)
    } else if n >= 1048576.0 {
        format!("{:.1} MiB", n / 1048576.0)
    } else if n >= 1024.0 {
        format!("{:.0} KiB", n / 1024.0)
    } else {
        format!("{n:.0} B")
    }
}
pub fn kb(n: u64) -> String {
    bytes(n as f64 * 1024.0)
}
pub fn rate(n: f64) -> String {
    format!("{}/s", bytes(n))
}
pub fn pct(n: f32) -> String {
    format!("{n:.1}%")
}
pub fn duration(n: f64) -> String {
    let n = n as u64;
    format!("{}h {}m", n / 3600, (n % 3600) / 60)
}
