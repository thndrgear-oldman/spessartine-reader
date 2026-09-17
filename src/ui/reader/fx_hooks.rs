#![cfg(feature = "fx")]

use crate::fx::fx_player::{FxPlayer, Piece, SlotInfo};
use crate::library_state::LibraryState;
use crate::reader::chapter_bundle::VirtualPage;

pub fn load_chapter(fx: &mut FxPlayer, lib: &LibraryState, chapter_path: &str) {
    let chapter_num = lib
        .chapters
        .iter()
        .position(|c| c.path == *chapter_path)
        .map(|i| i as u32 + 1)
        .unwrap_or(1);
    let _ = fx.open(chapter_path, chapter_num);
}

fn slot_pieces(pl: &FxPlayer, real: usize, is_pano: bool) -> Vec<Piece> {
    if is_pano {
        pl.pieces_pano(real)
    } else {
        pl.pieces_for(real)
    }
}

pub fn update_manga_spread(
    pl: &mut FxPlayer,
    page_index: usize,
    real_total: usize,
    aspect_ratios: &[f32],
    pano_thr: f32,
    rtl: bool,
    is_last_chapter: bool,
    mode_use_double: bool,
) -> (u64, usize) {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut slots: Vec<SlotInfo> = Vec::new();
    let spread: u64;
    if mode_use_double {
        let vp = crate::reader::chapter_bundle::build_virtual_pages(
            real_total,
            aspect_ratios,
            pano_thr,
            rtl,
            is_last_chapter,
        );
        let cur_v = crate::ui::reader::double_page::spread_start_for(&vp, page_index);
        spread = cur_v as u64;
        let even = cur_v & !1;
        let mut i = 0;
        while i < 2 {
            let vi = even + i;
            match vp.get(vi) {
                Some(VirtualPage::Real(r)) => {
                    let sp = slot_pieces(pl, *r, false);
                    slots.push(SlotInfo { key: *r as u32 + 1, count: sp.len() });
                    pieces.extend(sp);
                }
                Some(VirtualPage::PanoramaLeft(r)) | Some(VirtualPage::PanoramaRight(r)) => {
                    let sp = slot_pieces(pl, *r, true);
                    slots.push(SlotInfo { key: *r as u32 + 1, count: sp.len() });
                    pieces.extend(sp);
                    i += 1;
                }
                _ => {
                    slots.push(SlotInfo { key: vi as u32, count: 1 });
                    pieces.push(Piece { real: usize::MAX, frame: None, placeholder: true });
                }
            }
            i += 1;
        }
    } else {
        let is_pano = aspect_ratios.get(page_index).copied().unwrap_or(0.0) > pano_thr;
        spread = page_index as u64;
        let sp = slot_pieces(pl, page_index, is_pano);
        slots.push(SlotInfo { key: page_index as u32 + 1, count: sp.len() });
        pieces.extend(sp);
    }
    pl.current_spread(spread, &pieces);
    pl.set_slot_info(slots);
    (spread, pieces.len())
}
