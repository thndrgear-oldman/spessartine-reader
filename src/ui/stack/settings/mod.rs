use eframe::egui;

pub mod scroll_container;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    scroll_container::draw(app, ui);
}
