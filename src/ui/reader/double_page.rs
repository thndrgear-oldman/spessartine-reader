use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use super::placeholders;
use crate::lang::Lang;
use crate::reader::chapter_bundle::{VirtualPage, compute_virtual_total};

use crate::archive::SharedArchive;
use crate::scanner::PageItem;
use crate::ui::reader::{PageFlipState, FlipPhase};

fn fit_texture(
    ui: &mut egui::Ui,
    cache: &HashMap<usize, egui::TextureHandle>,
    idx: usize,
    rect: egui::Rect,
    uv: egui::Rect,
) {
    if rect.width() <= 0.0 { return; }
    if let Some(tex) = cache.get(&idx) {
        ui.painter().add(
            egui::epaint::RectShape::filled(rect, 0.0, egui::Color32::WHITE)
                .with_texture(tex.id(), uv),
        );
    } else {
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(32));
    }
}

fn render_vp_guard(
    ui: &mut egui::Ui,
    cache: &HashMap<usize, egui::TextureHandle>,
    vp: &[VirtualPage],
    vp_idx: usize,
    rect: egui::Rect,
    lang: &Lang,
    guard: bool,
) {
    render_vp(ui, cache, vp, vp_idx, rect, lang);
    if guard {
        let d = (255.0 * 0.05f32).clamp(0.0, 255.0) as u8;
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(d, d, d, 255));
    }
}

fn render_vp(
    ui: &mut egui::Ui,
    cache: &HashMap<usize, egui::TextureHandle>,
    vp: &[VirtualPage],
    vp_idx: usize,
    rect: egui::Rect,
    lang: &Lang,
) {
    if rect.width() <= 0.0 { return; }
    match vp[vp_idx] {
        VirtualPage::Real(idx) => {
            fit_texture(ui, cache, idx, rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)));
        }
        VirtualPage::PanoramaLeft(idx) => {
            fit_texture(ui, cache, idx, rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.5, 1.0)));
        }
        VirtualPage::PanoramaRight(idx) => {
            fit_texture(ui, cache, idx, rect, egui::Rect::from_min_max(egui::pos2(0.5, 0.0), egui::pos2(1.0, 1.0)));
        }
        VirtualPage::PanoramaPlaceholder => {
            placeholders::panorama(ui, rect, lang);
        }
        VirtualPage::EndPlaceholder(is_last) => {
            placeholders::end(ui, rect, is_last, lang);
        }
    }
}

#[cfg(not(feature = "fx"))]
fn render_spread(ui: &mut egui::Ui, cache: &HashMap<usize, egui::TextureHandle>, vp: &[VirtualPage], even: usize, r_rect: egui::Rect, l_rect: egui::Rect, lang: &Lang) {
    render_vp(ui, cache, vp, even, r_rect, lang);
    render_vp(ui, cache, vp, even + 1, l_rect, lang);
}

#[cfg(feature = "fx")]
fn render_vp_fx(
    ui: &mut egui::Ui,
    cache: &HashMap<usize, egui::TextureHandle>,
    vp: &[VirtualPage],
    vp_idx: usize,
    rect: egui::Rect,
    lang: &Lang,
    fx: &mut crate::fx::FxPlayer,
) {
    if rect.width() <= 0.0 { return; }
    match vp[vp_idx] {
        VirtualPage::Real(idx) => {
            if let Some(tex) = cache.get(&idx) {
                let drew = fx.draw_manga_page(ui, (idx + 1) as u32, tex.id(), rect);
                if !drew {
                    fit_texture(ui, cache, idx, rect, egui::Rect::from_min_max(
                        egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)));
                    fx.draw_curtain(ui, (idx + 1) as u32, tex.id(), rect);
                }
            } else {
                fit_texture(ui, cache, idx, rect, egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)));
            }
        }
        _ => {
            render_vp(ui, cache, vp, vp_idx, rect, lang);
        }
    }
}

