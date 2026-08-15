use egui::{Color32, Margin, Painter, Rect, Rounding, Stroke, Vec2, Visuals};

pub struct Theme;

impl Theme {
    // Deep Space / Glassmorphism Palette
    pub const BG_DEEP: Color32 = Color32::from_rgb(10, 13, 20);
    pub const BG_HEADER: Color32 = Color32::from_rgb(14, 18, 28);
    pub const BG_CARD: Color32 = Color32::from_rgb(18, 23, 36);
    pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(25, 32, 50);
    pub const BG_CARD_ACTIVE: Color32 = Color32::from_rgb(30, 39, 62);
    pub const BG_SUBROW: Color32 = Color32::from_rgb(13, 17, 26);
    pub const BG_PILL: Color32 = Color32::from_rgb(28, 36, 56);

    // Borders & Glass
    pub const BORDER_GLASS: Color32 = Color32::from_rgb(35, 45, 70);
    pub const BORDER_GLASS_LIGHT: Color32 = Color32::from_rgb(50, 65, 100);
    pub const BORDER_ACCENT: Color32 = Color32::from_rgb(99, 102, 241);

    // Typography
    pub const TEXT_TITLE: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(241, 245, 249);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(148, 163, 184);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 116, 139);

    // Cyberpunk / Neon Accents
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);
    pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(139, 92, 246);
    pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(52, 211, 153);
    pub const ACCENT_ROSE: Color32 = Color32::from_rgb(251, 113, 133);
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);

    pub fn apply_to_ctx(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = Self::BG_DEEP;
        visuals.window_fill = Self::BG_DEEP;
        visuals.faint_bg_color = Self::BG_HEADER;
        visuals.extreme_bg_color = Color32::from_rgb(7, 9, 14);

        // Buttons and Widgets
        visuals.widgets.noninteractive.bg_fill = Self::BG_CARD;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Self::BORDER_GLASS);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_PRIMARY);
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = Self::BG_PILL;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Self::BORDER_GLASS);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_PRIMARY);
        visuals.widgets.inactive.rounding = Rounding::same(8.0);

        visuals.widgets.hovered.bg_fill = Self::BG_CARD_HOVER;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Self::ACCENT_CYAN);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(8.0);

        visuals.widgets.active.bg_fill = Self::BG_CARD_ACTIVE;
        visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, Self::ACCENT_PURPLE);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.active.rounding = Rounding::same(8.0);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(99, 102, 241, 70);
        visuals.selection.stroke = Stroke::new(1.0_f32, Self::ACCENT_VIOLET);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.window_margin = Margin::same(14.0);
        ctx.set_style(style);
    }

    pub fn draw_glass_card(painter: &Painter, rect: Rect, is_hovered: bool, accent: Option<Color32>) {
        let bg_color = if is_hovered {
            Self::BG_CARD_HOVER
        } else {
            Self::BG_CARD
        };

        let stroke_color = if is_hovered {
            accent.unwrap_or(Self::ACCENT_CYAN)
        } else {
            accent.map(|c| Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), 100)).unwrap_or(Self::BORDER_GLASS)
        };

        // Draw shadow glow if hovered
        if is_hovered {
            let glow_rect = rect.expand(1.5);
            painter.rect_filled(
                glow_rect,
                Rounding::same(12.0),
                Color32::from_rgba_premultiplied(stroke_color.r(), stroke_color.g(), stroke_color.b(), 25),
            );
        }

        // Draw main body
        painter.rect_filled(rect, Rounding::same(10.0), bg_color);
        painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0_f32, stroke_color));
    }

    pub fn draw_progress_bar(painter: &Painter, rect: Rect, progress: f32, color: Color32) {
        // Background track
        painter.rect_filled(rect, Rounding::same(rect.height() * 0.5), Color32::from_rgb(24, 30, 46));
        
        let p = progress.clamp(0.0, 1.0);
        if p > 0.001 {
            let fill_width = (rect.width() * p).max(rect.height());
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
            painter.rect_filled(fill_rect, Rounding::same(rect.height() * 0.5), color);
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
