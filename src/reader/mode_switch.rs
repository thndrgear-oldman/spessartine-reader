use eframe::egui;
use crate::config::settings::ReaderSettings;
use crate::reader::mode::ReaderMode;
use crate::reader::state::ReaderState;

impl ReaderState {
    pub fn switch_mode(&mut self, ctx: &egui::Context, settings: &mut ReaderSettings, pending: &mut Option<(u64, Option<ReaderMode>)>) {
        if let Some((_id, Some(mode))) = pending.take() {
            match mode {
                crate::reader::mode::ReaderMode::Webtoon => {
                    let pop_data = if let Some(&mut crate::NavigationLevel::Reading { ref archive_path, ref dir_path, page_index }) = self.nav_stack.last_mut() {
                        Some((archive_path.clone(), dir_path.clone(), page_index))
                    } else { None };
                    if let Some((ap, dp, seek_pi)) = pop_data {
                        self.current_chapter.cache_clear();
                        self.prev_chapter.cache_clear();
                        self.joined_chapter.cache_clear();
                        while self.reader_rx.try_recv().is_ok() {}
                        self.nav_stack.pop();
                        self.nav_stack.push(crate::NavigationLevel::ReadingWebtoon { archive_path: ap.clone(), dir_path: dp });
                        let rw = if settings.webtoon_width > 0.0 {
                            settings.webtoon_width
                        } else {
                            settings.webtoon.default_ref_width
                        };
                        self.reader_webtoon_state = Some(crate::ui::webtoon_reader::state::WebtoonState::new(rw));
                        ctx.data_mut(|d| d.insert_persisted(
                            egui::Id::new(("webtoon_seek", ap.as_str())),
                            seek_pi,
                        ));
                    }
                }
                _ => {
                    let pop_data = if let Some(&mut crate::NavigationLevel::ReadingWebtoon { ref archive_path, ref dir_path }) = self.nav_stack.last_mut() {
                        let pi = self.reader_webtoon_state.as_ref().map(|s| {
                            if s.current_h.is_empty() { return 0; }
                            let spacing = settings.webtoon.page_spacing as f32;
                            let prev_total = s.prev_h.iter().sum::<f32>() + s.prev_h.len() as f32 * spacing;
                            let offset = (s.frame_offset.max(0.0) - prev_total).max(0.0);
                            let mut cum = vec![0.0];
                            cum.extend(s.current_h.iter().scan(0.0, |a, &h| { *a += h + spacing; Some(*a) }));
                            crate::ui::webtoon_reader::state::current_index(offset, &cum)
                        }).unwrap_or(0);
                        Some((archive_path.clone(), dir_path.clone(), pi))
                    } else { None };
                    if let Some((ap, dp, pi)) = pop_data {
                        self.nav_stack.pop();
                        self.nav_stack.push(crate::NavigationLevel::Reading { archive_path: ap, page_index: pi, dir_path: dp });
                        if let Some(ref s) = self.reader_webtoon_state {
                            settings.webtoon_width = s.ref_width;
                            settings.save();
                        }
                        self.reader_webtoon_state = None;
                    }
                }
            }
        }
    }
}