#[cfg(feature = "fx")]
fn render_spread_fx(
    ui: &mut egui::Ui,
    cache: &HashMap<usize, egui::TextureHandle>,
    vp: &[VirtualPage],
    even: usize,
    first_rect: egui::Rect,
    second_rect: egui::Rect,
    lang: &Lang,
    fx: &mut crate::fx::FxPlayer,
) {
    if (matches!(vp[even], VirtualPage::PanoramaLeft(_))
            && matches!(vp[even + 1], VirtualPage::PanoramaRight(_)))
        || (matches!(vp[even], VirtualPage::PanoramaRight(_))
            && matches!(vp[even + 1], VirtualPage::PanoramaLeft(_)))
    {
        let r1 = match vp[even] {
            VirtualPage::PanoramaLeft(r) | VirtualPage::PanoramaRight(r) => r,
            _ => unreachable!(),
        };
        let r2 = match vp[even + 1] {
            VirtualPage::PanoramaLeft(r) | VirtualPage::PanoramaRight(r) => r,
            _ => unreachable!(),
        };
        if r1 == r2 {
            if let Some(tex) = cache.get(&r1) {
                let glue = egui::Rect::from_min_max(
                    first_rect.min.min(second_rect.min),
                    first_rect.max.max(second_rect.max),
                );
                if fx.draw_manga_page(ui, (r1 + 1) as u32, tex.id(), glue) {
                    return;
                }
                render_vp(ui, cache, vp, even, first_rect, lang);
                render_vp(ui, cache, vp, even + 1, second_rect, lang);
                fx.draw_curtain(ui, (r1 + 1) as u32, tex.id(), glue);
                return;
            }
            render_vp(ui, cache, vp, even, first_rect, lang);
            render_vp(ui, cache, vp, even + 1, second_rect, lang);
            return;
        }
    }
    render_vp_fx(ui, cache, vp, even, first_rect, lang, fx);
    render_vp_fx(ui, cache, vp, even + 1, second_rect, lang, fx);
}

fn anchor_slot(p: &VirtualPage) -> Option<usize> {
    match *p {
        VirtualPage::Real(r) | VirtualPage::PanoramaLeft(r) | VirtualPage::PanoramaRight(r) => Some(r),
        _ => None,
    }
}

pub fn spread_start_for(vp: &[VirtualPage], real: usize) -> usize {
    for (i, p) in vp.iter().enumerate() {
        if anchor_slot(p) == Some(real) {
            return i & !1;
        }
    }
    vp.len().saturating_sub(2) & !1
}

fn real_anchor_of(vp: &[VirtualPage], v: usize, total: usize) -> usize {
    for s in v..vp.len().min(v + 2) {
        if let Some(r) = anchor_slot(&vp[s]) {
            return r;
        }
    }
    total
}

pub fn at_final_spread(vp: &[VirtualPage], real: usize) -> bool {
    spread_start_for(vp, real) + 2 >= vp.len()
}

