use eframe::egui;

pub mod library;
pub mod recent;
pub mod language;
pub mod theme;
pub mod settings;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    match app.current_view {
        crate::AppView::Library  => library::draw(app, ui),
        crate::AppView::Recent   => recent::draw(app, ui),
        crate::AppView::Language => language::draw(app, ui),
        crate::AppView::Theme    => theme::draw(app, ui),
        crate::AppView::Settings => settings::draw(app, ui),
    }
}
