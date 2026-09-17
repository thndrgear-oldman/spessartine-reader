use eframe::egui;
use std::time::Instant;
use crate::fx::soundmap::{Cut, FxCue};

pub mod soundmap;
pub mod resolve;
pub mod curtain;
pub mod fx_player;
pub use fx_player::FxPlayer;

mod flash;
mod flicker;
mod rain;
mod shake;
mod shine;
mod stars;
mod sway;
mod wave;
mod wind;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self((seed as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03)
    }

    pub fn f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) as f32
    }
}

pub(crate) fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

const PAGE_GRID_COLS: usize = 96;
const PAGE_GRID_ROWS: usize = 96;

type MeshCacheKey = u64;

struct MeshCacheEntry {
    base_pos: Vec<[f32; 2]>,
    wfields: Vec<Option<Vec<f32>>>,
    wind_ctx: Vec<Option<wind::WindCtx>>,
}

pub struct PageMeshCache {
    entries: std::collections::HashMap<MeshCacheKey, MeshCacheEntry>,
    lru: std::collections::VecDeque<MeshCacheKey>,
}

impl Default for PageMeshCache {
    fn default() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            lru: std::collections::VecDeque::new(),
        }
    }
}

type OneshotMap = std::collections::HashMap<(u32, usize), f32>;

pub struct FxState {
    started: Option<Instant>,
    last: Option<Instant>,
    drops: Vec<rain::Drop>,
    seeded: bool,
    pub oneshot_frame_at: OneshotMap,
    pub page_mesh_cache: PageMeshCache,
}

impl Default for FxState {
    fn default() -> Self {
        Self {
            started: Some(Instant::now()),
            last: None,
            drops: Vec::new(),
            seeded: false,
            oneshot_frame_at: OneshotMap::new(),
            page_mesh_cache: PageMeshCache::default(),
        }
    }
}

impl FxState {
    pub fn reset(&mut self) {
        self.started = Some(Instant::now());
        self.last = None;
        self.seeded = false;
        self.drops.clear();
        self.oneshot_frame_at.clear();
        self.page_mesh_cache.entries.clear();
        self.page_mesh_cache.lru.clear();
    }

    pub fn trigger_frame_oneshot(&mut self, key: u32, frame_idx: usize) {
        let start_t = self.elapsed_view();
        self.oneshot_frame_at.insert((key, frame_idx), start_t);
    }

    pub fn trigger_page_oneshot(&mut self, key: u32) {
        let start_t = self.elapsed_view();
        self.oneshot_frame_at.insert((key, usize::MAX), start_t);
    }

    pub fn elapsed_view(&self) -> f32 {
        self.started.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0)
    }
}

pub fn cue_time_frame(oneshots: &OneshotMap, key: u32, frame: Option<usize>, t: f32) -> f32 {
    match frame {
        Some(fi) => oneshots
            .get(&(key, fi))
            .map_or(f32::NEG_INFINITY, |at| t - *at),
        None => oneshots
            .get(&(key, usize::MAX))
            .map_or(t, |at| t - *at),
    }
}

pub fn cue_time_from(oneshots: &OneshotMap, key: u32, c: &FxCue, t: f32) -> f32 {
    if !c.oneshot.unwrap_or(false) {
        return t;
    }
    cue_time_frame(oneshots, key, c.frame, t)
}

pub fn draw(state: &mut FxState, ui: &egui::Ui, page_rect: egui::Rect, cues: &[FxCue], key: u32, t: f32) {
    let now = Instant::now();
    let dt = state.last.map(|l| now.duration_since(l).as_secs_f32().min(0.05)).unwrap_or(0.016);
    state.last = Some(now);

    let painter = ui.painter().with_clip_rect(page_rect);
    for c in cues {
        let ct = cue_time_from(&state.oneshot_frame_at, key, c, t);
        match c.kind.as_str() {
            "flash" => {
                if !flash::draw(&painter, page_rect, c, ct, t) {
                    return;
                }
            }
            "flicker" => flicker::draw(&painter, page_rect, c, t),
            "rain" => rain::draw(state, &painter, page_rect, c, dt),
            "shine" => shine::draw(&painter, page_rect, c, ct),
            "stars" => stars::draw(&painter, page_rect, c, t),
            _ => {}
        }
    }
}

pub fn page_deformed(cues: &[FxCue]) -> bool {
    cues.iter().any(|c| c.cut_idx.is_none()
        && matches!(
            c.kind.as_str(),
            "wave" | "wave2" | "wind" | "sway" | "shake"
        ))
}

