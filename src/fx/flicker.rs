use eframe::egui;
use super::paint_polygon;
use crate::fx::soundmap::{FxCue, Zone};

pub fn draw(painter: &egui::Painter, page_rect: egui::Rect, c: &FxCue, t: f32) {
    let hz = c.hz.unwrap_or(9.0).max(0.5);
    let base = c.intensity.unwrap_or(0.45).clamp(0.0, 1.0);
    let a = base * (0.55 + 0.45 * ((t * hz) * std::f32::consts::TAU).sin()) * 0.7;
    match &c.area {
        Some(poly) if poly.len() >= 3 => {
            let pts: Vec<egui::Pos2> = poly.iter()
                .map(|p| egui::pos2(
                    page_rect.left() + p[0] * page_rect.width(),
                    page_rect.top() + p[1] * page_rect.height(),
                ))
                .collect();
            paint_polygon(painter, &pts, egui::Color32::BLACK.gamma_multiply(a));
        }
        _ => {
            let r = zone_rect(c.zone.unwrap_or_default(), page_rect);
            painter.rect_filled(r, 0.0, egui::Color32::BLACK.gamma_multiply(a));
        }
    }
}

fn zone_rect(z: Zone, page: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            page.left() + page.width() * z.x,
            page.top() + page.height() * z.y,
        ),
        egui::vec2(page.width() * z.w.max(0.01), page.height() * z.h.max(0.01)),
    )
}
