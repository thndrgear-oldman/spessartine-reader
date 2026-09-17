use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use crate::archive::SharedArchive;
use crate::scanner::PageItem;
use crate::lang::Lang;
use crate::ui::reader::{PageFlipState, FlipPhase};

pub fn draw(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    page_index: &mut usize,
    archive_path: &mut String,
    total: usize,
    aspect_ratios: &[f32],
    lang: &Lang,
    reader_cache: &mut HashMap<usize, egui::TextureHandle>,
    reader_loading: &mut HashSet<usize>,
    pages: &[PageItem],
    current_archive_shared: &Option<SharedArchive>,
    reader_tx: &mpsc::Sender<(String, usize, Vec<u8>, i32, i32, bool)>,
    settings: &crate::config::settings::ReaderSettings,
    rtl: bool,
    container_width: f32,
    alt: bool,
    flip: &mut PageFlipState,
    fs: bool,
    #[cfg(feature = "fx")]
    fx: &mut crate::fx::FxPlayer,
) {
    let pano_thr = settings.reader.panorama_threshold;
    let cur_aspect = aspect_ratios.get(*page_index).copied().unwrap_or(0.0);
    let is_pano = cur_aspect > pano_thr;

    #[cfg(feature = "fx")]
    if !flip.is_active() {
        crate::ui::reader::fx_hooks::update_manga_spread(
            fx, *page_index, total, aspect_ratios, pano_thr, rtl, false, false);
    }

    #[cfg(feature = "fx")]
    let curtain_guard = fx.interactive() && fx.has_curtain();
    #[cfg(not(feature = "fx"))]
    let curtain_guard = false;

    #[cfg(feature = "fx")]
    let reveal_lock = fx.is_revealing();
    #[cfg(not(feature = "fx"))]
    let reveal_lock = false;

    let base_cw = if container_width > 0.0 {
        container_width.min(resp.rect.width()).max(settings.reader.min_container_width)
    } else {
        resp.rect.width()
    };

    let mut eff_cw = base_cw;
    if is_pano {
        let ref_idx = (0..*page_index).rev().find(|&i| {
            let a = aspect_ratios.get(i).copied().unwrap_or(0.0);
            a > 0.0 && a <= pano_thr
        });
        let pano_px = reader_cache.get(page_index).map(|t| t.size()[0]);
        let ref_px = ref_idx.and_then(|i| reader_cache.get(&i)).map(|t| t.size()[0]);
        if let (Some(pw), Some(rw)) = (pano_px, ref_px) {
            if rw > 0 {
                eff_cw = base_cw * pw as f32 / rw as f32;
            }
        } else if let Some(ri) = ref_idx {
            let ra = aspect_ratios.get(ri).copied().unwrap_or(0.0);
            if ra > 0.0 {
                eff_cw = base_cw * cur_aspect / ra;
            }
        }
        eff_cw = eff_cw.clamp(base_cw, resp.rect.width());
    }

    let avail = if container_width > 0.0 {
        let ox = (resp.rect.width() - eff_cw) / 2.0;
        egui::Rect::from_min_size(
            egui::pos2(resp.rect.left() + ox, resp.rect.top()),
            egui::vec2(eff_cw, resp.rect.height()),
        )
    } else { resp.rect };

    const DRAG_THRESHOLD: f32 = 10.0;

    if resp.drag_started() && !flip.is_active() && !curtain_guard {
        flip.start_x = ui.ctx().pointer_interact_pos().unwrap_or(avail.center()).x;
        flip.from_page = *page_index;
        flip.last_px = flip.start_x;
        flip.step = 1;
    }

    if resp.dragged() {
        let px = ui.ctx().pointer_interact_pos().unwrap_or(avail.center()).x;
        flip.last_px = px;

        if matches!(flip.phase, FlipPhase::Idle) && !curtain_guard {
            let offset = px - flip.start_x;
            if offset.abs() > DRAG_THRESHOLD {
                flip.phase = FlipPhase::Dragging;
                flip.start_x += offset.signum() * DRAG_THRESHOLD;
                flip.is_forward = if !rtl { offset < 0.0 } else { offset > 0.0 };

                if flip.is_forward {
                    if flip.from_page + 1 < total {
                        flip.front_page = flip.from_page;
                        flip.back_page = flip.from_page + 1;
                        flip.target_page = flip.from_page + 1;
                    } else {
                        flip.phase = FlipPhase::Idle;
                    }
                } else {
                    if flip.from_page > 0 {
                        flip.front_page = flip.from_page;
                        flip.back_page = flip.from_page - 1;
                        flip.target_page = flip.from_page - 1;
                    } else {
                        flip.phase = FlipPhase::Idle;
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
            if offset.abs() > avail.width() * 0.5 {
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
  
    #[cfg(feature = "fx")]
    let guard_front = fx.interactive() && fx.has_curtain();
    #[cfg(not(feature = "fx"))]
    let guard_front = false;
    #[cfg(feature = "fx")]
    let guard_back = if flip.back_page < total {
        fx.interactive() && fx.spread_covered_any(flip.back_page as u64)
    } else {
        false
    };
    #[cfg(not(feature = "fx"))]
    let guard_back = false;

    const CURTAIN_DIM: f32 = 0.05;
    let dim = |ui: &mut egui::Ui, rect: egui::Rect| {
        let d = (255.0 * CURTAIN_DIM).clamp(0.0, 255.0) as u8;
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(d, d, d, 255));
    };

    if rtl {
        if flip.is_active() {
            if flip.is_forward {
                let clip = egui::Rect::from_min_max(egui::pos2(split_x, avail.top()), avail.right_bottom(),);
                fit_texture_or_placeholder(ui, reader_cache, flip.back_page, avail);
                if guard_back { dim(ui, avail); }
                fit_texture_or_placeholder(ui, reader_cache, flip.front_page, clip);
                if guard_front { dim(ui, clip); }
            } else {
                let clip = egui::Rect::from_min_max(avail.left_top(), egui::pos2(split_x, avail.bottom()),);
                fit_texture_or_placeholder(ui, reader_cache, flip.back_page, avail);
                if guard_back { dim(ui, avail); }
                fit_texture_or_placeholder(ui, reader_cache, flip.front_page, clip);
                if guard_front { dim(ui, clip); }
            }
        } else if let Some(tex) = reader_cache.get(page_index) {
            #[cfg(feature = "fx")]
            let fx_drew = fx.draw_manga_page(ui, (*page_index + 1) as u32, tex.id(), avail);
            #[cfg(not(feature = "fx"))]
            let fx_drew = false;
            if !fx_drew {
                ui.painter().add(egui::epaint::RectShape::filled(avail, 0.0, egui::Color32::WHITE).with_texture(tex.id(), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0),
                    )),
                );
                #[cfg(feature = "fx")]
                fx.draw_curtain(ui, (*page_index + 1) as u32, tex.id(), avail);
            }
        } else {
            ui.painter().rect_filled(avail, 0.0, egui::Color32::from_gray(32));
            let label = if reader_loading.contains(page_index) {lang.loading} else {"📄"};
            ui.painter().text(avail.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(24.0), egui::Color32::GRAY,);
        }
    } else {
        if flip.is_active() {
            if flip.is_forward {
                let clip = egui::Rect::from_min_max(
                    avail.left_top(),
                    egui::pos2(split_x, avail.bottom()),
                );
                fit_texture_or_placeholder(ui, reader_cache, flip.back_page, avail);
                if guard_back { dim(ui, avail); }
                fit_texture_or_placeholder(ui, reader_cache, flip.front_page, clip);
                if guard_front { dim(ui, clip); }
            } else {
                let clip = egui::Rect::from_min_max(
                    egui::pos2(split_x, avail.top()),
                    avail.right_bottom(),
                );
                fit_texture_or_placeholder(ui, reader_cache, flip.back_page, avail);
                if guard_back { dim(ui, avail); }
                fit_texture_or_placeholder(ui, reader_cache, flip.front_page, clip);
                if guard_front { dim(ui, clip); }
            }
        } else if let Some(tex) = reader_cache.get(page_index) {
            #[cfg(feature = "fx")]
            let fx_drew = fx.draw_manga_page(ui, (*page_index + 1) as u32, tex.id(), avail);
            #[cfg(not(feature = "fx"))]
            let fx_drew = false;
            if !fx_drew {
                ui.painter().add(
                    egui::epaint::RectShape::filled(avail, 0.0, egui::Color32::WHITE)
                    .with_texture(tex.id(), egui::Rect::from_min_max(
                            egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0),
                    )),
                );
                #[cfg(feature = "fx")]
                fx.draw_curtain(ui, (*page_index + 1) as u32, tex.id(), avail);
            }
        } else {
            ui.painter().rect_filled(avail, 0.0, egui::Color32::from_gray(32));
            let label = if reader_loading.contains(page_index) {
                lang.loading
            } else {
                "📄"
            };
            ui.painter().text(
                avail.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(24.0),
                egui::Color32::GRAY,
            );
        }
    }

    if resp.clicked() && !alt && !flip.is_active() && !curtain_guard && !reveal_lock {
        let pointer = ui.ctx().pointer_interact_pos().unwrap_or(avail.center());
        let mid_x = avail.center().x;
        let cz = avail.width() * 0.1;
        if !(fs && (pointer.x - mid_x).abs() < cz) {
            if crate::reader::chapter_switch::is_prev_section(pointer.x, mid_x, rtl) && *page_index > 0 {
                trigger_flip(flip, *page_index, total, false);
            } else if crate::reader::chapter_switch::is_next_section(pointer.x, mid_x, rtl) && *page_index + 1 < total {
                trigger_flip(flip, *page_index, total, true);
            }
        }
    }

    let start = page_index.saturating_sub(settings.preload.single_before as usize);
    let end = (*page_index + settings.preload.single_after as usize).min(total);
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

fn fit_texture_or_placeholder(
    ui: &mut egui::Ui,
    cache: &HashMap<usize, egui::TextureHandle>,
    idx: usize,
    rect: egui::Rect,
) {
    if rect.width() <= 0.0 { return; }
    if let Some(tex) = cache.get(&idx) {
        ui.painter().add(
            egui::epaint::RectShape::filled(rect, 0.0, egui::Color32::WHITE)
                .with_texture(tex.id(), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))),
        );
    } else {
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(32));
    }
}

pub fn trigger_flip(flip: &mut PageFlipState, page_index: usize, total: usize, forward: bool) {
    if flip.is_active() { return; }
    let (target, back) = if forward {
        (page_index + 1, page_index + 1)
    } else {
        (page_index - 1, page_index - 1)
    };
    if back >= total { return; }
    flip.phase = FlipPhase::Animating(0.0);
    flip.from_page = page_index;
    flip.target_page = target;
    flip.front_page = page_index;
    flip.back_page = back;
    flip.step = 1;
    flip.is_forward = forward;
    flip.committing = true;
    flip.release_split = f32::NAN;
}
