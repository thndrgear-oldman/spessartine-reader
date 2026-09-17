use crate::reader::chapter_switch::Status;
use crate::NavigationLevel;
use crate::config::settings::ReaderSettings;
use crate::library_state::LibraryState;
use crate::reader::state::ReaderState;

impl ReaderState {
    pub fn preload_adjacent(&mut self, settings: &ReaderSettings, lib: &LibraryState) {
        if let Some(NavigationLevel::Reading { archive_path, page_index, .. }) = self.nav_stack.last() {
            let mode = lib.manga_list.iter()
                .find(|m| m.archives.iter().any(|a| *a == *archive_path))
                .map(|m| m.effective_mode())
                .unwrap_or(crate::reader::mode::ReaderMode::Manga);
            if mode != crate::reader::mode::ReaderMode::Webtoon {
                let total = self.current_chapter.pages.len();
                let ahead = settings.preload.reader_ahead as usize;
                for offset in 1..=ahead {
                    let next = *page_index + offset;
                    if next < total
                        && !self.current_chapter.cache.contains_key(&next)
                        && !self.current_chapter.loading.contains(&next)
                    {
                        let page_name = self.current_chapter.pages[next].name.clone();
                        let tx = self.reader_tx.clone();
                        let ap = archive_path.clone();
                        let arcs = self.current_chapter.archive.clone();
                        let max_decode = settings.reader.max_decode_width;
                        self.current_chapter.loading.insert(next);
                        let _ = crate::reader::worker::decode_tx().send(
                            crate::reader::worker::DecodeJob {
                                ap: ap.clone(),
                                n: next,
                                page_name: page_name.clone(),
                                max_decode_width: max_decode,
                                arcs,
                                tx,
                                use_max_width: false,
                            }
                        );
                    }
                }
            }
        }
    }

