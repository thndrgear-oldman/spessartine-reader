use eframe::egui;
use crate::ui::button;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            ui.add_space(40.0);
            ui.heading(app.lang.heading_theme);
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let is_dark  = app.theme.toolbar_bg == egui::Color32::from_rgb(0x0c, 0x0c, 0x0c);
                let is_flex  = app.theme.toolbar_bg == egui::Color32::from_rgb(0x10, 0x0F, 0x0F);
                if button::tag_btn(ui, "Dark",    if is_dark { app.theme.toolbar_btn_fg } else { app.theme.cell_fg }, &app.theme, 14.0).clicked() {
                    app.theme = crate::theme::AppTheme::dark();
                    app.theme_applied = false;
                    crate::config::save_theme("Dark");
                }
                if button::tag_btn(ui, "Flexoki", if is_flex { app.theme.toolbar_btn_fg } else { app.theme.cell_fg }, &app.theme, 14.0).clicked() {
                    app.theme = crate::theme::AppTheme::flexoki();
                    app.theme_applied = false;
                    crate::config::save_theme("Flexoki");
                }
            });
    });
}
