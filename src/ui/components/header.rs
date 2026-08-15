use egui::{Color32, RichText, Ui};
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
        // 1. Top Title & Live Status
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🎯 RamRadar")
                    .size(22.0)
                    .color(Theme::ACCENT_CYAN)
                    .strong(),
            );

            // Live status badge
            ui.label(
                RichText::new(" 🟢 LIVE 60 FPS ")
                    .size(10.5)
                    .strong()
                    .color(Color32::from_rgb(16, 185, 129))
                    .background_color(Color32::from_rgb(10, 40, 25)),
            );

            ui.label(
                RichText::new("• Arch Linux / Hyprland Memory Inspector")
                    .size(12.0)
                    .color(Theme::TEXT_MUTED),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Refresh Button
                if ui.button(RichText::new("🔄 Odśwież").color(Theme::TEXT_PRIMARY).strong()).clicked() {
                    on_refresh();
                }

                // Pause / Resume
                let pause_text = if *is_paused { "▶ Wznów" } else { "⏸ Wstrzymaj" };
                if ui.button(
                    RichText::new(pause_text).color(if *is_paused { Theme::ACCENT_AMBER } else { Theme::TEXT_SECONDARY }),
                ).clicked() {
                    *is_paused = !*is_paused;
                }

                // Refresh Rate ComboBox
                ui.add_space(4.0);
                egui::ComboBox::from_id_salt("hdr_refresh_rate")
                    .selected_text(format!("Interwał: {:.1}s", *refresh_interval))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(refresh_interval, 0.5, "0.5s (Ultraszybki)");
                        ui.selectable_value(refresh_interval, 1.0, "1.0s (Płynny)");
                        ui.selectable_value(refresh_interval, 2.0, "2.0s (Oszczędny)");
                        ui.selectable_value(refresh_interval, 5.0, "5.0s (Rzadki)");
                    });
            });
        });

        ui.add_space(8.0);

        // 2. Navigation Pills & Filter / Sort Controls
        ui.horizontal(|ui| {
            // Category Buttons
            let all_active = active_category.is_none();
            if ui.selectable_label(all_active, "⚡ Wszystkie").clicked() {
                *active_category = None;
            }
            if ui.selectable_label(*active_category == Some(Category::Apps), "🌐 Aplikacje").clicked() {
                *active_category = Some(Category::Apps);
            }
            if ui.selectable_label(*active_category == Some(Category::Desktop), "🪟 Hyprland & Desktop").clicked() {
                *active_category = Some(Category::Desktop);
            }
            if ui.selectable_label(*active_category == Some(Category::Development), "💻 Dev & Narzędzia").clicked() {
                *active_category = Some(Category::Development);
            }
            if ui.selectable_label(*active_category == Some(Category::System), "🛡️ System").clicked() {
                *active_category = Some(Category::System);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Search Box
            ui.label(RichText::new("🔍").size(13.0));
            ui.add(
                egui::TextEdit::singleline(filter_query)
                    .hint_text("Filtruj aplikację, PID, rolę...")
                    .desired_width(220.0),
            );
            if !filter_query.is_empty() && ui.small_button("✖").clicked() {
                filter_query.clear();
            }

            // Right side: Sort Controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dir_str = if *sort_direction == SortDirection::Descending { "↓ Malejąco" } else { "↑ Rosnąco" };
                if ui.button(RichText::new(dir_str).color(Theme::TEXT_SECONDARY)).clicked() {
                    *sort_direction = match *sort_direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                }

                egui::ComboBox::from_id_salt("hdr_sort_column")
                    .selected_text(match *sort_column {
                        SortColumn::PssRealisticRam => "Sortuj: Realny RAM (PSS)",
                        SortColumn::RssStandardRam => "Sortuj: Tradycyjny RSS",
                        SortColumn::UssPrivateRam => "Sortuj: Prywatny (USS)",
                        SortColumn::Cpu => "Sortuj: Zużycie CPU",
                        SortColumn::ProcessCount => "Sortuj: Liczba procesów",
                        SortColumn::Name => "Sortuj: Alfabetycznie",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(sort_column, SortColumn::PssRealisticRam, "Realny RAM (PSS)");
                        ui.selectable_value(sort_column, SortColumn::RssStandardRam, "Tradycyjny RSS");
                        ui.selectable_value(sort_column, SortColumn::UssPrivateRam, "Prywatny RAM (USS)");
                        ui.selectable_value(sort_column, SortColumn::Cpu, "Zużycie CPU");
                        ui.selectable_value(sort_column, SortColumn::ProcessCount, "Liczba procesów");
                        ui.selectable_value(sort_column, SortColumn::Name, "Nazwa");
                    });
            });
        });
    }
}
