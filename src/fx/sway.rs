use crate::fx::soundmap::FxCue;

pub fn sway_displace(c: &FxCue, u: f32, v: f32, t: f32, rect_w: f32) -> (f32, f32) {
    let z = c.zone.unwrap_or_default();
    let anch = c.anchor.unwrap_or(0);
    let dn_v = ((v - z.y) / z.h.max(0.01)).clamp(0.0, 1.0);
    let dn_u = ((u - z.x) / z.w.max(0.01)).clamp(0.0, 1.0);
    let (s, horiz) = match anch {
        1 => (1.0 - dn_v, true),
        2 => (dn_u, false),
        3 => (1.0 - dn_u, false),
        _ => (dn_v, true),
    };
    let stiff = c.band_width.unwrap_or(1.5).clamp(0.3, 4.0);
    let wl = c.wavelen.unwrap_or(1.0).max(0.15);
    let cyc_hz = c.hz.unwrap_or(0.6).max(0.05);
    let amp = c.intensity.unwrap_or(0.4).clamp(0.0, 1.0) * 0.025 * rect_w;
    let fall = s.powf(stiff);

    let ph = (t * cyc_hz - s * wl) * std::f32::consts::TAU
        + noise_phase(u, v, 7, c.seed.unwrap_or(3));
    let dir = if horiz { (1.0f32, 0.0f32) } else { (0.0, 1.0) };
    let rad = c.angle.unwrap_or(0.0).to_radians();
    let (dx0, dy0) = (
        dir.0 * rad.cos() - dir.1 * rad.sin(),
        dir.0 * rad.sin() + dir.1 * rad.cos(),
    );
    let m = amp * fall * ph.sin();
    (dx0 * m, dy0 * m)
}

fn noise_cell_hash(ix: i32, iy: i32, seed: u32) -> f32 {
    let mut h = ix.wrapping_mul(374761393)
        .wrapping_add(iy.wrapping_mul(668265263));
    h = h.wrapping_add((seed ^ 0x9E37_79B9) as i32);
    h ^= h >> 13;
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    (h as u32 % 1000) as f32 / 1000.0
}

fn noise_phase(u: f32, v: f32, cells: i32, seed: u32) -> f32 {
    let x = u * cells as f32;
    let y = v * cells as f32;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let a = noise_cell_hash(x0, y0, seed);
    let b = noise_cell_hash(x0 + 1, y0, seed);
    let c = noise_cell_hash(x0, y0 + 1, seed);
    let d = noise_cell_hash(x0 + 1, y0 + 1, seed);
    let n = a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy;
    n * std::f32::consts::TAU
}
