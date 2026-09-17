use super::Rng;
use crate::fx::soundmap::FxCue;

pub type WindCtx = (f32, f32, f32, f32, f32, Vec<f32>, Vec<f32>);

pub fn context(c: &FxCue, rect_w: f32) -> WindCtx {
    let inten = c.intensity.unwrap_or(0.6).clamp(-1.0, 1.0);
    let dir: f32 = if inten < 0.0 { -1.0 } else { 1.0 };
    let mag = inten.abs() * 0.03 * rect_w;
    let rad = c.angle.unwrap_or(0.0).to_radians();
    let (ax, ay) = (rad.cos(), -rad.sin());
    let period = c.dur.unwrap_or(1.5).max(0.3);
    let n = c.bands.unwrap_or(2).clamp(1, 8) as usize;
    let mut r = Rng::new(c.seed.unwrap_or(9) ^ 0x51DE);
    let mut offs = Vec::with_capacity(n);
    let mut muls = Vec::with_capacity(n);
    for _ in 0..n {
        offs.push(r.f32() * 0.8);
        muls.push(0.7 + r.f32() * 0.8);
    }
    (ax, ay, mag, dir, period, offs, muls)
}

pub fn displace(ctx: &WindCtx, c: &FxCue, u: f32, v: f32, t: f32) -> (f32, f32) {
    let (ax, ay, mag, dir, period, offs, muls) = ctx;
    let proj = (u - 0.5) * ax + (v - 0.5) * ay;
    let mut pmin = f32::MAX;
    let mut pmax = f32::MIN;
    if let Some(ap) = &c.area {
        for p in ap {
            let pr = (p[0] - 0.5) * ax + (p[1] - 0.5) * ay;
            pmin = pmin.min(pr);
            pmax = pmax.max(pr);
        }
    } else {
        pmin = -0.5;
        pmax = 0.5;
    }
    const MARGIN: f32 = 0.12;
    let lo = pmin - MARGIN;
    let span = (pmax - pmin).max(0.01) + 2.0 * MARGIN;
    let mut disp = 0.0f32;
    for (off, ml) in offs.iter().zip(muls.iter()) {
        let pc = ((t / period) + *off).fract();
        if pc > 0.8 { continue; }
        let pa = pc / 0.8;
        let front = lo + pa * span;
        let rel = (proj - front) * dir;
        if rel.abs() > 0.6 { continue; }
        let bw = c.band_width.unwrap_or(0.35).clamp(0.001, 1.5);
        let dr_s = 0.0129 * bw;
        let df_s = 0.0571 * bw;
        let g = if rel < 0.0 {
            (-(rel * rel) / (2.0 * dr_s)).exp()
        } else {
            (-(rel * rel) / (2.0 * df_s)).exp()
        };
        disp += g * ml;
    }
    if disp > 0.002 {
        let ph_n = (c.seed.unwrap_or(5) % 628) as f32 / 100.0;
        let wob = ((t * 2.9 + ph_n) * std::f32::consts::TAU).sin().abs()
            * 0.18 * disp.min(1.0);
        let k = (disp + wob).min(1.5);
        (ax * mag * dir * k, ay * mag * dir * k)
    } else {
        (0.0, 0.0)
    }
}
