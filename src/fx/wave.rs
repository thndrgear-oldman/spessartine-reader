use super::zone_mask_edge;
use crate::fx::soundmap::FxCue;

pub fn displace(c: &FxCue, u: f32, v: f32, t: f32, rect_w: f32, rect_h: f32) -> (f32, f32) {
    let z = c.zone.unwrap_or_default();
    let vertical = c.kind == "wave2" || c.axis.unwrap_or(0) == 1;
    let hband = zone_mask_edge((u - z.x) / z.w.max(0.01));
    let band = zone_mask_edge((v - z.y) / z.h.max(0.01));
    let amp = c.intensity.unwrap_or(0.5).clamp(0.0, 1.0)
        * 0.015
        * if vertical { rect_h } else { rect_w };
    let hz = c.hz.unwrap_or(0.8).max(0.05);
    let phase = (c.seed.unwrap_or(7) % 628) as f32 / 100.0;
    let wv = (if vertical { u } else { v } * 1.5 + t * hz) * std::f32::consts::TAU + phase;
    let d = amp * band * hband * wv.sin();
    if vertical {
        (0.0, d)
    } else {
        (d, 0.0)
    }
}
