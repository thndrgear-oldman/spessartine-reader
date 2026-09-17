use eframe::egui;
use crate::theme::AppTheme;

pub fn icon_btn(ui: &mut egui::Ui, icon: &str, theme: &AppTheme) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        theme.toolbar_btn_press_bg
    } else if resp.hovered() {
        theme.toolbar_btn_hover_bg
    } else {
        theme.toolbar_btn_bg
    };
    ui.painter_at(rect).rect_filled(rect, 8, bg);
    ui.painter_at(rect).text(
        rect.center(), egui::Align2::CENTER_CENTER, icon,
        egui::FontId::proportional(16.0), theme.toolbar_btn_fg,
    );
    resp
}

pub fn text_btn(
    ui: &mut egui::Ui,
    text: &str,
    fg: egui::Color32,
    theme: &AppTheme,
) -> egui::Response {
    text_btn_max(ui, text, fg, theme, f32::INFINITY)
}

pub fn text_btn_max(
    ui: &mut egui::Ui,
    text: &str,
    fg: egui::Color32,
    theme: &AppTheme,
    max_width: f32,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(14.0), fg);
    let text_w = (galley.size().x + 12.0).min(max_width);
    let size = egui::vec2(text_w, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        theme.btn_press_bg
    } else if resp.hovered() {
        theme.btn_hover_bg
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter_at(rect).rect_filled(rect, 6, bg);
    let max_text_w = (rect.width() - 12.0).max(1.0);
    if galley.size().x > max_text_w {
        let ellipsis = "\u{2026}";
        let mut truncated = text.to_string();
        loop {
            let g = ui.painter().layout_no_wrap(truncated.clone() + ellipsis, egui::FontId::proportional(14.0), fg);
            if g.size().x <= max_text_w || truncated.is_empty() {
                break;
            }
            truncated.pop();
        }
        let display = truncated + ellipsis;
        ui.painter_at(rect).text(
            egui::pos2(rect.left() + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &display,
            egui::FontId::proportional(14.0),
            fg,
        );
    } else {
        ui.painter_at(rect).text(
            egui::pos2(rect.left() + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(14.0),
            fg,
        );
    }
    resp
}

pub fn nav_btn(ui: &mut egui::Ui, text: &str, selected: bool, theme: &AppTheme) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 40.0),
        egui::Sense::click(),
    );
    let bg = if resp.is_pointer_button_down_on() {
        theme.lp_btn_bg
    } else if resp.hovered() {
        theme.lp_btn_hover_bg
    } else if selected {
        theme.lp_btn_active_bg
    } else {
        theme.lp_btn_bg
    };
    let margin = 10.0;
    let btn_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + margin, rect.top()),
        egui::vec2(rect.width() - margin * 2.0, rect.height()),
    );
    let fg = if resp.hovered() {
        egui::Color32::from_rgb(0xad, 0xb1, 0xb8)
    } else {
        egui::Color32::from_rgb(0xcf, 0xd2, 0xd8)
    };
    ui.painter().rect_filled(btn_rect, 6.0, bg);
    ui.painter().text(
        egui::pos2(btn_rect.left() + 15.0, btn_rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(16.0),
        fg,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub fn tag_btn(ui: &mut egui::Ui, text: &str, fg: egui::Color32, theme: &AppTheme, font_size: f32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(font_size),
        fg,
    );
    let w = galley.size().x + 20.0;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(w, 24.0),
        egui::Sense::click(),
    );
    let bg = if resp.is_pointer_button_down_on() {
        theme.btn_press_bg
    } else if resp.hovered() {
        theme.btn_hover_bg
    } else {
        theme.lp_btn_bg
    };
    ui.painter_at(rect).rect_filled(rect, 6, bg);
    ui.painter_at(rect).text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(font_size),
        fg,
    );
    resp
}
