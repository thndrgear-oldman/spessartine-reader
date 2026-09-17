use super::zone_mask_edge;
use crate::fx::soundmap::FxCue;

pub fn displace(c: &FxCue, u: f32, v: f32, t: f32, rect_w: f32, rect_h: f32) -> (f32, f32) {
    let z = c.zone.unwrap_or_default();
    let vertical = c.axis.unwrap_or(0) == 1;
    let band = zone_mask_edge((v - z.y) / z.h.max(0.01));
    let hband = zone_mask_edge((u - z.x) / z.w.max(0.01));
    let amp = c.intensity.unwrap_or(0.5).clamp(0.0, 1.0)
        * 0.02
        * if vertical { rect_h } else { rect_w };
    let hz = c.hz.unwrap_or(6.0).max(0.5);
    let dur = c.dur.unwrap_or(0.8).max(0.05);
    let cycles = (hz * dur).round().max(1.0);
    let wv = (t / dur * cycles) * std::f32::consts::TAU;
    let d = amp * band * hband * wv.sin() * dir_sign(c.seed) * shot_env(t, c, dur);
    if vertical {
        (0.0, d)
    } else {
        (d, 0.0)
    }
}

fn dir_sign(seed: Option<u32>) -> f32 {
    match seed.unwrap_or(0) % 2 {
        0 => 1.0,
        _ => -1.0,
    }
}

fn shot_env(t: f32, c: &FxCue, dur: f32) -> f32 {
    if !c.oneshot.unwrap_or(false) {
        return 1.0;
    }
    let k = (t / dur).clamp(0.0, 1.0);
    if k <= 0.5 {
        1.0
    } else {
        let u = ((k - 0.5) * 2.0).clamp(0.0, 1.0);
        let s = u * u * (3.0 - 2.0 * u);
        1.0 - s
    }
}
