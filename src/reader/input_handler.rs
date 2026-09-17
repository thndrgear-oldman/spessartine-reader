use eframe::egui;
use crate::NavigationLevel;
use crate::ui::reader::{PageFlipState, page_flip_id};
use crate::reader::mode::ReaderMode;
use crate::config::settings::ReaderSettings;
use crate::database::MangaDb;
use crate::library_state::LibraryState;
use crate::reader::state::ReaderState;

impl ReaderState {
    pub fn handle_keyboard(&mut self, ctx: &egui::Context, settings: &ReaderSettings, lib: &LibraryState, db: &mut Option<MangaDb>, sidebar_visible: &mut bool) {
        if ctx.input(|i| i.modifiers.alt) { return; }

        let reading = matches!(self.nav_stack.last(),
            Some(&NavigationLevel::Reading { .. }) | Some(&NavigationLevel::ReadingWebtoon { .. }));

        if reading && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            let id = |name| egui::Id::new(("fs", name));
            let next = ctx.data_mut(|d| {
                let active = d.get_temp::<bool>(id("active")).unwrap_or(false);
                if !active {
                    d.insert_temp(id("backup_sidebar"), *sidebar_visible);
                    *sidebar_visible = false;
                    d.insert_temp(id("toolbar"), false);
                } else {
                    if let Some(restore) = d.get_temp::<bool>(id("backup_sidebar")) {
                        *sidebar_visible = restore;
                    }
                }
                let next = !active;
                d.insert_temp(id("active"), next);
                next
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(next));
        }
        if reading && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let id = |name| egui::Id::new(("fs", name));
            let was_active = ctx.data_mut(|d| d.get_temp::<bool>(id("active"))).unwrap_or(false);
            if was_active {
                ctx.data_mut(|d| {
                    if let Some(restore) = d.get_temp::<bool>(id("backup_sidebar")) {
                        *sidebar_visible = restore;
                    }
                    d.insert_temp(id("active"), false);
                    d.insert_temp(id("toolbar"), false);
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
        }

        #[cfg(feature = "fx")]
        if ctx.input(|i| i.key_pressed(egui::Key::I)) {
            self.toggle_fx(lib, settings);
            return;
        }

        if let Some(&mut NavigationLevel::Reading { ref mut page_index, ref mut dir_path, ref mut archive_path }) =
            self.nav_stack.last_mut()
        {
            let total = self.current_chapter.pages.len();
            let mode = lib.manga_list.iter()
                .find(|m| m.archives.iter().any(|a| *a == *archive_path))
                .map(|m| m.effective_mode())
                .unwrap_or(crate::reader::mode::ReaderMode::Manga);
            let rtl = lib.manga_list.iter()
                .find(|m| m.archives.iter().any(|a| *a == *archive_path))
                .map(|m| m.effective_rtl())
                .unwrap_or(false);
            let step = crate::reader::chapter_switch::page_step(mode);
            let is_last_chapter = {
                let ch_total = lib.chapters.len();
                if ch_total > 0 {
                    lib.chapters.iter()
                        .position(|c| c.path == *archive_path)
                        .map(|idx| idx + 1 >= ch_total)
                        .unwrap_or(false)
                } else { false }
            };
            let mut nav_dir: Option<bool> = None;

            let flip_active = ctx.data(|d| d.get_temp::<PageFlipState>(page_flip_id()))
                .map_or(false, |f| f.is_active());
            if flip_active {
                return;
            }

            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                #[cfg(feature = "fx")]
                {
                    crate::ui::reader::fx_hooks::update_manga_spread(
                        &mut self.fx_player,
                        *page_index,
                        total,
                        &self.current_chapter.aspect_ratios,
                        settings.reader.panorama_threshold,
                        rtl,
                        is_last_chapter,
                        mode == ReaderMode::Double,
                    );
                    if self.fx_player.interactive() && self.fx_player.reveal_next() {
                        return;
                    }
                }
                nav_dir = Some(true);
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                nav_dir = Some(false);
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                nav_dir = Some(true);
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                nav_dir = Some(if rtl { true } else { false });
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                nav_dir = Some(if rtl { false } else { true });
            }

            if let Some(forward) = nav_dir {
                let at_edge = if mode == ReaderMode::Webtoon {
                    false
                } else if forward {
                    if mode == ReaderMode::Double {
                        let vp = crate::reader::chapter_bundle::build_virtual_pages(
                            total, &self.current_chapter.aspect_ratios,
                            settings.reader.panorama_threshold, rtl, is_last_chapter);
                        crate::ui::reader::double_page::at_final_spread(&vp, *page_index)
                    } else {
                        *page_index + 1 >= total
                    }
                } else {
                    *page_index == 0
                };

                if at_edge {
                    if let Some((nap, ndp, npage)) = crate::reader::chapter_switch::take_switch(
                        &mut self.next_switch, &mut self.prev_switch,
                        &mut self.current_chapter, &mut self.prev_chapter, &mut self.joined_chapter,
                        &self.reader_tx, settings, forward)
                    {
                        *archive_path = nap;
                        *dir_path = ndp;
                        *page_index = npage;
                        #[cfg(feature = "fx")]
                        crate::ui::reader::fx_hooks::load_chapter(
                            &mut self.fx_player, lib, archive_path.as_str());
                    } else {
                        crate::reader::chapter_switch::request_switch(
                            &mut self.next_switch, &mut self.prev_switch,
                            lib, settings, archive_path, dir_path, forward);
                    }
                } else {
                    let from = *page_index;
                    match mode {
                        ReaderMode::Manga => {
                            ctx.data_mut(|d| {
                                let mut flip = d.get_temp::<PageFlipState>(page_flip_id())
                                    .unwrap_or(PageFlipState::new());
                                crate::ui::reader::trigger_single(&mut flip, from, total, forward);
                                d.insert_temp(page_flip_id(), flip);
                            });
                        }
                        ReaderMode::Double => {
                            ctx.data_mut(|d| {
                                let mut flip = d.get_temp::<PageFlipState>(page_flip_id())
                                    .unwrap_or(PageFlipState::new());
                                crate::ui::reader::double_page::trigger_flip_from_real(
                                    &mut flip, from, &self.current_chapter.aspect_ratios,
                                    settings.reader.panorama_threshold, rtl, is_last_chapter, forward);
                                d.insert_temp(page_flip_id(), flip);
                            });
                        }
                        _ => {
                            let new_idx = if forward {
                                if *page_index + step < total { Some(*page_index + step) } else { None }
                            } else if *page_index >= step {
                                Some(*page_index - step)
                            } else {
                                None
                            };
                            if let Some(new_idx) = new_idx {
                                *page_index = new_idx;
                                crate::audio::page_turn(settings.audio.muted, settings.audio.volume);
                                if let Some(ref db) = *db {
                                    let _ = db.save_progress(dir_path, archive_path, new_idx, total);
                                }
                                if !self.current_chapter.cache.contains_key(&new_idx)
                                    && !self.current_chapter.loading.contains(&new_idx)
                                {
                                    let page_name = self.current_chapter.pages[new_idx].name.clone();
                                    let tx = self.reader_tx.clone();
                                    let ap = archive_path.clone();
                                    let arcs = self.current_chapter.archive.clone();
                                    let max_decode = settings.reader.max_decode_width;
                                    self.current_chapter.loading.insert(new_idx);
                                    let _ = crate::reader::worker::decode_tx().send(
                                        crate::reader::worker::DecodeJob {
                                            ap: ap.clone(),
                                            n: new_idx,
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
            }
        }
    }
}