pub fn draw(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    page_index: &mut usize,
    archive_path: &mut String,
    real_total: usize,
    lang: &Lang,
    reader_cache: &mut HashMap<usize, egui::TextureHandle>,
    reader_loading: &mut HashSet<usize>,
    pages: &[PageItem],
    current_archive_shared: &Option<SharedArchive>,
    reader_tx: &mpsc::Sender<(String, usize, Vec<u8>, i32, i32, bool)>,
    aspect_ratios: &[f32],
    settings: &crate::config::settings::ReaderSettings,
    rtl: bool,
    container_width: f32,
    alt: bool,
    flip: &mut PageFlipState,
    is_last_chapter: bool,
    fs: bool,
    #[cfg(feature = "fx")]
    fx: &mut crate::fx::FxPlayer,
) {
    let pano_thr = settings.reader.panorama_threshold;
    let vp = crate::reader::chapter_bundle::build_virtual_pages(real_total, aspect_ratios, pano_thr, rtl, is_last_chapter);
    let virtual_total = vp.len();
    debug_assert!(virtual_total == compute_virtual_total(real_total, aspect_ratios, pano_thr));
    let cur_v = spread_start_for(&vp, *page_index);

    #[cfg(feature = "fx")]
    if !flip.is_active() {
        crate::ui::reader::fx_hooks::update_manga_spread(
            fx, *page_index, real_total, aspect_ratios, pano_thr, rtl, is_last_chapter, true);
    }

    #[cfg(feature = "fx")]
    let curtain_guard = fx.interactive() && fx.has_curtain();
    #[cfg(not(feature = "fx"))]
    let curtain_guard = false;

    #[cfg(feature = "fx")]
    let reveal_lock = fx.is_revealing();
    #[cfg(not(feature = "fx"))]
    let reveal_lock = false;

    #[cfg(feature = "fx")]
    let whole_covered = fx.interactive()
        && fx.has_curtain()
        && fx.cur_spread().is_some_and(|sp| fx.revealed(sp) == 0);
    #[cfg(not(feature = "fx"))]
    let whole_covered = false;
    #[cfg(feature = "fx")]
    let edge_covered = fx.interactive() && fx.slot_covered(1);
    #[cfg(not(feature = "fx"))]
    let edge_covered = false;

    let rect = resp.rect;
    let gap = settings.reader.double_page_gap;
    let avail = if container_width > 0.0 {
        let cw = container_width.min(resp.rect.width()).max(settings.reader.min_container_width);
        let ox = (resp.rect.width() - cw) / 2.0;
        egui::Rect::from_min_size(
            egui::pos2(resp.rect.left() + ox, resp.rect.top()),
            egui::vec2(cw, resp.rect.height()),
        )
    } else { resp.rect };

    let left_rect = egui::Rect::from_min_size(
        avail.left_top(),
        egui::vec2((avail.width() - gap) / 2.0, avail.height()),
    );
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(left_rect.right() + gap, avail.top()),
        egui::vec2((avail.width() - gap) / 2.0, avail.height()),
    );

    const DRAG_THRESHOLD: f32 = 10.0;

    if resp.drag_started() && !flip.is_active() && !whole_covered {
        flip.start_x = ui.ctx().pointer_interact_pos().unwrap_or(avail.center()).x;
        flip.from_page = cur_v;
        flip.last_px = flip.start_x;
        flip.step = 2;
    }

    if resp.dragged() {
        let px = ui.ctx().pointer_interact_pos().unwrap_or(avail.center()).x;
        flip.last_px = px;

        if matches!(flip.phase, FlipPhase::Idle) && !whole_covered {
            let offset = px - flip.start_x;
            if offset.abs() > DRAG_THRESHOLD {
                let forward = if !rtl { offset < 0.0 } else { offset > 0.0 };
                if forward && edge_covered {
                } else {
                    flip.phase = FlipPhase::Dragging;
                    flip.start_x += offset.signum() * DRAG_THRESHOLD;
                    flip.is_forward = forward;

                    let start = cur_v;
                    if forward {
                        if start + 2 < virtual_total {
                            flip.front_page = start;
                            flip.back_page = start + 2;
                            flip.target_page = start + 2;
                            flip.target_real = real_anchor_of(&vp, start + 2, real_total);
                        } else {
                            flip.phase = FlipPhase::Idle;
                        }
                    } else {
                        if start >= 2 {
                            flip.front_page = start;
                            flip.back_page = start - 2;
                            flip.target_page = start - 2;
                            flip.target_real = real_anchor_of(&vp, start - 2, real_total);
                        } else {
                            flip.phase = FlipPhase::Idle;
                        }
                    }
                }
            }
        }
    }

    if resp.drag_stopped() {
        if matches!(flip.phase, FlipPhase::Dragging) {
            let offset = flip.last_px - flip.start_x;
            let split_side = flip.is_forward != rtl;
            let split = (if split_side { avail.right() } else { avail.left() } + offset)
                .clamp(avail.left(), avail.right());
            let gap_mid = left_rect.right() + gap * 0.5;
            let crossed = if split_side { split <= gap_mid } else { split >= gap_mid };
            if crossed {
                flip.commit(split);
            } else {
                flip.abort(split);
            }
        }
    }

    let split_x = if matches!(flip.phase, FlipPhase::Dragging) {
        let px = flip.last_px;
        let offset = px - flip.start_x;
        let split_side = flip.is_forward != rtl;
        (if split_side { avail.right() } else { avail.left() } + offset)
            .clamp(avail.left(), avail.right())
    } else if let FlipPhase::Animating(t) = flip.phase {
        let split_side = flip.is_forward != rtl;
        let start = if flip.release_split.is_nan() {
            if split_side { avail.right() } else { avail.left() }
        } else {
            flip.release_split
        };
        let target = if flip.committing {
            if split_side { avail.left() } else { avail.right() }
        } else {
            if split_side { avail.right() } else { avail.left() }
        };
        egui::lerp(start..=target, t)
    } else {
        if flip.is_forward != rtl { avail.right() } else { avail.left() }
    };

    let even = cur_v & !1;

    #[cfg(feature = "fx")]
    let guard_back = if flip.back_page >= vp.len() {
        false
    } else {
        fx.interactive() && fx.spread_covered_any((flip.back_page & !1) as u64)
    };
    #[cfg(not(feature = "fx"))]
    let guard_back = false;
    #[cfg(feature = "fx")]
    let guard_cur_even = fx.interactive() && fx.slot_covered(0);
    #[cfg(not(feature = "fx"))]
    let guard_cur_even = false;
    #[cfg(feature = "fx")]
    let guard_cur_next = fx.interactive() && (fx.slot_covered(1) || fx.spread_covered_any(even as u64));
    #[cfg(not(feature = "fx"))]
    let guard_cur_next = false;

    if rtl {
        if flip.is_active() {
            let b_even = flip.back_page & !1;
            let in_left = split_x < left_rect.right() + gap * 0.5;

            if flip.is_forward {
                if in_left {
                    let clip_b_l = egui::Rect::from_min_max(
                        egui::pos2(split_x, left_rect.top()),
                        left_rect.right_bottom(),
                    );
                    render_vp_guard(ui, reader_cache, &vp, b_even + 1, left_rect, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, even + 1, clip_b_l, lang, guard_cur_next);
                    render_vp_guard(ui, reader_cache, &vp, even, right_rect, lang, guard_cur_even);
                } else {
                    let clip_f_l = egui::Rect::from_min_max(
                        right_rect.left_top(),
                        egui::pos2(split_x, right_rect.bottom()),
                    );
                    render_vp_guard(ui, reader_cache, &vp, b_even + 1, left_rect, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, even, right_rect, lang, guard_cur_even);
                    render_vp_guard(ui, reader_cache, &vp, b_even, clip_f_l, lang, guard_back);
                }
            } else {
                if in_left {
                    let clip_f_l = egui::Rect::from_min_max(
                        egui::pos2(split_x, left_rect.top()),
                        left_rect.right_bottom(),
                    );
                    render_vp_guard(ui, reader_cache, &vp, even + 1, left_rect, lang, guard_cur_next);
                    render_vp_guard(ui, reader_cache, &vp, b_even + 1, clip_f_l, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, b_even, right_rect, lang, guard_back);
                } else {
                    let clip_b_r = egui::Rect::from_min_max(
                        right_rect.left_top(),
                        egui::pos2(split_x, right_rect.bottom()),
                    );
                    render_vp_guard(ui, reader_cache, &vp, even + 1, left_rect, lang, guard_cur_next);
                    render_vp_guard(ui, reader_cache, &vp, b_even, right_rect, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, even, clip_b_r, lang, guard_cur_even);
                }
            }
        } else {
            #[cfg(feature = "fx")]
            render_spread_fx(ui, reader_cache, &vp, even, right_rect, left_rect, lang, fx);
            #[cfg(not(feature = "fx"))]
            render_spread(ui, reader_cache, &vp, even, right_rect, left_rect, lang);
        }
    } else {
        if flip.is_active() {
            let b_even = flip.back_page & !1;
            let in_left = split_x < left_rect.right() + gap * 0.5;

            if flip.is_forward {
                if in_left {
                    let clip_b_l = egui::Rect::from_min_max(
                        egui::pos2(split_x, left_rect.top()),
                        left_rect.right_bottom(),
                    );
                    render_vp_guard(ui, reader_cache, &vp, even, left_rect, lang, guard_cur_even);
                    render_vp_guard(ui, reader_cache, &vp, b_even, clip_b_l, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, b_even + 1, right_rect, lang, guard_back);
                } else {
                    let clip_f_r = egui::Rect::from_min_max(
                        right_rect.left_top(),
                        egui::pos2(split_x, right_rect.bottom()),
                    );
                    render_vp_guard(ui, reader_cache, &vp, even, left_rect, lang, guard_cur_even);
                    render_vp_guard(ui, reader_cache, &vp, b_even + 1, right_rect, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, even + 1, clip_f_r, lang, guard_cur_next);
                }
            } else {
                if in_left {
                    let clip_f_l = egui::Rect::from_min_max(
                        egui::pos2(split_x, left_rect.top()),
                        left_rect.right_bottom(),
                    );
                    render_vp_guard(ui, reader_cache, &vp, b_even, left_rect, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, even, clip_f_l, lang, guard_cur_even);
                    render_vp_guard(ui, reader_cache, &vp, even + 1, right_rect, lang, guard_cur_next);
                } else {
                    let clip_b_r = egui::Rect::from_min_max(
                        right_rect.left_top(),
                        egui::pos2(split_x, right_rect.bottom()),
                    );
                    render_vp_guard(ui, reader_cache, &vp, b_even, left_rect, lang, guard_back);
                    render_vp_guard(ui, reader_cache, &vp, even + 1, right_rect, lang, guard_cur_next);
                    render_vp_guard(ui, reader_cache, &vp, b_even + 1, clip_b_r, lang, guard_back);
                }
            }
        } else {
            #[cfg(feature = "fx")]
            render_spread_fx(ui, reader_cache, &vp, even, left_rect, right_rect, lang, fx);
            #[cfg(not(feature = "fx"))]
            render_spread(ui, reader_cache, &vp, even, left_rect, right_rect, lang);
        }
    }

    #[cfg(feature = "fx")]
    if !flip.is_active() && fx.interactive() {
        // плейсхолдеры ридера («Далее панорама», «Конец главы/произведения»)
        // завешиваются шторой даже без записи в .spfx: служебный кусок спреда
        let slot_rects = if rtl { [right_rect, left_rect] } else { [left_rect, right_rect] };
        for (si, vi) in [even, even + 1].iter().enumerate() {
            let is_ph = matches!(
                vp.get(*vi),
                Some(VirtualPage::PanoramaPlaceholder) | Some(VirtualPage::EndPlaceholder(_))
            );
            if !is_ph {
                continue;
            }
            let count = fx.slot_count(si);
            if count == 0 {
                continue;
            }
            let rev = fx.slot_reveal(si);
            let dim_a = if rev == 0 { fx.curtain_dim() } else { 0.0 };
            if dim_a <= 0.0 {
                continue;
            }
            let d = (255.0 * dim_a).clamp(0.0, 255.0) as u8;
            ui.painter().rect_filled(
                slot_rects[si],
                0.0,
                egui::Color32::from_rgba_unmultiplied(d, d, d, 255),
            );
        }
    }

    let half_w = (rect.width() - gap) / 2.0;
    let shadow_w = settings.reader.double_page_shadow_width;
    let alpha = settings.reader.double_page_shadow_alpha as u8;
    let l = rect.left() + half_w;
    let r = l + gap;
    let t = rect.top();
    let b = rect.bottom();

    let mut mesh = egui::Mesh::default();

    let idx = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(l - shadow_w, t), uv: egui::pos2(0.0, 0.0), color: egui::Color32::TRANSPARENT });
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(l, t), uv: egui::pos2(0.0, 0.0), color: egui::Color32::from_black_alpha(alpha) });
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(l, b), uv: egui::pos2(0.0, 0.0), color: egui::Color32::from_black_alpha(alpha) });
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(l - shadow_w, b), uv: egui::pos2(0.0, 0.0), color: egui::Color32::TRANSPARENT });
    mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

    let idx = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(r, t), uv: egui::pos2(0.0, 0.0), color: egui::Color32::from_black_alpha(alpha) });
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(r + shadow_w, t), uv: egui::pos2(0.0, 0.0), color: egui::Color32::TRANSPARENT });
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(r + shadow_w, b), uv: egui::pos2(0.0, 0.0), color: egui::Color32::TRANSPARENT });
    mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(r, b), uv: egui::pos2(0.0, 0.0), color: egui::Color32::from_black_alpha(alpha) });
    mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

    ui.painter().add(egui::Shape::Mesh(mesh.into()));

    if resp.clicked() && !alt && !flip.is_active() {
        if curtain_guard {
            return;
        }
        let pointer = ui.ctx().pointer_interact_pos().unwrap_or(avail.center());
        let mid_x = avail.center().x;
        let cz = avail.width() * 0.1;
        if !(fs && (pointer.x - mid_x).abs() < cz) && !reveal_lock {
            if crate::reader::chapter_switch::is_prev_section(pointer.x, mid_x, rtl) {
                trigger_flip(flip, cur_v, &vp, real_total, false);
            } else if crate::reader::chapter_switch::is_next_section(pointer.x, mid_x, rtl) {
                trigger_flip(flip, cur_v, &vp, real_total, true);
            }
        }
    }

    let mut real_cur = 0;
    for (vi, page) in vp.iter().enumerate() {
        match *page {
            VirtualPage::Real(r) | VirtualPage::PanoramaLeft(r) | VirtualPage::PanoramaRight(r) => {
                if vi <= even { real_cur = r; }
            }
            _ => {}
        }
    }
    let start = real_cur.saturating_sub(settings.preload.double_spread_start.unsigned_abs() as usize * 2);
    let end = (real_cur + settings.preload.double_spread_end as usize * 2).min(real_total);
    for idx in start..end {
        if !reader_cache.contains_key(&idx) && !reader_loading.contains(&idx) {
            reader_loading.insert(idx);
            let _ = crate::reader::worker::decode_tx().send(
                crate::reader::worker::DecodeJob {
                    ap: archive_path.clone(),
                    n: idx,
                    page_name: pages[idx].name.clone(),
                    max_decode_width: settings.reader.max_decode_width,
                    arcs: current_archive_shared.clone(),
                    tx: reader_tx.clone(),
                    use_max_width: false,
                }
            );
        }
    }
}

