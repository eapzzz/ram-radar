use egui::layers::ShapeIdx;
use egui::{Color32, Margin, Painter, Rect, Rounding, Shape, Stroke, Ui, Vec2, Visuals};

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

    /// Width of the coloured bar drawn down the left edge of a card.
    const ACCENT_BAR_W: f32 = 3.0;

    /// Reserve the paint slot for a card's left accent bar.
    ///
    /// Call this first thing inside a [`egui::Frame`] body and pass the returned
    /// index to [`Theme::end_accent_bar`] as the last thing in that body.
    ///
    /// The two-step dance is required because `Frame::begin` hands its content
    /// `Ui` a `max_rect` covering *all* remaining space in the parent, not the
    /// card's eventual size, and it cannot narrow the clip rect either ("we
    /// don't know final size yet"). Sizing the bar from `ui.max_rect()` up front
    /// therefore paints a stripe down the whole panel. Reserving a slot and
    /// filling it from `ui.min_rect()` at the end uses the card's real bounds
    /// while keeping the bar behind the card's content in z-order.
    pub fn begin_accent_bar(ui: &Ui) -> ShapeIdx {
        ui.painter().add(Shape::Noop)
    }

    /// Fill the slot from [`Theme::begin_accent_bar`] using the card's final
    /// rect. `inner_margin` must match the frame's inner margin so the bar sits
    /// flush with the card edge rather than inset by the padding.
    pub fn end_accent_bar(ui: &Ui, idx: ShapeIdx, inner_margin: f32, color: Color32) {
        let card = ui.min_rect().expand(inner_margin);
        let bar = Rect::from_min_size(card.left_top(), Vec2::new(Self::ACCENT_BAR_W, card.height()));
        ui.painter().set(idx, Shape::rect_filled(bar, Rounding::same(2.0), color));
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
