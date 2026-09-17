use eframe::egui;
use crate::ui::button;
use crate::lang::Lang;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            ui.add_space(40.0);
            ui.heading(app.lang.heading_language);
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let ru_fg = if app.lang.is_ru() { app.theme.toolbar_btn_fg } else { app.theme.cell_fg };
                if button::tag_btn(ui, "Русский", ru_fg, &app.theme, 14.0).clicked() {
                    app.lang = Lang::ru();
                    app.settings.lang_code = "ru".into();
                    app.settings.save();
                }
                let en_fg = if app.lang.is_en() { app.theme.toolbar_btn_fg } else { app.theme.cell_fg };
                if button::tag_btn(ui, "English", en_fg, &app.theme, 14.0).clicked() {
                    app.lang = Lang::en();
                    app.settings.lang_code = "en".into();
                    app.settings.save();
                }
            });
    });
}