pub fn trigger_flip(flip: &mut PageFlipState, start_v: usize, vp: &[VirtualPage], total: usize, forward: bool) {
    if flip.is_active() { return; }
    let virtual_total = vp.len();
    let start = start_v & !1;
    let allowed = if forward { start + 2 < virtual_total } else { start >= 2 };
    if !allowed { return; }
    let target = if forward { start + 2 } else { start - 2 };
    flip.phase = FlipPhase::Animating(0.0);
    flip.from_page = start;
    flip.target_page = target;
    flip.target_real = real_anchor_of(vp, target, total);
    flip.front_page = start;
    flip.back_page = target;
    flip.step = 2;
    flip.is_forward = forward;
    flip.committing = true;
    flip.release_split = f32::NAN;
}

pub fn trigger_flip_from_real(
    flip: &mut PageFlipState,
    real: usize,
    aspect_ratios: &[f32],
    threshold: f32,
    rtl: bool,
    is_last_chapter: bool,
    forward: bool,
) {
    let vp = crate::reader::chapter_bundle::build_virtual_pages(aspect_ratios.len(), aspect_ratios, threshold, rtl, is_last_chapter);
    let start = spread_start_for(&vp, real);
    trigger_flip(flip, start, &vp, aspect_ratios.len(), forward);
}
