use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::audio;
use crate::audio::Channel;

use super::curtain::curtain_fill;
use super::resolve::resolve;
use super::soundmap::{
    title_spfx_path_for, Cut, Frame, FxCue, SoundMap, TitleMap, TitleMode,
};
use super::{
    build_cut_mesh_deformed, build_cut_mirror_mesh, build_page_mesh, draw as fx_draw,
    ease_out_cubic, page_deformed, FxState,
};

const CURTAIN_DIM: f32 = 0.05;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub real: usize,
    pub frame: Option<usize>,
    pub placeholder: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SlotInfo {
    pub key: u32,
    pub count: usize,
}

#[derive(Default)]
pub struct FxPlayer {
    mode: TitleMode,
    map: Option<SoundMap>,
    interactive: bool,
    fx_debug: bool,
    state: FxState,
    cur_spread: Option<u64>,
    cur_pieces: Vec<Piece>,
    progress: HashMap<u64, usize>,
    reveal_at: HashMap<u64, Instant>,
    reveal_back: HashMap<u64, bool>,
    page_audio_played: HashSet<u32>,
    webtoon_seen: HashSet<(u32, u8, usize)>,
    webtoon_fx_armed: HashMap<u32, HashSet<usize>>,
    last_page_keys: Vec<u32>,
    cur_slot_info: Vec<SlotInfo>,
    spread_pieces: HashMap<u64, usize>,
}

