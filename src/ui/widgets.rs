use super::theme::*;
use egui::{self, Color32, RichText, Stroke, Vec2};
pub fn panel(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(PANEL)
        .rounding(12.0)
        .inner_margin(20.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}
pub fn heading(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(RichText::new(title).size(30.0).strong());
    ui.label(muted(sub));
    ui.add_space(14.0);
}
pub fn meter(ui: &mut egui::Ui, value: f32, color: Color32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 3.0, LINE);
    let mut fill = r;
    fill.max.x = fill.min.x + fill.width() * (value / 100.0).clamp(0.0, 1.0);
    ui.painter().rect_filled(fill, 3.0, color);
}
pub fn metric(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    sub: &str,
    percent: f32,
    color: Color32,
) {
    panel(ui, |ui| {
        ui.label(muted(label));
        ui.label(RichText::new(value).size(29.0).color(color));
        ui.label(RichText::new(sub).size(12.0).color(MUTED));
        ui.add_space(8.0);
        meter(ui, percent, color);
    });
}
fn time_fraction(time: u64, first: u64, last: u64) -> f32 {
    time.saturating_sub(first) as f32 / last.saturating_sub(first).max(1) as f32
}
pub fn chart(
    ui: &mut egui::Ui,
    series: &[(&[f32], Color32)],
    times: &[(u64, bool)],
    height: f32,
    max: f32,
) {
    let (r, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(r);
    let first = times.first().map(|t| t.0).unwrap_or(0);
    let last = times.last().map(|t| t.0).unwrap_or(first);
    for i in 0..=4 {
        let y = r.top() + r.height() * i as f32 / 4.0;
        painter.line_segment(
            [egui::pos2(r.left(), y), egui::pos2(r.right(), y)],
            Stroke::new(1.0_f32, LINE),
        );
    }
    for (values, color) in series {
        for i in 1..values.len().min(times.len()) {
            if times[i].1 {
                continue;
            }
            let point = |j: usize| {
                egui::pos2(
                    r.left() + time_fraction(times[j].0, first, last) * r.width(),
                    r.bottom() - (values[j] / max.max(1.0)).clamp(0.0, 1.0) * r.height(),
                )
            };
            painter.line_segment([point(i - 1), point(i)], Stroke::new(2.0_f32, *color));
        }
    }
    if let Some(pos) = response.hover_pos() {
        painter.line_segment(
            [egui::pos2(pos.x, r.top()), egui::pos2(pos.x, r.bottom())],
            Stroke::new(1.0_f32, MUTED),
        );
        let target =
            first + ((pos.x - r.left()) / r.width() * last.saturating_sub(first) as f32) as u64;
        response.on_hover_ui(|ui| {
            if let Some(i) = times
                .iter()
                .enumerate()
                .min_by_key(|(_, t)| t.0.abs_diff(target))
                .map(|(i, _)| i)
            {
                ui.label(format!(
                    "{} seconds before latest sample",
                    last.saturating_sub(times[i].0)
                ));
                for (values, color) in series {
                    if let Some(value) = values.get(i) {
                        ui.colored_label(*color, format!("{value:.1}"));
                    }
                }
            }
        });
    }
    ui.horizontal(|ui| {
        ui.label(muted(format!("{}s ago", last.saturating_sub(first))).size(10.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(muted("Latest").size(10.0));
        });
    });
}
pub fn icon(ui: &mut egui::Ui, name: &str, path: Option<&std::path::Path>, size: f32) {
    if let Some(path) = path {
        ui.add(
            egui::Image::from_uri(format!("file://{}", path.display()))
                .fit_to_exact_size(egui::vec2(size, size)),
        );
    } else {
        let (r, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let color = [BLUE, GREEN, PEACH, PURPLE]
            [name.bytes().fold(0usize, |a, b| a.wrapping_add(b as usize)) % 4];
        ui.painter().rect_filled(r, 8.0, color.gamma_multiply(0.18));
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            name.chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string(),
            egui::FontId::proportional(size * 0.47),
            color,
        );
    }
}
pub fn keyvalue(ui: &mut egui::Ui, key: &str, value: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(muted(key));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(value.into());
        });
    });
}
pub fn nav_icon(ui: &mut egui::Ui, index: usize, color: Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
    let p = ui.painter();
    let s = Stroke::new(1.5_f32, color);
    match index {
        0 => {
            for x in 0..2 {
                for y in 0..2 {
                    p.rect_stroke(
                        egui::Rect::from_min_size(
                            r.min + egui::vec2(x as f32 * 10.0, y as f32 * 10.0),
                            egui::vec2(6.0, 6.0),
                        ),
                        1.0,
                        s,
                    );
                }
            }
        }
        1 => {
            for y in 0..3 {
                let o = r.min + egui::vec2(0.0, y as f32 * 6.0 + 2.0);
                p.circle_filled(o + egui::vec2(1.0, 0.0), 1.5, color);
                p.line_segment([o + egui::vec2(6.0, 0.0), o + egui::vec2(17.0, 0.0)], s);
            }
        }
        2 => {
            p.rect_stroke(r.shrink(3.0), 2.0, s);
            for i in 0..3 {
                let x = r.left() + 5.0 + i as f32 * 4.0;
                p.line_segment([egui::pos2(x, r.top()), egui::pos2(x, r.bottom())], s);
            }
        }
        3 => {
            p.line_segment([r.left_bottom(), r.right_bottom()], s);
            p.add(egui::Shape::line(
                vec![
                    r.left_center(),
                    r.center(),
                    r.center() + egui::vec2(4.0, -7.0),
                    r.right_center(),
                ],
                s,
            ));
        }
        _ => {
            p.circle_stroke(r.center(), 7.0, s);
            p.circle_stroke(r.center(), 2.0, s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chart_positions_preserve_elapsed_time() {
        assert_eq!(time_fraction(100, 100, 200), 0.0);
        assert_eq!(time_fraction(110, 100, 200), 0.1);
        assert_eq!(time_fraction(200, 100, 200), 1.0);
    }
}