fn zone_mask_edge(t: f32) -> f32 {
    let outside = (t - 0.5).abs() - 0.5;
    1.0 - (outside / 0.04).clamp(0.0, 1.0)
}

fn area_mask(c: &FxCue, u: f32, v: f32) -> f32 {
    match &c.area {
        Some(poly) if poly.len() >= 3 => {
            if point_in_poly(u, v, poly) { 1.0 } else { 0.0 }
        }
        _ => 1.0,
    }
}

pub fn compute_effect_displacement(
    c: &FxCue,
    u: f32,
    v: f32,
    t: f32,
    rect_w: f32,
    rect_h: f32,
) -> (f32, f32) {
    let z = c.zone.unwrap_or_default();
    if c.oneshot.unwrap_or(false) && !t.is_finite() { return (0.0, 0.0); }
    let t = if c.oneshot.unwrap_or(false) { t.max(0.0) } else { t };
    match c.kind.as_str() {
        "wave" | "wave2" => wave::displace(c, u, v, t, rect_w, rect_h),
        "shake" => {
            let (d, dd) = shake::displace(c, u, v, t, rect_w, rect_h);
            let m = area_mask(c, u, v);
            (d * m, dd * m)
        }
        "wind" => {
            let band = zone_mask_edge((v - z.y) / z.h.max(0.01));
            let hband = zone_mask_edge((u - z.x) / z.w.max(0.01));
            let ctx = wind::context(c, rect_w);
            let (sx, sy) = wind::displace(&ctx, c, u, v, t);
            (sx * band * hband, sy * band * hband)
        }
        "sway" => {
            let (sx, sy) = sway::sway_displace(c, u, v, t, rect_w);
            let hb = zone_mask_edge((u - z.x) / z.w.max(0.01));
            (sx * hb, sy * hb)
        }
        _ => (0.0, 0.0),
    }
}

pub fn build_cut_mirror_mesh(
    tex_id: egui::TextureId,
    page_rect: egui::Rect,
    cut: &Cut,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.texture_id = tex_id;

    let pts = &cut.poly;
    if pts.len() < 3 { return mesh; }

    let k = (cut.mesh_detail.max(4) as usize / 8).clamp(2, 12);
    let (grid_pts, grid_tris) = polygon_subdiv_tris(pts, k);
    if grid_tris.is_empty() { return mesh; }

    for tri_chunk in grid_tris.chunks(3) {
        let (a, b, c) = (tri_chunk[0], tri_chunk[1], tri_chunk[2]);
        let mut vi = [0u32; 3];
        for (n, &idx) in [a, b, c].iter().enumerate() {
            let (u, v) = (grid_pts[idx as usize][0], grid_pts[idx as usize][1]);
            let (ux, uy) = mirror_uv(u, v, pts);
            vi[n] = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(
                    page_rect.left() + u * page_rect.width(),
                    page_rect.top() + v * page_rect.height(),
                ),
                uv: egui::pos2(ux.clamp(0.0, 1.0), uy.clamp(0.0, 1.0)),
                color: egui::Color32::WHITE,
            });
        }
        mesh.add_triangle(vi[0], vi[1], vi[2]);
    }

    mesh
}

