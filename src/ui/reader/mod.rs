use eframe::egui;
use crate::reader::chapter_switch::Status;

pub mod placeholders;
pub mod mode_badge;
pub mod single_page;
pub mod double_page;
pub mod reader_context_menu;
pub mod seeker;
#[cfg(feature = "fx")]
pub mod fx_hooks;

pub use single_page::trigger_flip as trigger_single;

#[derive(Clone, Copy, Debug)]
pub enum FlipPhase {
    Idle,
    Dragging,
    Animating(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct PageFlipState {
    pub phase: FlipPhase,
    pub step: usize,
    pub from_page: usize,
    pub target_page: usize,
    pub target_real: usize,
    pub front_page: usize,
    pub back_page: usize,
    pub start_x: f32,
    pub last_px: f32,
    pub is_forward: bool,
    pub committing: bool,
    pub release_split: f32,
}

impl PageFlipState {
    pub fn new() -> Self {
        Self { phase: FlipPhase::Idle, step: 1, from_page: 0, target_page: 0, target_real: 0, front_page: 0, back_page: 0, start_x: 0.0, last_px: 0.0, is_forward: true, committing: false, release_split: 0.0 }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.phase, FlipPhase::Idle)
    }

    pub fn commit(&mut self, split: f32) {
        self.release_split = split;
        self.committing = true;
        self.phase = FlipPhase::Animating(0.0);
    }

    pub fn abort(&mut self, split: f32) {
        self.release_split = split;
        self.committing = false;
        self.phase = FlipPhase::Animating(0.0);
    }
}

pub fn page_flip_id() -> egui::Id { egui::Id::new("page_flip") }

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    if let Some(crate::NavigationLevel::ReadingWebtoon { archive_path, dir_path }) = app.reader.nav_stack.last() {
        let ap = archive_path.clone();
        let dp = dir_path.clone();
        crate::ui::webtoon_reader::draw(app, ui, &ap, &dp);
        return;
    }

