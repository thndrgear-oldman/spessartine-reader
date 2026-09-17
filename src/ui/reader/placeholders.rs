use eframe::egui;
use crate::lang::Lang;

pub fn end(ui: &mut egui::Ui, rect: egui::Rect, is_last: bool, lang: &Lang) {
    let label = if is_last { lang.placeholder_end } else { lang.placeholder_end_ch };
    draw(ui, rect, label);
}

pub fn panorama(ui: &mut egui::Ui, rect: egui::Rect, lang: &Lang) {
    draw(ui, rect, lang.placeholder_panorama);
}

fn draw(ui: &mut egui::Ui, rect: egui::Rect, label: &str) {
    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(190));
    let center = rect.center();
    let w = rect.width() * 0.5;

    ui.painter().line_segment(
        [egui::pos2(center.x - w * 0.4, center.y - 50.0),
         egui::pos2(center.x + w * 0.4, center.y - 50.0)],
        egui::Stroke::new(1.0, egui::Color32::BLACK),
    );
    ui.painter().text(
        egui::pos2(center.x, center.y),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(16.0),
        egui::Color32::BLACK,
    );
    ui.painter().line_segment(
        [egui::pos2(center.x - w * 0.4, center.y + 50.0),
         egui::pos2(center.x + w * 0.4, center.y + 50.0)],
        egui::Stroke::new(1.0, egui::Color32::BLACK),
    );
}