fn polygon_subdiv_tris(poly: &[[f32; 2]], k: usize) -> (Vec<[f32; 2]>, Vec<u32>) {
    if poly.len() < 3 {
        return (Vec::new(), Vec::new());
    }
    let k = k.max(1);

    let flat: Vec<f64> = poly
        .iter()
        .flat_map(|p| [p[0] as f64, p[1] as f64])
        .collect();
    let indices = match earcutr::earcut(&flat, &[], 2) {
        Ok(v) => v,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    if indices.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut verts: Vec<[f32; 2]> = Vec::new();
    let mut tris: Vec<u32> = Vec::new();
    let mut vmap: std::collections::HashMap<(i64, i64), u32> = std::collections::HashMap::new();

    let add = |verts: &mut Vec<[f32; 2]>, vmap: &mut std::collections::HashMap<(i64, i64), u32>, x: f32, y: f32| -> u32 {
        let key = ((x * 1e7).round() as i64, (y * 1e7).round() as i64);
        if let Some(&i) = vmap.get(&key) {
            return i;
        }
        let i = verts.len() as u32;
        verts.push([x, y]);
        vmap.insert(key, i);
        i
    };

    for chunk in indices.chunks(3) {
        let a = poly[chunk[0]];
        let b = poly[chunk[1]];
        let c = poly[chunk[2]];

        let mut cell = |i: usize, j: usize| -> u32 {
            let fi = i as f32 / k as f32;
            let fj = j as f32 / k as f32;
            let x = a[0] + (b[0] - a[0]) * fi + (c[0] - a[0]) * fj;
            let y = a[1] + (b[1] - a[1]) * fi + (c[1] - a[1]) * fj;
            add(&mut verts, &mut vmap, x, y)
        };

        for i in 0..k {
            for j in 0..(k - i) {
                let p00 = cell(i, j);
                let p10 = cell(i + 1, j);
                let p01 = cell(i, j + 1);
                tris.push(p00);
                tris.push(p10);
                tris.push(p01);
                if i + j + 2 <= k {
                    let p11 = cell(i + 1, j + 1);
                    tris.push(p10);
                    tris.push(p11);
                    tris.push(p01);
                }
            }
        }
    }

    (verts, tris)
}

fn mirror_uv(px: f32, py: f32, poly: &[[f32; 2]]) -> (f32, f32) {
    let n = poly.len();
    if n < 3 { return (px, py); }

    let (mut cx, mut cy) = (0.0_f32, 0.0_f32);
    for p in poly {
        cx += p[0];
        cy += p[1];
    }
    cx /= n as f32;
    cy /= n as f32;

    let (dx, dy) = (px - cx, py - cy);
    let d = (dx * dx + dy * dy).sqrt();
    let (ddx, ddy) = if d > 1e-5 {
        (dx / d, dy / d)
    } else {
        (1.0_f32, 0.0_f32)
    };

    let mut r = f32::INFINITY;
    for i in 0..n {
        let j = (i + 1) % n;
        let (ax, ay) = (poly[i][0], poly[i][1]);
        let (bx, by) = (poly[j][0], poly[j][1]);
        let ex = bx - ax;
        let ey = by - ay;
        let denom = ddx * ey - ddy * ex;
        if denom.abs() < 1e-9 { continue; }
        let wx = ax - cx;
        let wy = ay - cy;
        let t = (wx * ey - wy * ex) / denom;
        let s = (wx * ddy - wy * ddx) / denom;
        if t >= 0.0 && s >= -1e-4 && s <= 1.0 + 1e-4 {
            r = r.min(t);
        }
    }
    if !r.is_finite() { return (px, py); }

    const RING: f32 = 0.35;
    let t = (d / r).clamp(0.0, 1.0);
    let out_r = r * (1.0 + RING - RING * t);
    (cx + ddx * out_r, cy + ddy * out_r)
}

pub fn build_cut_mesh_deformed(
    tex_id: egui::TextureId,
    page_rect: egui::Rect,
    cut: &Cut,
    cues: &[FxCue],
    t: f32,
    key: u32,
    oneshots: &OneshotMap,
) -> egui::Mesh {
    let pts = &cut.poly;
    if pts.len() < 3 { return egui::Mesh::default(); }

    let mut mesh = egui::Mesh::default();
    mesh.texture_id = tex_id;

    let k = (cut.mesh_detail.max(8) as usize / 8).clamp(2, 12);
    let (grid_pts, grid_tris) = polygon_subdiv_tris(pts, k);
    if grid_tris.is_empty() { return mesh; }

    let n_ctl = cut.grid_points.len();
    let mut ctrl_disp: Vec<(f32, f32)> = Vec::with_capacity(n_ctl);
    if n_ctl > 0 {
        for gi in 0..n_ctl {
            if cut.grid_anchored.get(gi).copied().unwrap_or(false) {
                ctrl_disp.push((0.0, 0.0));
                continue;
            }
            let (gu, gv) = (cut.grid_points[gi][0], cut.grid_points[gi][1]);
            let mut dx: f32 = 0.0;
            let mut dy: f32 = 0.0;
            for c in cues {
                if !matches!(c.kind.as_str(), "wave" | "wave2" | "wind" | "sway" | "shake") { continue; }
                let (d, dd) = compute_effect_displacement(c, gu, gv, cue_time_from(oneshots, key, c, t), page_rect.width(), page_rect.height());
                dx += d;
                dy += dd;
            }
            ctrl_disp.push((dx, dy));
        }
    }

    let bpw = ((cut.bbox[2] / (cut.grid_cols.max(1) - 1).max(1) as f32)
        + (cut.bbox[3] / (cut.grid_rows.max(1) - 1).max(1) as f32)) * 0.5;
    let eps: f32 = (bpw * 0.5).max(0.001).powi(2);

    for tri_chunk in grid_tris.chunks(3) {
        let (a, b, c) = (tri_chunk[0], tri_chunk[1], tri_chunk[2]);
        let mut vi = [0u32; 3];
        for (n, &idx) in [a, b, c].iter().enumerate() {
            let u = grid_pts[idx as usize][0];
            let v = grid_pts[idx as usize][1];

            let mut total_dx: f32 = 0.0;
            let mut total_dy: f32 = 0.0;
            if n_ctl == 0 {
                for c in cues {
                    if !matches!(c.kind.as_str(), "wave" | "wave2" | "wind" | "sway" | "shake") { continue; }
                    let (d, dd) = compute_effect_displacement(c, u, v, cue_time_from(oneshots, key, c, t), page_rect.width(), page_rect.height());
                    total_dx += d;
                    total_dy += dd;
                }
            } else {
                let mut wsum: f32 = 0.0;
                for gi in 0..n_ctl {
                    let gp = cut.grid_points[gi];
                    let d2 = (u - gp[0]) * (u - gp[0]) + (v - gp[1]) * (v - gp[1]);
                    let w = 1.0 / (d2 + eps);
                    wsum += w;
                    total_dx += w * ctrl_disp[gi].0;
                    total_dy += w * ctrl_disp[gi].1;
                }
                if wsum > 1e-9 {
                    total_dx /= wsum;
                    total_dy /= wsum;
                } else {
                    total_dx = 0.0;
                    total_dy = 0.0;
                }
            }

            let new_u = u + total_dx / page_rect.width();
            let new_v = v + total_dy / page_rect.height();

            vi[n] = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(
                    page_rect.left() + new_u * page_rect.width(),
                    page_rect.top() + new_v * page_rect.height(),
                ),
                uv: egui::pos2(u, v),
                color: egui::Color32::WHITE,
            });
        }
        mesh.add_triangle(vi[0], vi[1], vi[2]);
    }

    mesh
}

