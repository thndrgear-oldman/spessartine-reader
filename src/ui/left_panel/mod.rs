use eframe::egui;
use crate::ui::button;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    ui.add_space(20.0);

    let items = [
        (app.lang.nav_library, crate::AppView::Library),
        (app.lang.nav_recent, crate::AppView::Recent),
        (app.lang.nav_language, crate::AppView::Language),
        (app.lang.nav_theme, crate::AppView::Theme),
        (app.lang.nav_settings, crate::AppView::Settings),
    ];

    for (label, view) in items.iter() {
        let resp = button::nav_btn(ui, label, app.current_view == *view, &app.theme);
        if resp.clicked() {
            let prev = app.current_view;
            app.current_view = *view;
            if prev == crate::AppView::Settings && *view == crate::AppView::Library {
                app.trigger_rescan();
            }
        }

        ui.add_space(4.0);
    }
}
