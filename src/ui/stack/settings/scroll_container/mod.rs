use eframe::egui;

pub mod scroll_content;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.add_space(20.0);
                scroll_content::draw(app, ui);
            });
        });
}
