use eframe::egui;
use super::{point_in_poly, FxState, Rng};
use crate::fx::soundmap::FxCue;

const MAX_DROPS: usize = 220;

pub(crate) struct Drop {
    x: f32,
    y: f32,
    len: f32,
    spd: f32,
}

pub fn draw(state: &mut FxState, painter: &egui::Painter, page: egui::Rect, c: &FxCue, dt: f32) {
    let area_filter = c.area.as_ref().filter(|a| a.len() >= 3);
    let intensity = c.intensity.unwrap_or(0.6).clamp(0.05, 1.0);
    let seed = c.seed.unwrap_or(42);
    let zone = c.zone.unwrap_or_default();
    let top_y = zone.y.clamp(0.0, 0.95);

    if !state.seeded {
        state.seeded = true;
        let mut rng = Rng::new(seed);
        let n = (30.0 + intensity * (MAX_DROPS as f32 - 30.0)) as usize;
        for _ in 0..n {
            state.drops.push(Drop {
                x: rng.f32(),
                y: -rng.f32(),
                len: 0.015 + rng.f32() * 0.03,
                spd: 0.9 + rng.f32() * 0.9,
            });
        }
    }

    let a = 0.10 + intensity * 0.25;
    let stroke = egui::Stroke::new(1.2, egui::Color32::WHITE.gamma_multiply(a));

    for d in &mut state.drops {
        d.y += d.spd * dt * (0.8 + intensity * 0.6);
        if d.y > 1.2 {
            d.y -= 1.4;
        }
        let x0 = page.left() + page.width() * (zone.x + d.x * zone.w.max(0.01)).min(zone.x + zone.w.max(0.01));
        let y0 = page.top() + page.height() * (top_y + (d.y - top_y).max(0.0));
        if let Some(poly) = area_filter {
            let nx = (x0 - page.left()) / page.width();
            let ny = (y0 - page.top()) / page.height();
            if !point_in_poly(nx, ny, poly) { continue; }
        }
        let x1 = x0 + d.len * page.width() * 0.06;
        let y1 = y0 + d.len * page.height();
        painter.line_segment([egui::pos2(x0, y0), egui::pos2(x1, y1)], stroke);
    }
}
