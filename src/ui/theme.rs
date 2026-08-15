use egui::{Color32, Margin, Painter, Rect, Rounding, Stroke, Vec2, Visuals};

pub struct Theme;

impl Theme {
    // ── Background Layers ──
    pub const BG_BASE: Color32 = Color32::from_rgb(13, 17, 23);       // GitHub-dark inspired
    pub const BG_SURFACE: Color32 = Color32::from_rgb(22, 27, 34);    // cards
    pub const BG_SURFACE_HOVER: Color32 = Color32::from_rgb(30, 37, 48);
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(36, 43, 56);
    pub const BG_SUBROW: Color32 = Color32::from_rgb(17, 22, 29);
    pub const BG_BADGE: Color32 = Color32::from_rgb(28, 35, 46);

    // ── Borders ──
    pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(48, 54, 61);
    pub const BORDER_MUTED: Color32 = Color32::from_rgb(36, 42, 50);

    // ── Text ──
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 237, 243);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(139, 148, 158);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(110, 118, 129);

    // ── Accent palette (harmonious, not neon chaos) ──
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(88, 166, 255);
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(63, 185, 80);
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(163, 113, 247);
    pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(210, 153, 34);
    pub const ACCENT_RED: Color32 = Color32::from_rgb(248, 81, 73);
    pub const ACCENT_TEAL: Color32 = Color32::from_rgb(57, 211, 183);

    pub fn apply_to_ctx(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = Self::BG_BASE;
        visuals.window_fill = Self::BG_BASE;
        visuals.faint_bg_color = Self::BG_SURFACE;
        visuals.extreme_bg_color = Color32::from_rgb(8, 11, 16);

        visuals.widgets.noninteractive.bg_fill = Self::BG_SURFACE;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Self::BORDER_MUTED);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_PRIMARY);
        visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

        visuals.widgets.inactive.bg_fill = Self::BG_SURFACE;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Self::BORDER_MUTED);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_SECONDARY);
        visuals.widgets.inactive.rounding = Rounding::same(6.0);

        visuals.widgets.hovered.bg_fill = Self::BG_SURFACE_HOVER;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Self::ACCENT_BLUE);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(6.0);

        visuals.widgets.active.bg_fill = Self::BG_ELEVATED;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Self::ACCENT_PURPLE);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.active.rounding = Rounding::same(6.0);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(88, 166, 255, 40);
        visuals.selection.stroke = Stroke::new(1.0_f32, Self::ACCENT_BLUE);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 5.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.window_margin = Margin::same(16.0);
        ctx.set_style(style);
    }

    pub fn draw_card(painter: &Painter, rect: Rect, hover: bool, accent: Option<Color32>) {
        let bg = if hover { Self::BG_SURFACE_HOVER } else { Self::BG_SURFACE };
        let border = if hover {
            accent.unwrap_or(Self::ACCENT_BLUE)
        } else {
            accent.map(|c| Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), 60))
                .unwrap_or(Self::BORDER_DEFAULT)
        };

        painter.rect_filled(rect, Rounding::same(8.0), bg);
        painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, border));
    }

    pub fn draw_bar(painter: &Painter, rect: Rect, progress: f32, color: Color32) {
        painter.rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(22, 27, 34));
        let p = progress.clamp(0.0, 1.0);
        if p > 0.005 {
            let w = (rect.width() * p).max(6.0);
            let fill = Rect::from_min_size(rect.min, Vec2::new(w, rect.height()));
            painter.rect_filled(fill, Rounding::same(3.0), color);
        }
    }

    pub fn format_kb(kb: u64) -> String {
        if kb >= 1024 * 1024 {
            format!("{:.2} GB", kb as f64 / (1024.0 * 1024.0))
        } else if kb >= 1024 {
            format!("{:.1} MB", kb as f64 / 1024.0)
        } else {
            format!("{} KB", kb)
        }
    }
}