fn page_mesh_key(cues: &[FxCue], rect: egui::Rect) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    rect.width().to_bits().hash(&mut h);
    rect.height().to_bits().hash(&mut h);
    for c in cues {
        c.kind.hash(&mut h);
        c.intensity.unwrap_or(0.0).to_bits().hash(&mut h);
        c.hz.unwrap_or(0.0).to_bits().hash(&mut h);
        c.angle.unwrap_or(0.0).to_bits().hash(&mut h);
        c.dur.unwrap_or(0.0).to_bits().hash(&mut h);
        c.bands.unwrap_or(0).hash(&mut h);
        c.band_width.unwrap_or(0.0).to_bits().hash(&mut h);
        c.wavelen.unwrap_or(0.0).to_bits().hash(&mut h);
        c.seed.unwrap_or(0).hash(&mut h);
        c.anchor.unwrap_or(0).hash(&mut h);
        c.axis.unwrap_or(0).hash(&mut h);
        c.oneshot.unwrap_or(false).hash(&mut h);
        match &c.zone {
            Some(z) => {
                z.x.to_bits().hash(&mut h);
                z.y.to_bits().hash(&mut h);
                z.w.to_bits().hash(&mut h);
                z.h.to_bits().hash(&mut h);
            }
            None => 0u8.hash(&mut h),
        }
        match &c.area {
            Some(a) => {
                a.len().hash(&mut h);
                for p in a {
                    p[0].to_bits().hash(&mut h);
                    p[1].to_bits().hash(&mut h);
                }
            }
            None => 0u8.hash(&mut h),
        }
    }
    h.finish()
}