    match app.reader.nav_stack.last_mut() {
        Some(&mut crate::NavigationLevel::Reading { ref mut page_index, ref mut dir_path, ref mut archive_path }) => {
            let reader_meta = app.lib.manga_list.iter()
                .find(|m| m.archives.iter().any(|a| *a == *archive_path))
                .map(|m| (m.effective_mode(), m.manual_mode, m.effective_rtl(), m.id));

            let total = app.reader.current_chapter.pages.len();
            if total == 0 { return; }

            let mode = app.lib.manga_list.iter()
                .find(|m| m.archives.iter().any(|a| *a == *archive_path))
                .map(|m| m.effective_mode())
                .unwrap_or(crate::reader::mode::ReaderMode::Manga);
            let rtl = app.lib.manga_list.iter()
                .find(|m| m.archives.iter().any(|a| *a == *archive_path))
                .map(|m| m.effective_rtl())
                .unwrap_or(false);
            let is_last_chapter = {
                let ch_total = app.lib.chapters.len();
                if ch_total > 0 {
                    app.lib.chapters.iter()
                        .position(|c| c.path == *archive_path)
                        .map(|idx| idx + 1 >= ch_total)
                        .unwrap_or(false)
                } else { false }
            };

            let bg = app.theme.reader_bg;
            let resp = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(resp.rect, 0.0, bg);

            if *page_index >= total {
                match &app.reader.next_switch.status {
                    Status::Loading { .. } => {
                        let cx = resp.rect.center();
                        let r = app.settings.reader.spinner_radius;
                        let angle = ((app.reader.current_chapter.loading.len() as f32 * app.settings.reader.spinner_speed).sin() * std::f32::consts::TAU * 4.0).min(std::f32::consts::TAU * 0.75);
                        let n = app.settings.reader.spinner_segments;
                        let pts: Vec<egui::Pos2> = (0..=n).map(|i| {
                            let a = angle * i as f32 / n as f32;
                            egui::pos2(cx.x + r * a.cos(), cx.y + r * a.sin())
                        }).collect();
                        ui.painter().add(egui::Shape::Path(egui::epaint::PathShape {
                            points: pts, closed: false, fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke::new(app.settings.reader.spinner_stroke, egui::Color32::GRAY).into(),
                        }));
                        ui.painter().text(cx + egui::vec2(0.0, r + 10.0), egui::Align2::CENTER_TOP, app.lang.loading_next, egui::FontId::proportional(app.settings.reader.status_font_size), egui::Color32::GRAY);
                    }
                    Status::Error(e) => {
                        ui.painter().text(resp.rect.center(), egui::Align2::CENTER_CENTER, app.lang.error_msg(e), egui::FontId::proportional(18.0), egui::Color32::RED);
                    }
                    _ => {}
                }
            }

            let alt = ui.input(|i| i.modifiers.alt);
            let popup_open = ui.ctx().data_mut(|d| d.get_persisted::<bool>(egui::Id::new("ch_popup"))).unwrap_or(false);
            let fs = ui.ctx().data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "active")))).unwrap_or(false);
            let mut flip = ui.data_mut(|d| d.get_temp::<PageFlipState>(page_flip_id())).unwrap_or(PageFlipState::new());
            let s = &app.settings;
            let single_def = if s.single_width > 0.0 { s.single_width } else { app.settings.single_width };
            let mut cw = match mode {
                crate::reader::mode::ReaderMode::Manga => single_def,
                crate::reader::mode::ReaderMode::Double => {
                    if s.double_width > 0.0 { s.double_width } else { single_def * app.settings.reader.double_width_multiplier }
                }
                _ => 0.0,
            };
            let raw = ui.input(|i| i.raw_scroll_delta.y);

            if alt && raw != 0.0 {
                let step = app.settings.reader.resize_step;
                let max_w = resp.rect.width() - 1.0;
                let min_w = app.settings.reader.min_content_width.min(max_w);
                let new = (cw + raw.signum() * step as f32).clamp(min_w, max_w);
                if (new - cw).abs() > app.settings.reader.resize_threshold {
                    cw = new;
                    match mode {
                        crate::reader::mode::ReaderMode::Manga => app.settings.single_width = new,
                        crate::reader::mode::ReaderMode::Double => app.settings.double_width = new,
                        _ => {}
                    }
                    app.settings.save();
                }
                ui.input_mut(|i| {
                    i.raw_scroll_delta = egui::Vec2::ZERO;
                    i.smooth_scroll_delta = egui::Vec2::ZERO;
                });
            } else if !alt && raw != 0.0 && app.reader.scroll_cooldown == 0 && !popup_open && !flip.is_active() {
                let step = crate::reader::chapter_switch::page_step(mode);
                let new = if raw > 0.0 {
                    page_index.saturating_sub(step)
                } else {
                    (*page_index + step).min(total.saturating_sub(1))
                };
                let changed = new != *page_index
                    || mode == crate::reader::mode::ReaderMode::Double;
                if changed {
                    let forward = raw < 0.0;
                    match mode {
                        crate::reader::mode::ReaderMode::Manga =>
                            single_page::trigger_flip(&mut flip, *page_index, total, forward),
                        crate::reader::mode::ReaderMode::Double =>
                            double_page::trigger_flip_from_real(
                                &mut flip, *page_index, &app.reader.current_chapter.aspect_ratios,
                                app.settings.reader.panorama_threshold, rtl, is_last_chapter, forward),
                        _ => {
                            *page_index = new;
                            crate::audio::page_turn(app.settings.audio.muted, app.settings.audio.volume);
                            if let Some(ref db) = app.db {
                                let _ = db.save_progress(dir_path, archive_path, new, total);
                            }
                            if !app.reader.current_chapter.cache.contains_key(&new)
                                && !app.reader.current_chapter.loading.contains(&new)
                            {
                                let page_name = app.reader.current_chapter.pages[new].name.clone();
                                let tx = app.reader.reader_tx.clone();
                                let ap = archive_path.clone();
                                let arcs = app.reader.current_chapter.archive.clone();
                                app.reader.current_chapter.loading.insert(new);
                                let max_decode_width = app.settings.reader.max_decode_width;
                                let _ = crate::reader::worker::decode_tx().send(
                                    crate::reader::worker::DecodeJob {
                                        ap: ap.clone(),
                                        n: new,
                                        page_name: page_name.clone(),
                                        max_decode_width,
                                        arcs,
                                        tx,
                                        use_max_width: false,
                                    }
                                );
                            }
                        }
                    }
                }
                app.reader.scroll_cooldown = app.settings.reader.scroll_cooldown_frames as u8;
            }
            if raw != 0.0 || ui.input(|i| i.smooth_scroll_delta) != egui::Vec2::ZERO {
                ui.input_mut(|i| {
                    i.raw_scroll_delta = egui::Vec2::ZERO;
                    i.smooth_scroll_delta = egui::Vec2::ZERO;
                });
            }
            if app.reader.scroll_cooldown > 0 {
                app.reader.scroll_cooldown -= 1;
            }

            #[cfg(feature = "fx")]
            let curtain_guard = app.reader.fx_player.interactive() && app.reader.fx_player.has_curtain();
            #[cfg(not(feature = "fx"))]
            let curtain_guard = false;

            if resp.clicked() && !alt && !popup_open && !flip.is_active() {
                if curtain_guard {
                    // штора на спреде: клик только раскрывает кадр/кусок, никаких
                    // перелистываний и переключения fs-тулбара (конфликт кликов).
                    // return здесь НЕ делаем: он пропускал рендер читалки и под
                    // страницей оставался голый фон — «мигание всего ридера».
                    #[cfg(feature = "fx")]
                    app.reader.fx_player.reveal_next();
                }
                let pointer = ui.ctx().pointer_interact_pos().unwrap_or(resp.rect.center());
                let mid_x = resp.rect.center().x;
                let cz = resp.rect.width() * 0.1;
                let in_fs_center = fs && (pointer.x - mid_x).abs() < cz;
                if !in_fs_center && !curtain_guard {
                    #[cfg(feature = "fx")]
                    if app.reader.fx_player.interactive() {
                        crate::ui::reader::fx_hooks::update_manga_spread(
                            &mut app.reader.fx_player,
                            *page_index,
                            total,
                            &app.reader.current_chapter.aspect_ratios,
                            app.settings.reader.panorama_threshold,
                            rtl,
                            is_last_chapter,
                            mode == crate::reader::mode::ReaderMode::Double,
                        );
                        if app.reader.fx_player.reveal_next() {
                            return;
                        }
                    }
                    let at_end = if mode == crate::reader::mode::ReaderMode::Double {
                        let vp = crate::reader::chapter_bundle::build_virtual_pages(
                            total, &app.reader.current_chapter.aspect_ratios,
                            app.settings.reader.panorama_threshold, rtl, is_last_chapter);
                        double_page::at_final_spread(&vp, *page_index)
                    } else {
                        *page_index + 1 >= total
                    };
                    if at_end && crate::reader::chapter_switch::is_next_section(pointer.x, mid_x, rtl) {
                        if let Some((nap, ndp, npage)) = crate::reader::chapter_switch::take_switch(
                            &mut app.reader.next_switch, &mut app.reader.prev_switch,
                            &mut app.reader.current_chapter, &mut app.reader.prev_chapter,
                            &mut app.reader.joined_chapter, &app.reader.reader_tx, &app.settings, true)
                        {
                            *archive_path = nap;
                            *dir_path = ndp;
                            *page_index = npage;
                            #[cfg(feature = "fx")]
                            crate::ui::reader::fx_hooks::load_chapter(
                                &mut app.reader.fx_player, &app.lib, archive_path.as_str());
                        } else {
                            crate::reader::chapter_switch::request_switch(
                                &mut app.reader.next_switch, &mut app.reader.prev_switch,
                                &app.lib, &app.settings, archive_path, dir_path, true);
                        }
                        return;
                    }
                    if *page_index == 0
                        && crate::reader::chapter_switch::is_prev_section(pointer.x, mid_x, rtl)
                    {
                        if let Some((nap, ndp, npage)) = crate::reader::chapter_switch::take_switch(
                            &mut app.reader.next_switch, &mut app.reader.prev_switch,
                            &mut app.reader.current_chapter, &mut app.reader.prev_chapter,
                            &mut app.reader.joined_chapter, &app.reader.reader_tx, &app.settings, false)
                        {
                            *archive_path = nap;
                            *dir_path = ndp;
                            *page_index = npage;
                            #[cfg(feature = "fx")]
                            crate::ui::reader::fx_hooks::load_chapter(
                                &mut app.reader.fx_player, &app.lib, archive_path.as_str());
                        } else {
                            crate::reader::chapter_switch::request_switch(
                                &mut app.reader.next_switch, &mut app.reader.prev_switch,
                                &app.lib, &app.settings, archive_path, dir_path, false);
                        }
                        return;
                    }
                }
            }

            if fs && resp.clicked() && !curtain_guard {
                let pointer = ui.ctx().pointer_interact_pos().unwrap_or(resp.rect.center());
                let mid_x = resp.rect.center().x;
                let center_zone = resp.rect.width() * 0.1;
                if (pointer.x - mid_x).abs() < center_zone {
                    let cur = ui.ctx().data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "toolbar")))).unwrap_or(false);
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("fs", "toolbar")), !cur));
                }
            }

            match mode {
                crate::reader::mode::ReaderMode::Manga => single_page::draw(
                    ui, &resp, page_index, archive_path, total, &app.reader.current_chapter.aspect_ratios, &app.lang,
                    &mut app.reader.current_chapter.cache, &mut app.reader.current_chapter.loading,
                    &app.reader.current_chapter.pages, &app.reader.current_chapter.archive,
                    &app.reader.reader_tx, &app.settings, rtl, cw, alt, &mut flip, fs,
                    #[cfg(feature = "fx")] &mut app.reader.fx_player),
                crate::reader::mode::ReaderMode::Double => {
                    double_page::draw(
                        ui, &resp, page_index, archive_path, total, &app.lang,
                        &mut app.reader.current_chapter.cache, &mut app.reader.current_chapter.loading,
                        &app.reader.current_chapter.pages, &app.reader.current_chapter.archive,
                        &app.reader.reader_tx, &app.reader.current_chapter.aspect_ratios, &app.settings, rtl, cw, alt, &mut flip,
                        is_last_chapter, fs,
                        #[cfg(feature = "fx")] &mut app.reader.fx_player);
                }
                crate::reader::mode::ReaderMode::Webtoon => {}
            }

            if let FlipPhase::Animating(t) = &mut flip.phase {
                let dt = ui.input(|i| i.stable_dt).min(0.1);
                *t = (*t + dt * 8.0).min(1.0);
                if *t >= 1.0 {
                    if flip.committing {
                        let new_idx = if mode == crate::reader::mode::ReaderMode::Double {
                            flip.target_real
                        } else {
                            flip.target_page
                        };
                        if new_idx <= total {
                            *page_index = new_idx;
                            crate::audio::page_turn(app.settings.audio.muted, app.settings.audio.volume);
                            if let Some(ref db) = app.db {
                                let _ = db.save_progress(dir_path, archive_path, *page_index, total);
                            }
                        }
                    }
                    flip.phase = FlipPhase::Idle;
                }
                ui.ctx().request_repaint();
            }
            ui.data_mut(|d| d.insert_temp(page_flip_id(), flip));

            app.reader.reader_page = *page_index;
            app.reader.reader_total = total;
            let pending_action = std::cell::RefCell::new(None::<(u64, crate::ui::reader::reader_context_menu::MenuAction)>);
            if let Some((_mode, manual_mode, rtl, manga_id)) = reader_meta {
                resp.context_menu(|ui| {
                    let (ma, ms, md, mw) = (app.lang.mode_auto, app.lang.mode_single, app.lang.mode_double, app.lang.mode_webtoon);
                    crate::ui::reader::reader_context_menu::draw(ui, manual_mode, rtl, ma, ms, md, mw, |action| {
                        *pending_action.borrow_mut() = Some((manga_id, action));
                    });
                    ui.separator();
                    if ui.button("󰊓").on_hover_text(app.lang.fullscreen).clicked() {
                        let ctx = ui.ctx();
                        let fs_id = |name| egui::Id::new(("fs", name));
                        let next = ctx.data_mut(|d| {
                            let active = d.get_temp::<bool>(fs_id("active")).unwrap_or(false);
                            if !active {
                                d.insert_temp(fs_id("backup_sidebar"), app.sidebar_visible);
                                app.sidebar_visible = false;
                                d.insert_temp(fs_id("toolbar"), false);
                            } else {
                                if let Some(restore) = d.get_temp::<bool>(fs_id("backup_sidebar")) {
                                    app.sidebar_visible = restore;
                                }
                            }
                            let next = !active;
                            d.insert_temp(fs_id("active"), next);
                            next
                        });
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(next));
                    }
                });
            }
            if let Some((id, action)) = pending_action.into_inner() {
                match action {
                    crate::ui::reader::reader_context_menu::MenuAction::SetMode(mode) => {
                        if mode == Some(crate::reader::mode::ReaderMode::Webtoon) {
                            app.reader.set_manga_mode_with_switch(&mut app.lib, &mut app.db, id, mode, &mut app.pending_reader_mode);
                        } else {
                            app.reader.set_manga_mode(&mut app.lib, &mut app.db, id, mode);
                        }
                    }
                    crate::ui::reader::reader_context_menu::MenuAction::SetRtl(rtl) => {
                        app.reader.set_manga_rtl(&mut app.lib, &mut app.db, id, rtl);
                    }
                }
            }
        }
        _ => {}
    }
}
