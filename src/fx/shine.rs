use eframe::egui;
use super::point_in_poly;
use crate::fx::soundmap::FxCue;

pub fn draw(painter: &egui::Painter, page: egui::Rect, c: &FxCue, t: f32) {
    let at = c.at.unwrap_or(0.0);
    let dur = c.dur.unwrap_or(1.2).max(0.05);
    let q = if c.oneshot.unwrap_or(false) {
        ((t - at) / dur).clamp(0.0, 1.0)
    } else {
        ((t - at) / dur).rem_euclid(1.0)
    };

    let rad = c.angle.unwrap_or(30.0).to_radians();
    let d = egui::vec2(rad.cos(), -rad.sin());

    let intensity = c.intensity.unwrap_or(0.8).clamp(0.0, 1.0);
    let width_frac = c.hz.unwrap_or(0.12).clamp(0.02, 1.0); // ширина, доля диагонали

    let diag = (page.width() * page.width() + page.height() * page.height()).sqrt();
    let hw = width_frac * diag * 0.5;
    let travel = diag * 0.5 + hw;

    let band_c = (q * 2.0 - 1.0) * travel;
    let env = (((q * 6.0).min(1.0)) * (((1.0 - q) * 6.0).min(1.0))).clamp(0.0, 1.0);

    match &c.area {
        Some(poly) if poly.len() >= 3 => {
            const K: usize = 48;
            let mut mnx = f32::MAX;
            let mut mn_y = f32::MAX;
            let mut mxx = f32::MIN;
            let mut mx_y = f32::MIN;
            for p in poly {
                mnx = mnx.min(p[0]);
                mxx = mxx.max(p[0]);
                mn_y = mn_y.min(p[1]);
                mx_y = mx_y.max(p[1]);
            }
            let brect = egui::Rect::from_min_max(
                page.lerp_inside(egui::vec2(mnx, mn_y)),
                page.lerp_inside(egui::vec2(mxx, mx_y)),
            );
            let cw = brect.width() / K as f32;
            let chh = brect.height() / K as f32;

            let mut mesh = egui::Mesh::default();
            for iy in 0..K {
                for ix in 0..K {
                    let nx = mnx + (ix as f32 + 0.5) / K as f32 * (mxx - mnx);
                    let ny = mn_y + (iy as f32 + 0.5) / K as f32 * (mx_y - mn_y);
                    if !point_in_poly(nx, ny, poly) { continue; }

                    let px = page.left() + nx * page.width();
                    let py = page.top() + ny * page.height();
                    let rel = egui::vec2(px - page.center().x, py - page.center().y);
                    let dist = rel.dot(d);
                    let off_band = (dist - band_c).abs();
                    if off_band > hw { continue; }
                    let edge = 1.0 - (off_band / hw).powi(2);
                    let a = intensity * env * (0.15 + 0.85 * edge);

                    let i0 = mesh.vertices.len() as u32;
                    for (dx, dy) in [
                        (-0.5f32, -0.5f32),
                        (0.5, -0.5),
                        (0.5, 0.5),
                        (-0.5, 0.5),
                    ] {
                        mesh.colored_vertex(
                            egui::pos2(px + dx * cw, py + dy * chh),
                            egui::Color32::WHITE.gamma_multiply(a),
                        );
                    }
                    mesh.add_triangle(i0, i0 + 1, i0 + 2);
                    mesh.add_triangle(i0, i0 + 2, i0 + 3);
                }
            }
            painter.add(egui::Shape::Mesh(mesh.into()));
        }
        _ => {
            let perp = egui::vec2(-d.y, d.x);
            let center = page.center() + d * band_c;
            let hp = diag * 0.75 + hw;
            let corners = [
                center + d * hw + perp * hp,
                center - d * hw + perp * hp,
                center - d * hw - perp * hp,
                center + d * hw - perp * hp,
            ];
            let a = intensity * env;
            let mut mesh = egui::Mesh::default();
            for corner in corners {
                mesh.colored_vertex(corner, egui::Color32::WHITE.gamma_multiply(a));
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(egui::Shape::Mesh(mesh.into()));
        }
    }
}