pub fn build_page_mesh(
    tex_id: egui::TextureId,
    rect: egui::Rect,
    cues: &[FxCue],
    t: f32,
    key: u32,
    oneshots: &OneshotMap,
    debug: bool,
    cache: &mut PageMeshCache,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.texture_id = tex_id;
    let stride = (PAGE_GRID_COLS + 1) as u32;

    let mesh_key = page_mesh_key(cues, rect);
    if !cache.entries.contains_key(&mesh_key) {
        let mut entry = MeshCacheEntry {
            base_pos: Vec::new(),
            wfields: Vec::new(),
            wind_ctx: Vec::new(),
        };
        entry.base_pos.clear();
        for iy in 0..=PAGE_GRID_ROWS {
            let v = iy as f32 / PAGE_GRID_ROWS as f32;
            for ix in 0..=PAGE_GRID_COLS {
                let u = ix as f32 / PAGE_GRID_COLS as f32;
                entry.base_pos.push([u, v]);
            }
        }
        entry.wfields.clear();
        entry.wfields.resize(cues.len(), None);
        for (ci, c) in cues.iter().enumerate() {
            if !matches!(c.kind.as_str(), "wave" | "wave2" | "wind" | "sway" | "shake") { continue; }
            if let Some(poly) = &c.area {
                if poly.len() >= 3 {
                    entry.wfields[ci] = Some(pip_field(poly, PAGE_GRID_COLS, PAGE_GRID_ROWS));
                }
            }
        }
        entry.wind_ctx.clear();
        entry.wind_ctx.resize(cues.len(), None);
        for (ci, c) in cues.iter().enumerate() {
            if c.kind != "wind" { continue; }
            entry.wind_ctx[ci] = Some(wind::context(c, rect.width()));
        }
        cache.entries.insert(mesh_key, entry);
        cache.lru.push_back(mesh_key);
        while cache.lru.len() > 8 {
            if let Some(old) = cache.lru.pop_front() {
                cache.entries.remove(&old);
            }
        }
    }
    let entry = cache.entries.get(&mesh_key).expect("mesh cache entry");

    let vw = PAGE_GRID_COLS + 1;
    for (vi, bp) in entry.base_pos.iter().enumerate() {
        let (u, v) = (bp[0], bp[1]);
        let iy = vi / vw;
        let ix = vi - iy * vw;

        let mut dx = 0.0;
        let mut dy = 0.0;
        for (ci, c) in cues.iter().enumerate() {
            let z = c.zone.unwrap_or_default();
            let wq = entry.wfields[ci].as_ref().map_or(1.0, |f| f[iy * vw + ix]);
            let ct = cue_time_from(oneshots, key, c, t);
            if !ct.is_finite() { continue; }
            match c.kind.as_str() {
                "wave" | "wave2" => {
                    let (d, dd) = wave::displace(c, u, v, ct, rect.width(), rect.height());
                    dx += d * wq;
                    dy += dd * wq;
                }
                "shake" => {
                    let (d, dd) = shake::displace(c, u, v, ct, rect.width(), rect.height());
                    dx += d * wq;
                    dy += dd * wq;
                }
                "wind" => {
                    let band = zone_mask_edge((v - z.y) / z.h.max(0.01));
                    let hband = zone_mask_edge((u - z.x) / z.w.max(0.01));
                    if let Some(ctx) = &entry.wind_ctx[ci] {
                        let (sx, sy) = wind::displace(ctx, c, u, v, ct);
                        dx += sx * band * hband * wq;
                        dy += sy * band * hband * wq;
                    }
                }
                "sway" => {
                    let (sx, sy) = sway::sway_displace(c, u, v, ct, rect.width());
                    let hb = zone_mask_edge((u - z.x) / z.w.max(0.01));
                    dx += sx * hb * wq;
                    dy += sy * hb * wq;
                }
                _ => {}
            }
        }
        let pos = egui::pos2(
            rect.left() + u * rect.width() + dx,
            rect.top() + v * rect.height() + dy,
        );
        let hot = debug && (dx.abs() + dy.abs()) > 0.35;
        let col = if hot {
            egui::Color32::from_rgb(255, 70, 45)
        } else {
            egui::Color32::WHITE
        };
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::pos2(u, v),
            color: col,
        });
    }
    for iy in 0..PAGE_GRID_ROWS {
        for ix in 0..PAGE_GRID_COLS {
            let a = iy as u32 * stride + ix as u32;
            let b = a + 1;
            let c2 = a + stride;
            let d2 = c2 + 1;
            mesh.add_triangle(a, b, c2);
            mesh.add_triangle(b, d2, c2);
        }
    }
    mesh
}

