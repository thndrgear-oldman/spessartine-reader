use eframe::egui;
use crate::ui::webtoon_reader::state::PullDir;
use std::collections::HashMap;
use crate::ui::webtoon_reader::state::{WebtoonState, current_index};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RotaDir { Up, Down }

fn pull_resistance(raw: f32, resistance: f32) -> f32 {
    let abs = raw.abs();
    raw.signum() * resistance * (abs / (abs + resistance))
}

pub fn scroll_controller(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut WebtoonState,
    cum_y: &[f32],
    settings: &crate::config::settings::ReaderSettings,
    prev_cache: &HashMap<usize, egui::TextureHandle>,
    current_cache: &HashMap<usize, egui::TextureHandle>,
    joined_cache: &HashMap<usize, egui::TextureHandle>,
    _width: f32,
    #[cfg(feature = "fx")]
    fx: &mut crate::fx::FxPlayer,
) -> Option<RotaDir> {
    let dt = ui.input(|i| i.stable_dt).min(settings.scroll.dt_min_clamp);
    let vp_h = rect.height();
    let spacing = settings.webtoon.page_spacing;
    let prev_len = state.prev_h.len();
    let current_len = state.current_h.len();
    let _joined_len = state.joined_h.len();
    let _total = state.prev_h.iter().chain(state.current_h.iter()).chain(state.joined_h.iter());
    let content_width = state.ref_width.min(rect.width()).max(settings.webtoon.min_content_width);
    let offset_x = (rect.width() - content_width) / 2.0;

    let total_h = cum_y[cum_y.len() - 1];
    let diff = total_h - state.last_total_h;
    if diff.abs() > settings.scroll.height_change_threshold && state.rotation_lock == 0 {
        state.frame_offset = (state.frame_offset + diff).max(0.0);
    }
    state.last_total_h = total_h;

    if state.rotation_lock > 0 {
        state.rotation_lock -= 1;
    }

    let pointer = ui.input(|i| i.pointer.clone());
    let alt = ui.input(|i| i.modifiers.alt);
    if !alt && pointer.any_down() {
        state.velocity = -pointer.velocity().y * dt;
        state.frame_offset -= pointer.delta().y;
     }
    let (sd, raw, hover_pos) = ui.input(|i| (i.smooth_scroll_delta.y, i.raw_scroll_delta.y, i.pointer.hover_pos()));
    let on_content = hover_pos.is_some_and(|hp| rect.contains(hp));
    let arrow_scroll = {
        let inp = ui.input(|i| i.clone());
        if inp.key_pressed(egui::Key::ArrowDown) { -1.0 }
        else if inp.key_pressed(egui::Key::ArrowUp) { 1.0 }
        else { 0.0 }
    };
    if alt && raw != 0.0 && on_content {
        let max_w = rect.width() - 1.0;
        let min_w = settings.webtoon.min_width.min(max_w);
        let step = settings.webtoon.resize_step;
        let new = (state.ref_width + raw.signum() * step as f32).clamp(min_w, max_w);
        if (new - state.ref_width).abs() > settings.webtoon.resize_threshold {
            let scale = new / state.ref_width.max(f32::EPSILON);
            state.ref_width = new;
            for h in &mut state.prev_h { *h *= scale; }
            for h in &mut state.current_h { *h *= scale; }
            for h in &mut state.joined_h { *h *= scale; }
            state.frame_offset *= scale;
            state.last_total_h *= scale;
        }
        state.velocity = 0.0;
        ui.input_mut(|i| {
            i.smooth_scroll_delta = egui::Vec2::ZERO;
            i.raw_scroll_delta = egui::Vec2::ZERO;
        });
    } else if !alt && arrow_scroll != 0.0 {
        state.velocity = state.velocity.signum() * state.velocity.abs().min(settings.scroll.max_velocity);
        state.velocity -= arrow_scroll * settings.scroll.keys_scroll_factor;
        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    } else if !alt && sd != 0.0 && on_content {
        state.velocity = state.velocity.signum() * state.velocity.abs().min(settings.scroll.max_velocity);
        state.velocity -= sd * state.wheel_sensitivity * settings.scroll.wheel_scroll_factor;
        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    } else if sd != 0.0 {
        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    }

    let max_offset = (total_h - vp_h).max(0.0);
    let inp = ui.input(|i| i.clone());
    if inp.key_pressed(egui::Key::Home) { state.frame_offset = 0.0; state.velocity = 0.0; ui.ctx().request_repaint(); }
    if inp.key_pressed(egui::Key::End) { state.frame_offset = max_offset; state.velocity = 0.0; ui.ctx().request_repaint(); }
    if inp.key_pressed(egui::Key::PageDown) { state.frame_offset = (state.frame_offset + vp_h * settings.scroll.page_scroll_fraction).min(max_offset); ui.ctx().request_repaint(); }
    if inp.key_pressed(egui::Key::PageUp) { state.frame_offset = (state.frame_offset - vp_h * settings.scroll.page_scroll_fraction).max(0.0); ui.ctx().request_repaint(); }

    if state.velocity.abs() > settings.scroll.velocity_inertia_threshold && !pointer.any_down() {
        state.frame_offset += state.velocity;
        state.velocity *= settings.scroll.inertia_decay.powf(settings.scroll.inertia_decay_fps * dt);
        ui.ctx().request_repaint();
    } else {
        state.velocity = 0.0;
    }

    if state.frame_offset < 0.0 {
        state.frame_offset = pull_resistance(state.frame_offset, settings.scroll.pull_resistance);
        state.pull = Some(crate::ui::webtoon_reader::state::PullDir::Top);
        state.pull_progress = (-state.frame_offset / (vp_h * settings.scroll.pull_overscroll_fraction)).min(settings.scroll.pull_progress_max);
    } else if state.frame_offset > max_offset {
        let diff = state.frame_offset - max_offset;
        state.frame_offset = pull_resistance(state.frame_offset, settings.scroll.pull_resistance);
        state.pull = Some(crate::ui::webtoon_reader::state::PullDir::Bottom);
        state.pull_progress = (diff / (vp_h * settings.scroll.pull_overscroll_fraction)).min(settings.scroll.pull_progress_max);
    } else {
        state.pull = None;
        state.pull_progress = 0.0;
    }

    state.frame_offset = state.frame_offset.max(0.0);

    let mut rot = None;
    if state.rotation_lock == 0 {
        let cur = current_index(state.frame_offset, cum_y);
        if cur < prev_len && !state.prev_h.is_empty() && state.pull == Some(PullDir::Top) {
            rot = Some(RotaDir::Up);
        } else if cur >= prev_len + current_len && !state.joined_h.is_empty() && state.velocity <= settings.scroll.velocity_inertia_threshold {
            rot = Some(RotaDir::Down);
        }
    }

    if state.rotation_lock == 0 {
        let cur = current_index(state.frame_offset, cum_y);
        let local = cur.saturating_sub(prev_len);
        state.near_end = local >= current_len.saturating_sub(settings.scroll.near_threshold as usize);
        state.near_start = local <= 1;
    }

    let mut vs = 0usize;
    let mut ve = cum_y.len() - 1;
    for i in 0..cum_y.len() - 1 {
        let page_end = cum_y[i] + state.current_h.get(i.saturating_sub(prev_len)).or(state.prev_h.get(i)).or(state.joined_h.get(i.saturating_sub(prev_len + current_len))).copied().unwrap_or(0.0);
        if page_end < state.frame_offset { vs = i + 1; }
        if cum_y[i] > state.frame_offset + vp_h && ve == cum_y.len() - 1 { ve = i; break; }
    }
    let buf = settings.scroll.visible_buffer as usize;
    vs = vs.saturating_sub(buf);
    ve = (ve + buf).min(cum_y.len() - 1);

    // триггеры fx текущей главы по видимости (действия/фон/bgm/oneshot-кью)
    #[cfg(feature = "fx")]
    {
        let mut visible: Vec<(u32, f32, f32)> = Vec::new();
        for idx in vs..ve {
            if !(idx >= prev_len && idx < prev_len + current_len) {
                continue;
            }
            let top = cum_y[idx];
            let h = (cum_y[idx + 1] - cum_y[idx] - spacing as f32).max(0.0);
            visible.push(((idx - prev_len + 1) as u32, top, top + h));
        }
        let vt = state.frame_offset.max(0.0);
        fx.webtoon_poll(&visible, vt, vt + rect.height());
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        for idx in vs..ve {
            let y = cum_y[idx] - state.frame_offset;
            let h = cum_y[idx + 1] - cum_y[idx] - spacing as f32;
            let pr = egui::Rect::from_min_size(
                egui::pos2(rect.left() + offset_x, rect.top() + y),
                egui::vec2(content_width, h),
            );

            let tex_opt = if idx < prev_len { prev_cache.get(&idx) }
                else if idx < prev_len + current_len { current_cache.get(&(idx - prev_len)) }
                else { joined_cache.get(&(idx - prev_len - current_len)) };

            if let Some(tex) = tex_opt {
                // fx применяем только к страницам текущей главы (карта эффектов
                // открыта для неё; prev/joined — другие главы без совпадений)
                #[cfg(feature = "fx")]
                let fx_drew = if idx >= prev_len && idx < prev_len + current_len {
                    fx.draw_webtoon_page(ui, (idx - prev_len + 1) as u32, tex.id(), pr)
                } else {
                    false
                };
                #[cfg(not(feature = "fx"))]
                let fx_drew = false;
                if !fx_drew {
                    ui.painter().add(egui::epaint::RectShape::filled(pr, 0.0, egui::Color32::WHITE)
                        .with_texture(tex.id(), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))));
                }
            } else {
                ui.painter().rect_filled(pr, 0.0, egui::Color32::from_gray(32));
                ui.painter().rect_stroke(pr, 1.0, egui::Stroke::new(1.0, egui::Color32::from_gray(48)), egui::StrokeKind::Middle);
            }
        }
    });
    rot
}
