use egui::{RichText, Ui};
use crate::process::types::{Category, SortColumn, SortDirection};
use crate::ui::theme::Theme;

pub struct HeaderBar;

impl HeaderBar {
    pub fn show(
        ui: &mut Ui,
        active_category: &mut Option<Category>,
        filter_query: &mut String,
        sort_column: &mut SortColumn,
        sort_direction: &mut SortDirection,
        refresh_interval: &mut f32,
        is_paused: &mut bool,
        on_refresh: impl FnOnce(),
    ) {
        // ── Title Row ──
        ui.horizontal(|ui| {
            ui.label(RichText::new("RAM RADAR").size(18.0).color(Theme::ACCENT_BLUE).strong());
            ui.label(RichText::new("·").size(18.0).color(Theme::TEXT_MUTED));
            ui.label(RichText::new("Linux PSS Memory Inspector").size(13.0).color(Theme::TEXT_MUTED));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("↻ Scan").color(Theme::TEXT_PRIMARY)).clicked() {
                    on_refresh();
                }

                let pause_txt = if *is_paused { "▶ Resume" } else { "⏸ Pause" };
                let pause_col = if *is_paused { Theme::ACCENT_ORANGE } else { Theme::TEXT_SECONDARY };
                if ui.button(RichText::new(pause_txt).color(pause_col)).clicked() {
                    *is_paused = !*is_paused;
                }

                // Live dot
                let dot_color = if *is_paused { Theme::ACCENT_ORANGE } else { Theme::ACCENT_GREEN };
                ui.label(RichText::new("●").size(10.0).color(dot_color));

                egui::ComboBox::from_id_salt("refresh_rate")
                    .selected_text(format!("{:.1}s", *refresh_interval))
                    .width(55.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(refresh_interval, 0.5, "0.5s");
                        ui.selectable_value(refresh_interval, 1.0, "1.0s");
                        ui.selectable_value(refresh_interval, 2.0, "2.0s");
                        ui.selectable_value(refresh_interval, 5.0, "5.0s");
                    });
            });
        });

        ui.add_space(4.0);

        // ── Category Tabs + Search + Sort ──
        ui.horizontal(|ui| {
            let cats: &[(&str, Option<Category>)] = &[
                ("⊞ All", None),
                ("🌐 Apps", Some(Category::Apps)),
                ("🪟 Desktop", Some(Category::Desktop)),
                ("💻 Dev & CLI", Some(Category::Development)),
                ("⚙️ Services", Some(Category::Background)),
                ("🛡️ System", Some(Category::System)),
            ];
            for &(label, cat) in cats {
                let active = *active_category == cat;
                if ui.selectable_label(active, label).clicked() {
                    *active_category = cat;
                }
            }

            ui.separator();

            ui.label(RichText::new("🔍").size(12.0));
            ui.add(
                egui::TextEdit::singleline(filter_query)
                    .hint_text("Filter...")
                    .desired_width(160.0),
            );
            if !filter_query.is_empty() && ui.small_button("✕").clicked() {
                filter_query.clear();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dir_txt = if *sort_direction == SortDirection::Descending { "↓" } else { "↑" };
                if ui.button(RichText::new(dir_txt).color(Theme::TEXT_SECONDARY)).clicked() {
                    *sort_direction = match sort_direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                }

                egui::ComboBox::from_id_salt("sort_col")
                    .selected_text(match *sort_column {
                        SortColumn::PssRealisticRam => "PSS",
                        SortColumn::RssStandardRam => "RSS",
                        SortColumn::UssPrivateRam => "USS",
                        SortColumn::Cpu => "CPU",
                        SortColumn::ProcessCount => "Procs",
                        SortColumn::Name => "Name",
                    })
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(sort_column, SortColumn::PssRealisticRam, "PSS (Real)");
                        ui.selectable_value(sort_column, SortColumn::RssStandardRam, "RSS");
                        ui.selectable_value(sort_column, SortColumn::UssPrivateRam, "USS");
                        ui.selectable_value(sort_column, SortColumn::Cpu, "CPU %");
                        ui.selectable_value(sort_column, SortColumn::ProcessCount, "Processes");
                        ui.selectable_value(sort_column, SortColumn::Name, "Name");
                    });
                ui.label(RichText::new("Sort:").size(11.0).color(Theme::TEXT_MUTED));
            });
        });
    }
}
