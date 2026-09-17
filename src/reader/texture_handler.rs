use eframe::egui;
use crate::NavigationLevel;
use crate::config::settings::ReaderSettings;
use crate::database::MangaDb;
use crate::library_state::LibraryState;
use crate::reader::mode::ReaderMode;
use crate::reader::state::ReaderState;

const MAX_TEXTURES_PER_FRAME: u32 = 3;

impl ReaderState {
    pub fn handle_textures(&mut self, ctx: &egui::Context, settings: &ReaderSettings, lib: &mut LibraryState, db: &mut Option<MangaDb>) {
        let mut count = 0u32;
        while count < MAX_TEXTURES_PER_FRAME {
            let (ap, page_idx, rgba, w, h, full_res) = match self.reader_rx.try_recv() {
                Ok(v) => v,
                Err(_) => break,
            };
            count += 1;
            if w == 0 || h == 0 { continue; }

            if let Some(ref mut s) = self.reader_webtoon_state {
                let display_w = s.ref_width.max(settings.webtoon.min_content_width);
                let rh = display_w * h as f32 / w as f32;
                if ap == self.current_chapter.archive_path {
                    if page_idx < s.current_h.len() { s.current_h[page_idx] = rh; }
                    if page_idx < self.current_chapter.heights.len() { self.current_chapter.heights[page_idx] = rh; }
                } else if ap == self.prev_chapter.archive_path {
                    if page_idx < self.prev_chapter.heights.len() { self.prev_chapter.heights[page_idx] = rh; }
                } else if ap == self.joined_chapter.archive_path {
                    if page_idx < self.joined_chapter.heights.len() { self.joined_chapter.heights[page_idx] = rh; }
                }
            }

            let is_current = ap == self.current_chapter.archive_path;

            let mut pinned = [usize::MAX; 2];
            if let Some(NavigationLevel::Reading { page_index, .. }) = self.nav_stack.last() {
                pinned[0] = *page_index;
                let is_double = lib.manga_list.iter()
                    .find(|m| m.archives.iter().any(|a| *a == ap))
                    .map(|m| m.effective_mode() == ReaderMode::Double)
                    .unwrap_or(false);
                if is_double { pinned[1] = *page_index + 1; }
            }

            let bundle = if ap == self.prev_chapter.archive_path { &mut self.prev_chapter }
                else if ap == self.current_chapter.archive_path { &mut self.current_chapter }
                else if ap == self.joined_chapter.archive_path { &mut self.joined_chapter }
                else { continue; };

            bundle.loading.remove(&page_idx);
            if full_res {
                bundle.decode_full.insert(page_idx);
            } else {
                bundle.decode_full.remove(&page_idx);
            }
            if page_idx < bundle.aspect_ratios.len() {
                bundle.aspect_ratios[page_idx] = w as f32 / h as f32;
            }
            let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ctx.load_texture("reader", img, egui::TextureOptions::default());
            let size = w as usize * h as usize * 4;
            let budget = if settings.preload.texture_budget_mb > 0 {
                let is_webtoon = self.reader_webtoon_state.is_some();
                let mult = if is_webtoon { 2 } else { 1 };
                (settings.preload.texture_budget_mb as usize * mult) * 1024 * 1024
            } else {
                usize::MAX
            };
            bundle.cache_push(page_idx, tex, size, settings.preload.page_cache_max as usize, budget, pinned);

            let has_current = pinned[0] == usize::MAX || bundle.cache.contains_key(&pinned[0]);
            if is_current && has_current && bundle.cache.len() >= settings.reader.auto_detect_cache_threshold as usize {
                let dims: Vec<(u32, u32)> = bundle.cache.iter()
                    .map(|(_, tex)| { let s = tex.size(); (s[0] as u32, s[1] as u32) })
                    .collect();
                if let Some(manga) = lib.manga_list.iter_mut().find(|m| {
                    m.auto_mode.is_none() && m.archives.iter().any(|a| *a == ap)
                }) {
                    manga.auto_mode = Some(crate::reader::auto_detect::detect_from_pages(
                            &dims,
                            settings.webtoon.aspect_ratio_threshold,
                            settings.webtoon.auto_detect_majority_factor,
                    ));
                    if let Some(ref db) = *db {
                        let _ = db.save_manga_settings(&manga.directory, manga.manual_mode, manga.auto_mode, manga.manual_rtl, manga.auto_rtl);
                    }
                }
            }
        }
    }
}
