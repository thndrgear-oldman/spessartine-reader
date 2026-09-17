pub mod state;
pub mod scroll;
pub mod preload;
pub mod rotate;

use eframe::egui;

pub fn draw(
    app: &mut crate::SpessartineApp,
    ui: &mut egui::Ui,
    archive_path: &str,
    _dir_path: &str,
) {
    let state = app.reader.reader_webtoon_state
        .get_or_insert_with(|| {
            let s = crate::config::settings::ReaderSettings::load();
            let rw = if s.webtoon_width > 0.0 { s.webtoon_width } else { s.webtoon.min_width };
            state::WebtoonState::new(rw)
        });

    if state.last_chapter != archive_path {
        state.last_chapter = archive_path.to_string();
        state.prev_h.clear();
        state.current_h.clear();
        state.joined_h.clear();
    }

    if !app.reader.current_chapter.heights.is_empty()
        && (state.prev_h.len() != app.reader.prev_chapter.heights.len()
            || state.current_h.len() != app.reader.current_chapter.heights.len()
            || state.joined_h.len() != app.reader.joined_chapter.heights.len())
    {
        state.prev_h.clone_from(&app.reader.prev_chapter.heights);
        state.current_h.clone_from(&app.reader.current_chapter.heights);
        state.joined_h.clone_from(&app.reader.joined_chapter.heights);
    }
    let _dt = ui.input(|i| i.stable_dt).min(0.1);
    let vp_h = ui.available_rect_before_wrap().height().max(500.0);
    let h = vp_h * 0.92;
    if state.current_h.is_empty() && !app.reader.current_chapter.pages.is_empty() {
        state.current_h = vec![h; app.reader.current_chapter.pages.len()];
    }

    if state.prev_h.is_empty() && !app.reader.prev_chapter.pages.is_empty() {
        state.prev_h = vec![h; app.reader.prev_chapter.pages.len()];
    }
    if state.joined_h.is_empty() && !app.reader.joined_chapter.pages.is_empty() {
        state.joined_h = vec![h; app.reader.joined_chapter.pages.len()];
    }

    let seek_id = egui::Id::new(("webtoon_seek", archive_path));
    let seek = std::cell::RefCell::new(None::<usize>);
    ui.ctx().data_mut(|d| {
        *seek.borrow_mut() = d.get_persisted::<usize>(seek_id);
        d.remove::<usize>(seek_id);
    });
    if let Some(seek_page) = seek.into_inner() {
        state.seek_target = Some((seek_page, 0.0));
    }
    if state.seek_target.is_some() {
        state.prev_h.clear();
        state.joined_h.clear();
        app.reader.prev_chapter.pages.clear();
        app.reader.joined_chapter.pages.clear();
    }

    let avail_w = ui.available_rect_before_wrap().width();
    if state.ref_width > avail_w && !state.current_h.is_empty() {
        let scale = avail_w / state.ref_width;
        state.ref_width = avail_w;
        for h in &mut state.prev_h { *h *= scale; }
        for h in &mut state.current_h { *h *= scale; }
        for h in &mut state.joined_h { *h *= scale; }
        state.frame_offset *= scale;
        state.last_total_h *= scale;
    }

    let spacing = app.settings.webtoon.page_spacing;
    let prev_h = &state.prev_h;
    let current_h = &state.current_h;
    let joined_h = &state.joined_h;
    let total_len = prev_h.len() + current_h.len() + joined_h.len();
    let mut cum_y = Vec::with_capacity(total_len + 1);
    cum_y.push(0.0);
    for h in prev_h.iter().chain(current_h.iter()).chain(joined_h.iter()) {
        cum_y.push(cum_y.last().copied().unwrap_or(0.0) + h + spacing as f32);
    }
    let old_total = state.last_total_h;
    state.last_total_h = *cum_y.last().unwrap_or(&0.0);

    if let Some((target_page, _)) = state.seek_target.take() {
        state.pending_seek = Some(target_page);
        state.seek_page = Some(target_page);
    }

    if let Some(ps) = state.pending_seek {
        if ps < state.current_h.len() {
            let current_total = state.current_h.iter().sum::<f32>();
            let changed = old_total.abs() < 0.5  // первый кадр
                || (current_total - state.seek_initial_current_total).abs() > 0.5;
            if changed {
            let seek_y = cum_y.get(ps + state.prev_h.len()).copied().unwrap_or(0.0) + spacing as f32;
                state.frame_offset = seek_y;
                state.seek_initial_current_total = current_total;
                state.last_total_h = *cum_y.last().unwrap_or(&0.0);
                state.pending_seek = None;
             }
         }
     }

    if let Some(sp) = state.seek_page {
        let loading = !app.reader.current_chapter.loading.is_empty()
            || !app.reader.prev_chapter.loading.is_empty()
            || !app.reader.joined_chapter.loading.is_empty();
        if loading {
            let seek_y = cum_y.get(sp + state.prev_h.len()).copied().unwrap_or(0.0) + spacing as f32;
            state.frame_offset = seek_y;
            state.last_total_h = *cum_y.last().unwrap_or(&0.0);
        } else {
            state.frame_offset = cum_y.get(sp + state.prev_h.len()).copied().unwrap_or(0.0) + spacing as f32;
            state.last_total_h = *cum_y.last().unwrap_or(&0.0);
            state.seek_page = None;
        }
    }

    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, app.theme.reader_bg);
    let rota = crate::ui::webtoon_reader::scroll::scroll_controller(
        ui, rect, state, &cum_y, &app.settings,
        &app.reader.prev_chapter.cache,
        &app.reader.current_chapter.cache,
        &app.reader.joined_chapter.cache,
        rect.width(),
        #[cfg(feature = "fx")] &mut app.reader.fx_player,
    );

    app.reader.prev_chapter.heights.clone_from(&state.prev_h);
    app.reader.current_chapter.heights.clone_from(&state.current_h);
    app.reader.joined_chapter.heights.clone_from(&state.joined_h);

    if let Some(dir) = rota {
        match dir {
            crate::ui::webtoon_reader::scroll::RotaDir::Up => {
                app.reader.joined_chapter = std::mem::take(&mut app.reader.current_chapter);
                app.reader.current_chapter = std::mem::take(&mut app.reader.prev_chapter);
            }
            crate::ui::webtoon_reader::scroll::RotaDir::Down => {
                app.reader.prev_chapter = std::mem::take(&mut app.reader.current_chapter);
                app.reader.current_chapter = std::mem::take(&mut app.reader.joined_chapter);
            }
        }

        if let Some(crate::NavigationLevel::ReadingWebtoon { archive_path, dir_path }) = app.reader.nav_stack.last_mut() {
            *archive_path = app.reader.current_chapter.archive_path.clone();
            *dir_path = app.reader.current_chapter.dir_path.clone();
        }

        // глава сменилась в вебтуне — перезагрузить эффекты (8.5: stop + open)
        #[cfg(feature = "fx")]
        crate::ui::reader::fx_hooks::load_chapter(
            &mut app.reader.fx_player,
            &app.lib,
            app.reader.current_chapter.archive_path.as_str(),
        );

        let y_into_joined = match dir {
            crate::ui::webtoon_reader::scroll::RotaDir::Down =>
                (state.frame_offset - cum_y.get(state.prev_h.len() + state.current_h.len()).copied().unwrap_or(0.0)).max(0.0),
            _ => 0.0,
        };

        state.rotation_lock = 5;
        state.velocity = 0.0;
        state.near_start = false;
        state.near_end = false;
        state.pending_seek = None;
        state.seek_page = None;
        state.prev_h.clone_from(&app.reader.prev_chapter.heights);
        state.current_h.clone_from(&app.reader.current_chapter.heights);
        state.joined_h.clone_from(&app.reader.joined_chapter.heights);
        cum_y.clear();
        cum_y.push(0.0);
        for h in state.prev_h.iter().chain(state.current_h.iter()).chain(state.joined_h.iter()) {
            cum_y.push(cum_y.last().copied().unwrap_or(0.0) + h + spacing as f32);
        }
        match dir {
            crate::ui::webtoon_reader::scroll::RotaDir::Up =>
                state.frame_offset = cum_y.get(state.prev_h.len()).copied().unwrap_or(0.0).max(0.0),
            crate::ui::webtoon_reader::scroll::RotaDir::Down =>
                state.frame_offset = cum_y.get(state.prev_h.len()).copied().unwrap_or(0.0) + y_into_joined,
        }

         state.last_total_h = *cum_y.last().unwrap_or(&0.0);
    }

    let raw_cur = crate::ui::webtoon_reader::state::current_index(
        state.frame_offset.max(0.0), &cum_y,
    );
     let seek_bias = state.seek_page.and_then(|sp| {
         let ty = cum_y.get(sp).copied()?;
         Some((state.frame_offset - ty).abs() < vp_h * 0.5)
     }).unwrap_or(false);
     let cur = if seek_bias {
         state.seek_page.unwrap_or(0)
     } else {
         raw_cur
     };
    let prev_len = state.prev_h.len();
    let cur_total = state.current_h.len();

    let (page, total) = if cur < prev_len {
        (cur + 1, prev_len)
    } else if cur < prev_len + cur_total {
        (cur - prev_len + 1, cur_total)
    } else {
        (cur - prev_len - cur_total + 1, state.joined_h.len())
    };
    app.reader.reader_page = page.saturating_sub(1);
    app.reader.reader_total = total;

    let (bundle, ap): (&mut crate::reader::ChapterBundle, String) = if cur < prev_len {
        let ap = app.reader.prev_chapter.archive_path.clone();
        (&mut app.reader.prev_chapter, ap)
    } else if cur < prev_len + cur_total {
        let ap = app.reader.current_chapter.archive_path.clone();
        (&mut app.reader.current_chapter, ap)
    } else {
        let ap = app.reader.joined_chapter.archive_path.clone();
        (&mut app.reader.joined_chapter, ap)
    };
    let local_page = if cur < prev_len { cur }
        else if cur < prev_len + cur_total { cur - prev_len }
        else { cur - prev_len - cur_total };
    let total_pages = bundle.pages.len();
    let page = local_page.min(total_pages.saturating_sub(1));
    let ps = page.saturating_sub(2);
    let pc = 7.min(total_pages.saturating_sub(ps));
    if pc > 0 && !bundle.archive_path.is_empty() {
        crate::ui::webtoon_reader::preload::preload_pages(
            bundle, &ap, &app.reader.reader_tx, ps, pc,
            app.settings.preload.max_concurrent as usize,
            app.settings.reader.max_decode_width,
        );
    }

    if total_pages > 0 {
        let save_ap = bundle.archive_path.clone();
        let save_dir = bundle.dir_path.clone();
        let save_page = page.min(total_pages.saturating_sub(1));
        if let Some(ref db) = app.db {
            if let Some(ref mut st) = app.reader.reader_webtoon_state {
                if st.last_saved_ap != save_ap || st.last_saved_page != save_page {
                    st.last_saved_ap = save_ap;
                    st.last_saved_page = save_page;
                    st.last_saved_total = total_pages;
                    let dir = if !save_dir.is_empty() {
                        save_dir
                    } else {
                        app.reader.current_chapter.dir_path.clone()
                    };
                    let _ = db.save_progress(&dir, &st.last_saved_ap, save_page, total_pages);
                }
            }
        }
    }

    if cur >= prev_len + cur_total.saturating_sub(2) && !app.reader.joined_chapter.pages.is_empty() {
        let j_ps = 0usize;
        let j_pc = 7.min(app.reader.joined_chapter.pages.len());
        let j_ap = app.reader.joined_chapter.archive_path.clone();
        if j_pc > 0 {
            crate::ui::webtoon_reader::preload::preload_pages(
                &mut app.reader.joined_chapter,
                &j_ap,
                &app.reader.reader_tx, j_ps, j_pc,
                app.settings.preload.max_concurrent as usize,
                app.settings.reader.max_decode_width,
            );
        }
    }

    let manga_id = app.lib.manga_list.iter()
        .find(|m| m.archives.iter().any(|a| a == archive_path))
        .map(|m| m.id);
    let manual_mode = app.lib.manga_list.iter()
        .find(|m| m.archives.iter().any(|a| a == archive_path))
        .and_then(|m| m.manual_mode);
    let rtl = app.lib.manga_list.iter()
        .find(|m| m.archives.iter().any(|a| a == archive_path))
        .map(|m| m.effective_rtl())
        .unwrap_or(false);

    let pending_action = std::cell::RefCell::new(
        None::<(u64, crate::ui::reader::reader_context_menu::MenuAction)>,
    );
    let ctx_resp = ui.interact(rect, egui::Id::new("webtoon_ctx"), egui::Sense::click());
    if let Some(id) = manga_id {
        ctx_resp.context_menu(|ui| {
            let (ma, ms, md, mw) = (app.lang.mode_auto, app.lang.mode_single, app.lang.mode_double, app.lang.mode_webtoon);
            crate::ui::reader::reader_context_menu::draw(ui, manual_mode, rtl, ma, ms, md, mw, |action| {
                *pending_action.borrow_mut() = Some((id, action));
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

    if ui.data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "active")))).unwrap_or(false) && ctx_resp.clicked() {
        let cur = ui.data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "toolbar")))).unwrap_or(false);
        ui.data_mut(|d| d.insert_temp(egui::Id::new(("fs", "toolbar")), !cur));
    }
    if let Some((id, action)) = pending_action.into_inner() {
        match action {
            crate::ui::reader::reader_context_menu::MenuAction::SetMode(mode) => {
                if mode != Some(crate::reader::mode::ReaderMode::Webtoon) {
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