fn pip_field(poly: &[[f32; 2]], cols: usize, rows: usize) -> Vec<f32> {
    let vw = cols + 1;
    let vh = rows + 1;
    let mut f = vec![1.0f32; vw * vh];
    let cell_in = |ux: usize, uy: usize| -> bool {
        let u = (ux as f32 + 0.5) / cols as f32;
        let v = (uy as f32 + 0.5) / rows as f32;
        point_in_poly(u, v, poly)
    };
    for iy in 0..vh {
        for ix in 0..vw {
            let mut inside = point_in_poly(ix as f32 / cols as f32, iy as f32 / rows as f32, poly);
            if !inside {
                if ix < cols && iy < rows && cell_in(ix, iy) { inside = true; }
                if ix > 0 && iy < rows && cell_in(ix - 1, iy) { inside = true; }
                if ix < cols && iy > 0 && cell_in(ix, iy - 1) { inside = true; }
                if ix > 0 && iy > 0 && cell_in(ix - 1, iy - 1) { inside = true; }
            }
            if !inside {
                f[iy * vw + ix] = 0.0;
            }
        }
    }
    let src = f.clone();
    for iy in 0..vh {
        for ix in 0..vw {
            let i = iy * vw + ix;
            let mut sum = src[i];
            let mut n = 1.0f32;
            if ix > 0 { sum += src[i - 1]; n += 1.0; }
            if ix < cols { sum += src[i + 1]; n += 1.0; }
            if iy > 0 { sum += src[i - vw]; n += 1.0; }
            if iy < rows { sum += src[i + vw]; n += 1.0; }
            f[i] = sum / n;
        }
    }
    f
}

pub fn point_in_poly(px: f32, py: f32, poly: &[[f32; 2]]) -> bool {
    let n = poly.len();
    if n < 3 { return false; }
    let mut inside = false;
    let mut j = n - 1;
    for p in poly {
        let (xi, yi) = (p[0], p[1]);
        let (xj, yj) = (poly[j][0], poly[j][1]);
        if (yi > py) != (yj > py)
            && px < (xj - xi) * (py - yi) / (yj - yi) + xi
        {
            inside = !inside;
        }
        j += 1;
        if j >= n { j = 0; }
    }
    inside
}

pub fn triangulate(pts: &[egui::Pos2]) -> Vec<[u32; 3]> {
    let n = pts.len();
    if n < 3 { return Vec::new(); }

    let area2 = |a: usize, b: usize, c: usize| -> f32 {
        (pts[b].x - pts[a].x) * (pts[c].y - pts[a].y)
            - (pts[b].y - pts[a].y) * (pts[c].x - pts[a].x)
    };
    let mut s = 0.0;
    for i in 0..n {
        s += area2(i, (i + 1) % n, (i + 2) % n);
    }
    let sign = if s >= 0.0 { 1.0 } else { -1.0 };
    let cross = |a: usize, b: usize, c: usize| -> f32 { area2(a, b, c) * sign };

    let mut idx: Vec<usize> = (0..n).collect();
    let mut out: Vec<[u32; 3]> = Vec::new();
    let mut guard = 0usize;

    while idx.len() > 3 && guard < 20_000 {
        guard += 1;
        let len = idx.len();
        let mut cut = None;
        for k in 0..len {
            let i0 = idx[(len + k - 1) % len];
            let i1 = idx[k];
            let i2 = idx[(k + 1) % len];
            if cross(i0, i1, i2) <= 1e-12 { continue; }
            let mut blocked = false;
            for &p in idx.iter() {
                if p == i0 || p == i1 || p == i2 { continue; }
                let d1 = cross(i0, i1, p);
                let d2 = cross(i1, i2, p);
                let d3 = cross(i2, i0, p);
                if d1 >= -1e-9 && d2 >= -1e-9 && d3 >= -1e-9 {
                    blocked = true;
                    break;
                }
            }
            if !blocked {
                cut = Some(k);
                break;
            }
        }
        match cut {
            Some(k) => {
                let len = idx.len();
                let i0 = idx[(len + k - 1) % len];
                let i1 = idx[k];
                let i2 = idx[(k + 1) % len];
                out.push([i0 as u32, i1 as u32, i2 as u32]);
                idx.remove(k);
            }
            None => break,
        }
    }
    if idx.len() == 3 {
        out.push([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
    }
    out
}

pub fn paint_polygon(painter: &egui::Painter, pts: &[egui::Pos2], color: egui::Color32) {
    if pts.len() < 3 { return; }
    let tris = triangulate(pts);
    if tris.is_empty() { return; }
    let mut mesh = egui::Mesh::default();
    for p in pts {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *p,
            uv: egui::pos2(0.0, 0.0),
            color,
        });
    }
    for t in tris {
        mesh.add_triangle(t[0], t[1], t[2]);
    }
    painter.add(egui::Shape::Mesh(mesh.into()));
}
