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
pub enum ChartUnit {
    Percent,
    BytesPerSecond,
}
pub struct ChartSeries<'a> {
    pub name: &'a str,
    pub values: &'a [f32],
    pub color: Color32,
    pub unit: ChartUnit,
}
impl<'a> ChartSeries<'a> {
    pub fn percent(name: &'a str, values: &'a [f32], color: Color32) -> Self {
        Self {
            name,
            values,
            color,
            unit: ChartUnit::Percent,
        }
    }
    pub fn rate(name: &'a str, values: &'a [f32], color: Color32) -> Self {
        Self {
            name,
            values,
            color,
            unit: ChartUnit::BytesPerSecond,
        }
    }
}
pub fn chart(
    ui: &mut egui::Ui,
    series: &[ChartSeries<'_>],
    times: &[(u64, bool)],
    height: f32,
    max: f32,
    window_seconds: u64,
) {
    let (r, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(r);
    let last = times.last().map(|t| t.0).unwrap_or(0);
    let first = last.saturating_sub(window_seconds.max(1));
    for i in 0..=4 {
        let y = r.top() + r.height() * i as f32 / 4.0;
        painter.line_segment(
            [egui::pos2(r.left(), y), egui::pos2(r.right(), y)],
            Stroke::new(1.0_f32, LINE),
        );
    }
    for line in series {
        let values = line.values;
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
            painter.line_segment([point(i - 1), point(i)], Stroke::new(2.0_f32, line.color));
        }
        // A single collected sample still has a visible point.
        for (i, _) in times.iter().enumerate().take(values.len()) {
            if values.len() == 1 || times[i].1 {
                painter.circle_filled(
                    egui::pos2(
                        r.left() + time_fraction(times[i].0, first, last) * r.width(),
                        r.bottom() - (values[i] / max.max(1.0)).clamp(0.0, 1.0) * r.height(),
                    ),
                    2.5,
                    line.color,
                );
            }
        }
    }
    if let Some(pos) = response.hover_pos() {
        let target = first as f64
            + ((pos.x - r.left()) / r.width()).clamp(0.0, 1.0) as f64
                * last.saturating_sub(first) as f64;
        let tolerance = last.saturating_sub(first) as f64 * 4.0 / r.width().max(1.0) as f64;
        let selected = nearest_sample(times, target, tolerance);
        let x = selected
            .map(|i| r.left() + time_fraction(times[i].0, first, last) * r.width())
            .unwrap_or(pos.x);
        painter.line_segment(
            [egui::pos2(x, r.top()), egui::pos2(x, r.bottom())],
            Stroke::new(1.0_f32, MUTED),
        );
        if let Some(i) = selected {
            for line in series {
                if let Some(value) = line.values.get(i) {
                    painter.circle_filled(
                        egui::pos2(
                            x,
                            r.bottom() - (value / max.max(1.0)).clamp(0.0, 1.0) * r.height(),
                        ),
                        4.0,
                        line.color,
                    );
                }
            }
        }
        egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id, |ui| {
            if let Some(i) = selected {
                ui.strong(format!("Sample {}", sample_clock(times[i].0)));
                ui.label(muted(format!(
                    "{} before latest sample",
                    time_span(last.saturating_sub(times[i].0))
                )));
                for line in series {
                    if let Some(value) = line.values.get(i) {
                        let formatted = match line.unit {
                            ChartUnit::Percent => pct(*value),
                            ChartUnit::BytesPerSecond => rate(*value as f64),
                        };
                        ui.colored_label(line.color, format!("{}: {formatted}", line.name));
                    }
                }
            } else {
                ui.label("No sample at this time");
            }
        });
    }
    ui.horizontal(|ui| {
        ui.label(muted(format!("{} ago", time_span(window_seconds))).size(10.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(muted("Latest").size(10.0));
        });
    });
}
fn nearest_sample(times: &[(u64, bool)], target: f64, tolerance: f64) -> Option<usize> {
    // Markers remain selectable at window edges and next to collection gaps.
    if target < times.first()?.0 as f64 - tolerance || target > times.last()?.0 as f64 + tolerance {
        return None;
    }
    if times.windows(2).any(|pair| {
        pair[1].1 && target > pair[0].0 as f64 + tolerance && target < pair[1].0 as f64 - tolerance
    }) {
        return None;
    }
    times
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (a.0 as f64 - target)
                .abs()
                .total_cmp(&(b.0 as f64 - target).abs())
        })
        .map(|(i, _)| i)
}
fn sample_clock(timestamp: u64) -> String {
    let Ok(timestamp) = libc::time_t::try_from(timestamp) else {
        return timestamp.to_string();
    };
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&timestamp, &mut local) }.is_null() {
        return timestamp.to_string();
    }
    format!(
        "{:02}:{:02}:{:02}",
        local.tm_hour, local.tm_min, local.tm_sec
    )
}
pub fn time_span(seconds: u64) -> String {
    if seconds >= 3600 && seconds.is_multiple_of(3600) {
        format!("{}h", seconds / 3600)
    } else if seconds >= 60 && seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
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

    fn hover_labels(times: &[(u64, bool)], fraction: f32) -> String {
        chart_labels(
            &[
                ChartSeries::percent("CPU", &[25.0, 90.0], GREEN),
                ChartSeries::percent("Memory", &[45.0, 80.0], BLUE),
            ],
            times,
            fraction,
            times.last().unwrap().0 - times.first().unwrap().0,
        )
    }

    fn chart_labels(
        series: &[ChartSeries<'_>],
        times: &[(u64, bool)],
        fraction: f32,
        seconds: u64,
    ) -> String {
        let ctx = egui::Context::default();
        ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
        let mut labels = String::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 300.0),
                    )),
                    time: Some(frame as f64),
                    events: vec![egui::Event::PointerMoved(egui::pos2(
                        8.0 + 384.0 * fraction,
                        40.0,
                    ))],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        chart(ui, series, times, 80.0, 100.0, seconds);
                    });
                },
            );
            for shape in output.shapes {
                if let egui::epaint::Shape::Text(text) = shape.shape {
                    labels.push_str(text.galley.text());
                    labels.push('\n');
                }
            }
        }
        labels
    }

    #[test]
    fn hover_identifies_metrics_and_units() {
        let labels = hover_labels(&[(100, false), (200, false)], 0.0);
        assert!(labels.contains("CPU: 25.0%"), "{labels}");
        assert!(labels.contains("Memory: 45.0%"), "{labels}");
    }

    #[test]
    fn hover_does_not_invent_values_inside_sampling_gap() {
        let labels = hover_labels(&[(100, false), (200, true)], 0.5);
        assert!(labels.contains("No sample at this time"), "{labels}");
        assert!(!labels.contains("25.0"), "{labels}");
    }

    #[test]
    fn hover_chooses_nearest_sample_without_truncating_time() {
        let labels = hover_labels(&[(100, false), (101, false)], 0.9);
        assert!(labels.contains("CPU: 90.0%"), "{labels}");
    }

    #[test]
    fn singleton_sample_can_be_hovered_inside_its_visible_marker() {
        let labels = chart_labels(
            &[ChartSeries::percent("CPU", &[50.0], GREEN)],
            &[(100, false)],
            382.0 / 384.0,
            1,
        );
        assert!(labels.contains("CPU: 50.0%"), "{labels}");
    }

    #[test]
    fn gap_endpoint_can_be_hovered_inside_its_visible_marker() {
        let labels = hover_labels(&[(100, false), (200, true)], 382.0 / 384.0);
        assert!(labels.contains("CPU: 90.0%"), "{labels}");
    }

    #[test]
    fn network_hover_formats_each_direction_as_a_rate() {
        let labels = chart_labels(
            &[
                ChartSeries::rate("Download", &[2048.0, 4096.0], PEACH),
                ChartSeries::rate("Upload", &[512.0, 1024.0], PURPLE),
            ],
            &[(100, false), (200, false)],
            0.0,
            100,
        );
        assert!(labels.contains("Download: 2 KiB/s"), "{labels}");
        assert!(labels.contains("Upload: 512 B/s"), "{labels}");
    }

    #[test]
    fn unrecorded_part_of_selected_window_has_no_hover_values() {
        let labels = chart_labels(
            &[ChartSeries::percent("CPU", &[25.0, 90.0], GREEN)],
            &[(190, false), (200, false)],
            0.5,
            100,
        );
        assert!(labels.contains("No sample at this time"), "{labels}");
        assert!(!labels.contains("CPU:"), "{labels}");
    }
    #[test]
    fn chart_positions_preserve_elapsed_time() {
        assert_eq!(time_fraction(100, 100, 200), 0.0);
        assert_eq!(time_fraction(110, 100, 200), 0.1);
        assert_eq!(time_fraction(200, 100, 200), 1.0);
    }
}
