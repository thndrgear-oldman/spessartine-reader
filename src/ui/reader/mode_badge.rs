use eframe::egui;
use crate::reader::mode::ReaderMode;

pub fn draw(
    painter: &egui::Painter,
    rect: egui::Rect,
    mode: ReaderMode,
    is_manual: bool,
) {
    let icon = match mode {
        ReaderMode::Manga => "󰧮",
        ReaderMode::Double => "󰗚",
        ReaderMode::Webtoon => "",
    };
    let color = if is_manual {
        egui::Color32::from_rgb(0x4a, 0x9e, 0xff)
    } else {
        egui::Color32::GRAY
    };

    let badge_size = 22.0;
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - badge_size - 4.0, rect.top() + 4.0),
        egui::vec2(badge_size, badge_size),
    );

    painter.rect_filled(badge_rect, badge_size / 2.0, egui::Color32::from_black_alpha(120));
    painter.text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(12.0),
        color,
    );
}