    pub fn preload_chapters(&mut self, settings: &ReaderSettings, lib: &LibraryState) {
        let (pi, dp, ap) = match self.nav_stack.last() {
            Some(&NavigationLevel::Reading { page_index, ref dir_path, ref archive_path }) => {
                (page_index, dir_path.clone(), archive_path.clone())
            }
            Some(&NavigationLevel::ReadingWebtoon { ref dir_path, ref archive_path }) => {
                (0, dir_path.clone(), archive_path.clone())
            }
            _ => (0, String::new(), String::new()),
        };

        let cur_ap = if !self.current_chapter.archive_path.is_empty() {
            &self.current_chapter.archive_path
        } else {
            &ap
        };

        let next_path = lib.chapter_idx.get(cur_ap)
            .copied()
            .and_then(|i| lib.chapters.get(i + 1))
            .map(|c| c.path.clone());

        let prev_path = if !self.prev_chapter.archive_path.is_empty() {
            lib.chapter_idx.get(&self.prev_chapter.archive_path)
                .copied()
                .and_then(|i| if i > 0 { lib.chapters.get(i - 1) } else { None })
                .map(|c| c.path.clone())
        } else {
            lib.chapter_idx.get(cur_ap)
                .copied()
                .and_then(|i| if i > 0 { lib.chapters.get(i - 1) } else { None })
                .map(|c| c.path.clone())
        };

        let in_reading = matches!(self.nav_stack.last(),
            Some(&NavigationLevel::Reading { .. }) | Some(&NavigationLevel::ReadingWebtoon { .. }));

        if in_reading {
            let preload_mode = if matches!(self.nav_stack.last(), Some(&NavigationLevel::ReadingWebtoon { .. })) {
                crate::reader::mode::ReaderMode::Webtoon
            } else {
                lib.manga_list.iter()
                    .find(|m| m.archives.iter().any(|a| *a == ap))
                    .map(|m| m.effective_mode())
                    .unwrap_or(crate::reader::mode::ReaderMode::Manga)
            };

            if preload_mode == crate::reader::mode::ReaderMode::Webtoon {
                let ne = self.reader_webtoon_state.as_ref().map_or(false, |s| s.near_end);
                let ns = self.reader_webtoon_state.as_ref().map_or(false, |s| s.near_start);

                if ne && self.joined_chapter.pages.is_empty() {
                    if let Some(ref np) = next_path {
                        if matches!(self.next_switch.status, Status::Idle) {
                            self.next_switch.trigger(
                                np, &dp,
                                self.reader_webtoon_state.as_ref().map_or(settings.webtoon.default_ref_width, |s| s.ref_width),
                                settings.reader.dimension_header_read_size,
                                settings.webtoon.default_page_height * 0.0 + 1.0,
                                settings.webtoon.default_page_height,
                            );
                        }
                    }
                }

                if ns && self.prev_chapter.pages.is_empty() {
                    if let Some(ref pp) = prev_path {
                        if matches!(self.prev_switch.status, Status::Idle) {
                            self.prev_switch.trigger_prev(
                                pp, &dp,
                                self.reader_webtoon_state.as_ref().map_or(settings.webtoon.default_ref_width, |s| s.ref_width),
                                settings.reader.dimension_header_read_size,
                                settings.webtoon.default_page_height * 0.0 + 1.0,
                                settings.webtoon.default_page_height,
                            );
                        }
                    }
                }

                if let Some(data) = self.next_switch.take_ready() {
                    self.joined_chapter.pages = data.pages;
                    self.joined_chapter.archive = Some(data.shared);
                    self.joined_chapter.archive_path = data.archive_path;
                    self.joined_chapter.dir_path = data.dir_path;
                    self.joined_chapter.heights = data.heights;
                    self.joined_chapter.aspect_ratios = data.aspect_ratios;
                    if let Some(ref mut s) = self.reader_webtoon_state {
                        s.joined_h = self.joined_chapter.heights.clone();
                    }
                }

                if let Some(data) = self.prev_switch.take_ready() {
                    self.prev_chapter.pages = data.pages;
                    self.prev_chapter.archive = Some(data.shared);
                    self.prev_chapter.archive_path = data.archive_path;
                    self.prev_chapter.dir_path = data.dir_path;
                    self.joined_chapter.clear();
                    self.prev_chapter.heights = data.heights;
                    self.prev_chapter.aspect_ratios = data.aspect_ratios;
                    let spacing = settings.webtoon.page_spacing as f32;
                    if let Some(ref mut s) = self.reader_webtoon_state {
                        s.prev_h = self.prev_chapter.heights.clone();
                        let prev_total = s.prev_h.iter().sum::<f32>()
                            + s.prev_h.len() as f32 * spacing;
                        s.frame_offset += prev_total;
                    }
                }

                self.next_switch.poll();
                self.prev_switch.poll();
            } else {
                let manga_prev_path = lib.chapter_idx.get(cur_ap)
                    .copied()
                    .and_then(|i| if i > 0 { lib.chapters.get(i - 1) } else { None })
                    .map(|c| c.path.clone());

                if let Some(np) = next_path {
                    if np != self.joined_chapter.archive_path
                        && np != self.current_chapter.archive_path
                        && matches!(self.next_switch.status, Status::Idle)
                        && crate::reader::chapter_switch::is_at_chapter_end(pi, self.current_chapter.pages.len(), preload_mode)
                    {
                        self.next_switch.trigger(
                            &np, &dp,
                            self.reader_webtoon_state.as_ref().map_or(settings.webtoon.default_ref_width, |s| s.ref_width),
                            settings.reader.dimension_header_read_size,
                            settings.webtoon.default_page_height * 0.0 + 1.0,
                            settings.webtoon.default_page_height,
                        );
                    }
                }

                if let Some(pp) = manga_prev_path {
                    if pp != self.prev_chapter.archive_path
                        && pp != self.current_chapter.archive_path
                        && matches!(self.prev_switch.status, Status::Idle)
                        && crate::reader::chapter_switch::is_at_chapter_start(pi)
                    {
                        self.prev_switch.trigger_prev(
                            &pp, &dp,
                            self.reader_webtoon_state.as_ref().map_or(settings.webtoon.default_ref_width, |s| s.ref_width),
                            settings.reader.dimension_header_read_size,
                            settings.webtoon.default_page_height * 0.0 + 1.0,
                            settings.webtoon.default_page_height,
                        );
                    }
                }

                self.next_switch.poll();
                self.prev_switch.poll();
            }
        }
    }
}
