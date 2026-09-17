use eframe::egui;
use super::Rng;
use crate::fx::soundmap::FxCue;

pub fn draw(painter: &egui::Painter, page: egui::Rect, c: &FxCue, t: f32) {
    let Some(area) = c.area.as_ref().filter(|p| p.len() >= 3) else {
        return;
    };
    let inten = c.intensity.unwrap_or(0.5).clamp(0.0, 1.0);
    if inten <= 0.0 {
        return;
    }
    let hz = c.hz.unwrap_or(0.6).max(0.01);
    let seed = c.seed.unwrap_or(42);
    let base_rot = c.angle.unwrap_or(0.0).to_radians();
    let sc = c.scale.unwrap_or(1.0).clamp(0.05, 5.0);
    let many = !c.single.unwrap_or(false);
    let pinned = c.pinned.unwrap_or(true);

    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in area {
        minx = minx.min(p[0]);
        miny = miny.min(p[1]);
        maxx = maxx.max(p[0]);
        maxy = maxy.max(p[1]);
    }
    if maxx <= minx || maxy <= miny {
        return;
    }

    let scale = page.width().min(page.height());

    let mut uvs: Vec<(f32, f32)> = Vec::new();
    if !many {
        uvs.push(((minx + maxx) * 0.5, (miny + maxy) * 0.5));
    } else {
        let n = (6.0 + inten * 24.0).round() as usize;
        // случайный разброс (rejection sampling по bbox)
        for i in 0..n {
            let mut rng =
                Rng::new(seed.wrapping_mul(0x9E37_79B9).rotate_left(7) ^ i as u32);
            for _ in 0..64 {
                let uu = minx + rng.f32() * (maxx - minx);
                let vv = miny + rng.f32() * (maxy - miny);
                if super::point_in_poly(uu, vv, area) {
                    uvs.push((uu, vv));
                    break;
                }
            }
        }
    }
    if uvs.is_empty() {
        return;
    }

    for (k, &(u, v)) in uvs.iter().enumerate() {
        let mut rng =
            Rng::new(seed.wrapping_mul(0xC2B2AE35).rotate_left(13) ^ k as u32);

        let len = if !many {
            scale * sc * 0.08
        } else {
            scale * sc * (0.035 + rng.f32() * 0.05)
        };
        let rot = base_rot + rng.f32() * std::f32::consts::FRAC_PI_2;
        let phase = rng.f32();
        let duty = 0.25 + rng.f32() * 0.25;
        let jit = 0.7 + rng.f32() * 1.3;

        let cyc = (t * hz * jit + phase).fract();
        let env = if cyc < duty {
            (std::f32::consts::PI * (cyc / duty)).sin().powf(1.5)
        } else {
            0.0
        };
        let br = env * (0.6 + 0.4 * inten);
        if br <= 0.004 {
            continue;
        }

        let (pu, pv) = if pinned {
            (u, v)
        } else {
            let cycle = ((t * hz * jit + phase).floor() as u64) & 0xFFFF;
            let mut r = Rng::new(
                seed.wrapping_mul(0x94D0_49BB).rotate_left(11) ^ k as u32
                    ^ (cycle as u32).wrapping_mul(0x51ED_270B),
            );
            (
                (u + (r.f32() * 2.0 - 1.0) * 0.1 * (maxx - minx)).clamp(minx, maxx),
                (v + (r.f32() * 2.0 - 1.0) * 0.1 * (maxy - miny)).clamp(miny, maxy),
            )
        };

        let pos = egui::pos2(
            page.left() + pu * page.width(),
            page.top() + pv * page.height(),
        );
        let alpha = (br * 255.0).round().min(255.0) as u8;
        star(
            painter,
            pos,
            len,
            len * 0.17,
            rot,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
    }
}

fn star(
    painter: &egui::Painter,
    pos: egui::Pos2,
    r_tip: f32,
    r_waist: f32,
    rot: f32,
    color: egui::Color32,
) {
    if r_tip <= 0.0 {
        return;
    }
    let seg = 28;
    let dir = |a: f32| egui::vec2(a.cos(), a.sin());
    let mut pts = Vec::with_capacity(4 * seg + 1);
    for k in 0..4 {
        let th = rot + k as f32 * std::f32::consts::FRAC_PI_2;
        let a = pos + dir(th) * r_tip;
        let b = pos + dir(th + std::f32::consts::FRAC_PI_2) * r_tip;
        let m = pos + dir(th + std::f32::consts::FRAC_PI_4) * r_waist;
        for j in 0..seg {
            let tt = j as f32 / seg as f32;
            let u = 1.0 - tt;
            // P = A·u³ + M·3tu + B·t³  (кубический Безье с P1=P2=M)
            let p = a + (m - a) * (3.0 * tt * u) + (b - a) * (tt * tt * tt);
            pts.push(p);
        }
    }
    pts.push(pos + dir(rot) * r_tip);

    let mut mesh = egui::Mesh::default();
    mesh.vertices.push(egui::epaint::Vertex {
        pos,
        uv: egui::pos2(0.0, 0.0),
        color,
    });
    for p in &pts {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *p,
            uv: egui::pos2(0.0, 0.0),
            color,
        });
    }
    for k in 1..pts.len() {
        mesh.add_triangle(0, k as u32, k as u32 + 1);
    }
    painter.add(egui::Shape::Mesh(mesh.into()));
}
