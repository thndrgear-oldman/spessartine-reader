use eframe::egui;

pub(crate) fn curtain_fill(
    painter: &egui::Painter,
    tex: egui::TextureId,
    rect: egui::Rect,
    fill: &[Vec<[f32; 2]>],
    holes: &[Vec<[f32; 2]>],
    tint: egui::Color32,
    alpha: f32,
) {
    if alpha <= 0.0 {
        return;
    }
    const STEP: f32 = 1.0;
    const OVERLAP: f32 = 1.0;

    let mut ymin = f32::INFINITY;
    let mut ymax = f32::NEG_INFINITY;
    if fill.is_empty() {
        ymin = rect.min.y;
        ymax = rect.max.y;
    } else {
        for poly in fill {
            for q in poly {
                let y = rect.min.y + q[1] * rect.height();
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
    }
    ymin = ymin.max(rect.min.y);
    ymax = ymax.min(rect.max.y);
    if !ymax.is_finite() || ymax <= ymin {
        return;
    }

    let a8 = (255.0 * alpha).clamp(0.0, 255.0) as u8;
    let tint = egui::Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), a8);

    let mut y = ymin;
    while y < ymax {
        let y0 = y;
        let y1 = (y + STEP + OVERLAP).min(ymax);
        let yc = (y0 + y1) * 0.5;

        let mut src: Vec<(f32, f32)> = Vec::new();
        if fill.is_empty() {
            src.push((rect.min.x, rect.max.x));
        } else {
            for poly in fill {
                let mut xs: Vec<f32> = Vec::new();
                for i in 0..poly.len() {
                    let a = poly[i];
                    let b = poly[(i + 1) % poly.len()];
                    let (ay, by) = (
                        rect.min.y + a[1] * rect.height(),
                        rect.min.y + b[1] * rect.height(),
                    );
                    if (ay <= yc && by > yc) || (by <= yc && ay > yc) {
                        let t = (yc - ay) / (by - ay);
                        xs.push(rect.min.x + a[0] * rect.width() + t * ((b[0] - a[0]) * rect.width()));
                    }
                }
                xs.sort_by(|p, q| p.partial_cmp(q).unwrap());
                let mut k = 0;
                while k + 1 < xs.len() {
                    src.push((xs[k], xs[k + 1]));
                    k += 2;
                }
            }
        }
        let src = merge_spans(&src);

        let mut hole: Vec<(f32, f32)> = Vec::new();
        for poly in holes {
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..poly.len() {
                let a = poly[i];
                let b = poly[(i + 1) % poly.len()];
                let (ay, by) = (
                    rect.min.y + a[1] * rect.height(),
                    rect.min.y + b[1] * rect.height(),
                );
                if (ay <= yc && by > yc) || (by <= yc && ay > yc) {
                    let t = (yc - ay) / (by - ay);
                    xs.push(rect.min.x + a[0] * rect.width() + t * ((b[0] - a[0]) * rect.width()));
                }
            }
            xs.sort_by(|p, q| p.partial_cmp(q).unwrap());
            let mut k = 0;
            while k + 1 < xs.len() {
                hole.push((xs[k], xs[k + 1]));
                k += 2;
            }
        }
        let hole = merge_spans(&hole);

        for (s, e) in subtract_spans(&src, &hole) {
            let s = s.max(rect.min.x);
            let e = e.min(rect.max.x);
            if e <= s {
                continue;
            }
            let uv = egui::Rect::from_min_max(
                egui::pos2((s - rect.min.x) / rect.width(), (y0 - rect.min.y) / rect.height()),
                egui::pos2((e - rect.min.x) / rect.width(), (y1 - rect.min.y) / rect.height()),
            );
            painter.image(
                tex,
                egui::Rect::from_min_max(egui::pos2(s, y0), egui::pos2(e, y1)),
                uv,
                tint,
            );
        }

        y += STEP;
    }
}

fn merge_spans(s: &[(f32, f32)]) -> Vec<(f32, f32)> {
    if s.is_empty() {
        return Vec::new();
    }
    let mut v: Vec<(f32, f32)> = s.to_vec();
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut out: Vec<(f32, f32)> = Vec::new();
    for (st, en) in v {
        if let Some(last) = out.last_mut() {
            if st <= last.1 {
                last.1 = last.1.max(en);
            } else {
                out.push((st, en));
            }
        } else {
            out.push((st, en));
        }
    }
    out
}

fn subtract_spans(a: &[(f32, f32)], b: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = Vec::new();
    let mut bi = 0;
    for &(mut s, e) in a {
        while bi < b.len() && b[bi].1 <= s {
            bi += 1;
        }
        while bi < b.len() && b[bi].0 < e {
            let (hs, he) = b[bi];
            if hs > s {
                out.push((s, hs));
            }
            s = he.max(s);
            if s >= e {
                break;
            }
            bi += 1;
        }
        if s < e {
            out.push((s, e));
        }
    }
    out
}
