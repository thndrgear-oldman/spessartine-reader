use eframe::egui;
use super::{ease_out_cubic, paint_polygon};
use crate::fx::soundmap::FxCue;

pub fn draw(painter: &egui::Painter, page_rect: egui::Rect, c: &FxCue, ct: f32, t: f32) -> bool {
    let dur = c.dur.unwrap_or(1.2).max(0.1);
    let ph = if c.oneshot.unwrap_or(false) {
        if !ct.is_finite() || ct < 0.0 {
            return false;
        }
        (ct / dur).clamp(0.0, 1.0)
    } else {
        (t / dur).fract()
    };
    let a = 0.85 * (1.0 - ease_out_cubic(ph)).powi(2);
    match &c.area {
        Some(poly) if poly.len() >= 3 => {
            let pts: Vec<egui::Pos2> = poly.iter()
                .map(|p| egui::pos2(
                    page_rect.left() + p[0] * page_rect.width(),
                    page_rect.top() + p[1] * page_rect.height(),
                ))
                .collect();
            paint_polygon(painter, &pts, egui::Color32::WHITE.gamma_multiply(a));
        }
        _ => {
            painter.rect_filled(
                page_rect, 0.0,
                egui::Color32::WHITE.gamma_multiply(a),
            );
        }
    }
    true
}