impl FxPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(&self) -> TitleMode {
        self.mode
    }

    pub fn interactive(&self) -> bool {
        self.interactive
    }

    pub fn curtain_dim(&self) -> f32 {
        CURTAIN_DIM
    }

    pub fn cur_spread(&self) -> Option<u64> {
        self.cur_spread
    }

    pub fn pieces_for(&self, real: usize) -> Vec<Piece> {
        let key = real as u32 + 1;
        let Some(pm) = self.map.as_ref().and_then(|m| m.pages.get(&key)) else {
            return vec![Piece { real, frame: None, placeholder: false }];
        };
        let seq = frame_seq(&pm.frames);
        if seq.is_empty() {
            // нет карта/кадра (или все кадры отключены) — цельная страница
            return vec![Piece { real, frame: None, placeholder: false }];
        }
        seq.into_iter()
            .map(|fi| Piece { real, frame: Some(fi), placeholder: false })
            .collect()
    }

    pub fn pieces_pano(&self, real: usize) -> Vec<Piece> {
        let key = real as u32 + 1;
        let Some(pm) = self.map.as_ref().and_then(|m| m.pages.get(&key)) else {
            return vec![Piece { real, frame: None, placeholder: false }];
        };
        let seq = frame_seq(&pm.frames);
        if seq.is_empty() {
            return vec![Piece { real, frame: None, placeholder: false }];
        }
        seq.into_iter()
            .map(|fi| Piece { real, frame: Some(fi), placeholder: false })
            .collect()
    }

    pub fn set_debug(&mut self, on: bool) {
        self.fx_debug = on;
    }

    pub fn open(&mut self, chapter_path: &str, chapter_num: u32) -> Result<(), String> {
        let spfx = title_spfx_path_for(chapter_path);
        let tm = TitleMap::load(&spfx)
            .map_err(|e| { audio::stop_all(); e })?;
        let sm = tm
            .chapters
            .get(&chapter_num)
            .cloned()
            .ok_or_else(|| {
                audio::stop_all();
                format!("главы {} нет в {}", chapter_num, spfx.display())
            })?;
        let page1 = sm.pages.get(&1);
        let has_bgm = page1
            .and_then(|p| p.bgm.as_ref())
            .is_some_and(|st| st.tracks.iter().filter_map(|t| resolve(t)).next().is_some());
        let has_ambient = page1
            .and_then(|p| p.ambient.as_ref())
            .is_some_and(|st| st.tracks.iter().filter_map(|t| resolve(t)).next().is_some());
        audio::stop_frame_and_sfx();
        if has_bgm {
            audio::stop_playlist(Channel::Bgm, 0.0);
        }
        if has_ambient {
            audio::stop_playlist(Channel::Ambient, 0.0);
        }
        self.map = Some(sm);
        self.mode = tm.mode;
        self.state.reset();
        self.progress.clear();
        self.reveal_at.clear();
        self.reveal_back.clear();
        self.page_audio_played.clear();
        self.webtoon_seen.clear();
        self.webtoon_fx_armed.clear();
        self.cur_pieces.clear();
        self.cur_spread = None;
        self.last_page_keys.clear();
        self.cur_slot_info.clear();
        self.spread_pieces.clear();
        Ok(())
    }

    pub fn current_spread(&mut self, spread: u64, pieces: &[Piece]) {
        let changed = self.cur_spread != Some(spread)
            || self.cur_pieces.len() != pieces.len()
            || !self.cur_pieces.iter().eq(pieces);
        if changed {
            self.cur_pieces = pieces.to_vec();
            if pieces.is_empty() {
                self.progress.insert(spread, 0);
            }
        }
        self.cur_spread = Some(spread);
        self.progress.entry(spread).or_insert(0);
        self.spread_pieces.insert(spread, pieces.len());
        self.last_page_keys = self
            .cur_pieces
            .iter()
            .filter(|p| !p.placeholder)
            .map(|p| p.real as u32 + 1)
            .collect();
    }

    pub fn set_interactive(&mut self, on: bool, cur_spread: Option<u64>) {
        if on {
            if let Some(sp) = cur_spread {
                for (&k, v) in self.progress.iter_mut() {
                    if k < sp {
                        if let Some(&total) = self.spread_pieces.get(&k) {
                            *v = total;
                        }
                    }
                }
                self.progress.retain(|&k, _| k <= sp);
                self.progress.insert(sp, 0);
                self.reveal_at.retain(|&k, _| k < sp);
                self.reveal_back.retain(|&k, _| k < sp);
            }
            self.interactive = true;
        } else {
            self.interactive = false;
        }
    }

    pub fn revealed(&self, spread: u64) -> usize {
        self.progress.get(&spread).copied().unwrap_or(0)
    }

    pub fn total_pieces(&self) -> usize {
        self.cur_pieces.len()
    }

    pub fn set_slot_info(&mut self, info: Vec<SlotInfo>) {
        self.cur_slot_info = info;
    }

    pub fn slot_count(&self, slot_index: usize) -> usize {
        self.cur_slot_info.get(slot_index).map(|s| s.count).unwrap_or(0)
    }

    pub fn slot_reveal(&self, slot_index: usize) -> usize {
        let Some(sp) = self.cur_spread else {
            return usize::MAX;
        };
        let rev = self.progress.get(&sp).copied().unwrap_or(0);
        let mut start = 0;
        for (i, s) in self.cur_slot_info.iter().enumerate() {
            if i == slot_index {
                return rev.saturating_sub(start);
            }
            start += s.count;
        }
        usize::MAX
    }

    pub fn slot_covered(&self, slot_index: usize) -> bool {
        let Some(sp) = self.cur_spread else {
            return false;
        };
        let rev = self.progress.get(&sp).copied().unwrap_or(0);
        let mut end = 0;
        for (i, s) in self.cur_slot_info.iter().enumerate() {
            end += s.count;
            if i == slot_index {
                let total = self.cur_pieces.len();
                return rev < end.min(total);
            }
        }
        false
    }

    pub fn spread_covered_any(&self, spread: u64) -> bool {
        let total = self.spread_pieces.get(&spread).copied().unwrap_or(usize::MAX);
        if total == 0 {
            return false;
        }
        let rev = self.progress.get(&spread).copied().unwrap_or(0);
        rev < total
    }

    pub fn reveal_alpha(&self) -> f32 {
        let Some(sp) = self.cur_spread else { return 1.0 };
        match self.reveal_at.get(&sp) {
            Some(t0) => {
                let tt = (t0.elapsed().as_secs_f32() / 0.3).clamp(0.0, 1.0);
                if self.reveal_back.get(&sp) == Some(&true) {
                    ease_out_cubic(1.0 - tt)
                } else {
                    ease_out_cubic(tt)
                }
            }
            None => 1.0,
        }
    }

    pub fn has_curtain(&self) -> bool {
        match self.cur_spread {
            Some(sp) => {
                !self.cur_pieces.is_empty()
                    && self.progress.get(&sp).copied().unwrap_or(0) < self.cur_pieces.len()
            }
            None => false,
        }
    }

    pub fn is_revealing(&self) -> bool {
        match self.cur_spread {
            Some(sp) => self
                .reveal_at
                .get(&sp)
                .map_or(false, |t0| t0.elapsed().as_secs_f32() < 0.3),
            None => false,
        }
    }

    pub fn reveal_next(&mut self) -> bool {
        let Some(sp) = self.cur_spread else { return false };
        let rev = self.progress.get(&sp).copied().unwrap_or(0);
        if rev >= self.cur_pieces.len() {
            return false;
        }
        self.progress.insert(sp, rev + 1);
        self.reveal_at.insert(sp, Instant::now());
        self.reveal_back.insert(sp, false);

        let piece = self.cur_pieces[rev];
        if piece.placeholder {
            return true;
        }
        let key = piece.real as u32 + 1;
        let Some(pm) = self.map.as_ref().and_then(|m| m.pages.get(&key)) else {
            return true;
        };
        if let Some(fi) = piece.frame {
            self.state.trigger_frame_oneshot(key, fi);
            if let Some(pl) = pm.frames.get(fi).and_then(|f| f.playlist.clone()) {
                let resolved: Vec<_> = pl.tracks.iter().filter_map(|t| resolve(t)).collect();
                if !resolved.is_empty() {
                    audio::play_frame(&resolved, pl.crossfade);
                }
            }
        }
        if !self.page_audio_played.contains(&key) && self.is_page_first_piece(rev) {
            self.page_audio_played.insert(key);
            self.apply_page_audio(key);
        }
        true
    }

    pub fn reveal_prev(&mut self) -> bool {
        let Some(sp) = self.cur_spread else { return false };
        let rev = self.progress.get(&sp).copied().unwrap_or(0);
        if rev == 0 {
            return false;
        }
        self.progress.insert(sp, rev - 1);
        self.reveal_at.insert(sp, Instant::now());
        self.reveal_back.insert(sp, true);
        true
    }

    fn is_page_first_piece(&self, p: usize) -> bool {
        let piece = self.cur_pieces[p];
        self.cur_pieces[..p].iter().all(|q| q.real != piece.real)
    }

    fn apply_page_audio(&self, key: u32) {
        let Some(pm) = self.map.as_ref().and_then(|m| m.pages.get(&key)) else {
            return;
        };
        if let Some(bgm) = &pm.bgm {
            let resolved: Vec<_> = bgm.tracks.iter().filter_map(|t| resolve(t)).collect();
            if !resolved.is_empty() {
                audio::play_playlist(Channel::Bgm, &resolved, bgm.volume, bgm.crossfade);
            }
        }
        if let Some(amb) = &pm.ambient {
            let resolved: Vec<_> = amb.tracks.iter().filter_map(|t| resolve(t)).collect();
            if !resolved.is_empty() {
                audio::play_playlist(Channel::Ambient, &resolved, amb.volume, amb.crossfade);
            }
        }
    }

    pub fn draw_manga_page(
        &mut self,
        ui: &mut egui::Ui,
        key: u32,
        tex: egui::TextureId,
        rect: egui::Rect,
    ) -> bool {
        let Some(pm) = self.map.as_ref().and_then(|m| m.pages.get(&key)) else {
            return false;
        };
        let t = self.state.elapsed_view();

        let page_cues: Vec<FxCue> = pm
            .fx
            .iter()
            .filter(|c| c.cut_idx.is_none())
            .cloned()
            .collect();
        let mirror_holes: Vec<Vec<[f32; 2]>> = pm
            .cuts
            .iter()
            .filter(|c| c.fill_mode == 2 && c.poly.len() >= 3)
            .map(|c| c.poly.clone())
            .collect();
        if page_deformed(&page_cues) {
            let mesh = build_page_mesh(
                tex,
                rect,
                &page_cues,
                t,
                key,
                &self.state.oneshot_frame_at,
                self.fx_debug,
                &mut self.state.page_mesh_cache,
            );
            ui.painter().add(egui::Shape::Mesh(mesh.into()));
        } else if !mirror_holes.is_empty() {
            curtain_fill(&ui.painter(), tex, rect, &[], &mirror_holes, egui::Color32::WHITE, 1.0);
        } else {
            ui.painter().add(
                egui::epaint::RectShape::filled(rect, 0.0, egui::Color32::WHITE)
                    .with_texture(tex, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))),
            );
        }

        let mut sorted: Vec<(usize, &Cut)> = pm.cuts.iter().enumerate().collect();
        sorted.sort_by_key(|(_, c)| c.z_order);
        for (ci, cut) in sorted {
            if cut.poly.len() < 3 {
                continue;
            }
            let screen_pts: Vec<egui::Pos2> = cut
                .poly
                .iter()
                .map(|p| rect.min + egui::vec2(p[0] * rect.width(), p[1] * rect.height()))
                .collect();
            match cut.fill_mode {
                1 => {
                    if let Some([r, g, b, a]) = cut.fill_color {
                        let color = egui::Color32::from_rgba_unmultiplied(
                            (r * 255.0) as u8,
                            (g * 255.0) as u8,
                            (b * 255.0) as u8,
                            (a * 255.0) as u8,
                        );
                        ui.painter().add(egui::Shape::Path(
                            egui::epaint::PathShape::convex_polygon(
                                screen_pts,
                                color,
                                egui::Stroke::NONE,
                            ),
                        ));
                    }
                }
                2 => {
                    let mesh = build_cut_mirror_mesh(tex, rect, cut);
                    if !mesh.vertices.is_empty() {
                        ui.painter().add(egui::Shape::Mesh(mesh.into()));
                    }
                }
                _ => {}
            }
            let cut_cues: Vec<FxCue> = pm
                .fx
                .iter()
                .filter(|c| c.cut_idx == Some(ci))
                .cloned()
                .collect();
            if !cut_cues.is_empty() {
                let mesh = build_cut_mesh_deformed(
                    tex,
                    rect,
                    cut,
                    &cut_cues,
                    t,
                    key,
                    &self.state.oneshot_frame_at,
                );
                ui.painter().add(egui::Shape::Mesh(mesh.into()));
            }
        }

        fx_draw(&mut self.state, ui, rect, &pm.fx, key, t);

        if self.interactive {
            self.curtain_for_page(ui, key, tex, rect);
        }

        true
    }

    fn curtain_for_page(&mut self, ui: &egui::Ui, key: u32, tex: egui::TextureId, rect: egui::Rect) {
        let Some(sp) = self.cur_spread else { return };
        let total_rev = self.progress.get(&sp).copied().unwrap_or(0);

        let mut first = None;
        let mut cnt = 0usize;
        for (i, piece) in self.cur_pieces.iter().enumerate() {
            if !piece.placeholder && piece.real as u32 + 1 == key {
                if first.is_none() {
                    first = Some(i);
                }
                cnt += 1;
            }
        }
        let Some(first) = first else { return };
        if cnt == 0 {
            return;
        }
        let n = total_rev.saturating_sub(first).min(cnt);
        if n == 0 {
            let d = (255.0 * CURTAIN_DIM).clamp(0.0, 255.0) as u8;
            let dim = egui::Color32::from_rgba_unmultiplied(d, d, d, 255);
            curtain_fill(&ui.painter(), tex, rect, &[], &[], dim, 1.0);
            return;
        }

        let active_here = first + n - 1 == total_rev - 1;
        let revealing = self.is_revealing();
        if n >= cnt && !(active_here && revealing) {
            return;
        }

        let pm = self.map.as_ref().and_then(|m| m.pages.get(&key));
        let poly_of = |fi: usize| -> Option<Vec<[f32; 2]>> {
            pm.and_then(|pm| pm.frames.get(fi))
                .and_then(|f| f.poly.clone())
                .filter(|p| p.len() >= 3)
        };
        let whole = vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];

        let mut hole: Vec<Vec<[f32; 2]>> = Vec::new();
        for k in 0..n {
            match self.cur_pieces[first + k].frame {
                Some(fi) => {
                    if let Some(poly) = poly_of(fi) {
                        hole.push(poly);
                    } else {
                        hole.push(whole.clone());
                    }
                }
                None => hole.push(whole.clone()),
            }
        }
        let active_poly: Option<Vec<[f32; 2]>> = match self.cur_pieces[first + n - 1].frame {
            Some(fi) => poly_of(fi).or(Some(whole)),
            None => Some(whole),
        };

        let d = (255.0 * CURTAIN_DIM).clamp(0.0, 255.0) as u8;
        let dim = egui::Color32::from_rgba_unmultiplied(d, d, d, 255);

        curtain_fill(&ui.painter(), tex, rect, &[], &hole, dim, 1.0);

        let dur = 0.3;
        let a = match self.reveal_at.get(&sp) {
            Some(t0) => {
                let tt = (t0.elapsed().as_secs_f32() / dur).clamp(0.0, 1.0);
                if self.reveal_back.get(&sp) == Some(&true) {
                    ease_out_cubic(1.0 - tt)
                } else {
                    ease_out_cubic(tt)
                }
            }
            None => 1.0,
        };
        let scan_a = 1.0 - a;
        if let Some(poly) = active_poly {
            curtain_fill(&ui.painter(), tex, rect, &[poly], &[], dim, scan_a);
        }
    }

    pub fn draw_curtain(&mut self, ui: &egui::Ui, key: u32, tex: egui::TextureId, rect: egui::Rect) {
        if self.interactive {
            self.curtain_for_page(ui, key, tex, rect);
        }
    }

    pub fn webtoon_poll(&mut self, pages: &[(u32, f32, f32)], view_top: f32, view_bottom: f32) {
        let Some(m) = &self.map else { return };
        self.last_page_keys.clear();
        for &(key, top, bottom) in pages {
            let Some(pm) = m.pages.get(&key) else { continue };
            let h = bottom - top;
            let top_n = if h > 0.0 {
                (view_top - top) / h
            } else {
                0.0
            };
            let bot_n = if h > 0.0 {
                (view_bottom - top) / h
            } else {
                1.0
            };
            if top_n <= 1.0 && bot_n >= 0.0 {
                self.last_page_keys.push(key);
            }

            for (ci, c) in pm.fx.iter().enumerate() {
                if !c.oneshot.unwrap_or(false) {
                    continue;
                }
                let (s, e) = cue_y_range(c);
                let in_view = s <= bot_n && e >= top_n;
                let armed = self.webtoon_fx_armed.entry(key).or_default();
                if in_view {
                    if !armed.contains(&ci) {
                        armed.insert(ci);
                        self.state.trigger_page_oneshot(key);
                    }
                } else {
                    armed.remove(&ci);
                }
            }

            for (i, a) in pm.actions.iter().enumerate() {
                let in_view = norm_range(a.y_start, a.y_end)
                    .map_or(false, |(s, e)| s <= bot_n && e >= top_n);
                let k = (key, 0u8, i);
                if in_view {
                    if !self.webtoon_seen.contains(&k) {
                        self.webtoon_seen.insert(k);
                        if let Some(p) = resolve(&a.file) {
                            audio::play_sfx(i, &p);
                        }
                    }
                } else {
                    self.webtoon_seen.remove(&k);
                }
            }

            if let Some(track) = &pm.bgm {
                let in_view =
                    norm_range(pm.bgm_y_start, pm.bgm_y_end).map_or(true, |(s, e)| s <= bot_n && e >= top_n);
                if in_view {
                    let k = (key, 1u8, 0);
                    if !self.webtoon_seen.contains(&k) {
                        self.webtoon_seen.insert(k);
                        let resolved: Vec<_> = track.tracks.iter().filter_map(|t| resolve(t)).collect();
                        if !resolved.is_empty() {
                            audio::play_playlist(Channel::Bgm, &resolved, track.volume, track.crossfade);
                        }
                    }
                }
            }

            if let Some(track) = &pm.ambient {
                let in_view =
                    norm_range(pm.background_y_start, pm.background_y_end)
                        .map_or(true, |(s, e)| s <= bot_n && e >= top_n);
                if in_view {
                    let k = (key, 2u8, 0);
                    if !self.webtoon_seen.contains(&k) {
                        self.webtoon_seen.insert(k);
                        let resolved: Vec<_> = track.tracks.iter().filter_map(|t| resolve(t)).collect();
                        if !resolved.is_empty() {
                            audio::play_playlist(Channel::Ambient, &resolved, track.volume, track.crossfade);
                        }
                    }
                }
            }
        }
    }

    pub fn draw_webtoon_page(
        &mut self,
        ui: &mut egui::Ui,
        key: u32,
        tex: egui::TextureId,
        rect: egui::Rect,
    ) -> bool {
        let Some(pm) = self.map.as_ref().and_then(|m| m.pages.get(&key)) else {
            return false;
        };
        let t = self.state.elapsed_view();
        let page_cues: Vec<FxCue> = pm
            .fx
            .iter()
            .filter(|c| c.cut_idx.is_none())
            .cloned()
            .collect();

        if page_deformed(&page_cues) {
            let mesh = build_page_mesh(
                tex,
                rect,
                &page_cues,
                t,
                key,
                &self.state.oneshot_frame_at,
                self.fx_debug,
                &mut self.state.page_mesh_cache,
            );
            ui.painter().add(egui::Shape::Mesh(mesh.into()));
        } else {
            ui.painter().add(
                egui::epaint::RectShape::filled(rect, 0.0, egui::Color32::WHITE)
                    .with_texture(tex, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))),
            );
        }
        fx_draw(&mut self.state, ui, rect, &pm.fx, key, t);
        true
    }

    pub fn active(&self) -> bool {
        if self.is_revealing() {
            return true;
        }
        if let Some(m) = &self.map {
            for &k in &self.last_page_keys {
                if let Some(pm) = m.pages.get(&k) {
                    if !pm.fx.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn cue_y_range(c: &FxCue) -> (f32, f32) {
    if let Some(poly) = &c.area {
        if poly.len() >= 3 {
            let mut s = 1.0f32;
            let mut e = 0.0f32;
            for pt in poly {
                s = s.min(pt[1]);
                e = e.max(pt[1]);
            }
            return (s, e);
        }
    }
    if let Some(z) = c.zone {
        return (z.y, (z.y + z.h).clamp(z.y, 1.0));
    }
    (0.0, 1.0)
}

fn norm_range(s: Option<f32>, e: Option<f32>) -> Option<(f32, f32)> {
    if s.is_none() && e.is_none() {
        return None;
    }
    Some((s.unwrap_or(0.0), e.unwrap_or(1.0)))
}


fn frame_seq(frames: &[Frame]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..frames.len()).filter(|&i| frames[i].enabled).collect();
    idx.sort_by(|&a, &b| frames[a].order.cmp(&frames[b].order));
    idx
}
