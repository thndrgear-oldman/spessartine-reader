use eframe::egui;
use crate::ui::button;
use crate::ui::reader::seeker;

fn address_string(app: &crate::SpessartineApp) -> String {
    match app.current_view {
        crate::AppView::Library => match app.reader.nav_stack.last() {
            Some(crate::NavigationLevel::Titles) => app.lang.tb_library.into(),
            Some(crate::NavigationLevel::Chapters { dir_path }) => dir_path.clone(),
            Some(crate::NavigationLevel::Pages { archive_path, .. }) => archive_path.clone(),
            Some(crate::NavigationLevel::Reading { archive_path, .. })
            | Some(crate::NavigationLevel::ReadingWebtoon { archive_path, .. }) => {
                std::path::Path::new(archive_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(app.lang.tb_reading)
                    .to_string()
            }
            _none => app.lang.tb_library.into(),
        },
        _ => app.lang.tb_library.into(),
    }
}

fn handle_back(app: &mut crate::SpessartineApp) {
    if let Some(crate::NavigationLevel::ReadingWebtoon { archive_path, dir_path }) =
        app.reader.nav_stack.last()
    {
        if let Some(ref db) = app.db {
            if let Some(ref state) = app.reader.reader_webtoon_state {
                if state.last_saved_total > 0 && !state.last_saved_ap.is_empty() {
                    let total = state.last_saved_total;
                    let cur = state.last_saved_page.min(total.saturating_sub(1));
                    let _ = db.save_progress(dir_path, &state.last_saved_ap, cur, total);
                } else {
                    let cum: Vec<f32> = state
                        .current_h
                        .iter()
                        .scan(0.0, |a, &h| {
                            *a += h;
                            Some(*a)
                        })
                        .collect();
                    let cur = if cum.is_empty() {
                        0
                    } else {
                        crate::ui::webtoon_reader::state::current_index(
                            state.frame_offset.max(0.0),
                            &cum,
                        )
                    };
                    let total = app.reader.current_chapter.pages.len();
                    let _ = db.save_progress(
                        dir_path,
                        archive_path,
                        cur.min(total.saturating_sub(1)),
                        total,
                    );
                }
            }
        }
    }
    app.reader.nav_stack.pop();
    app.last_scroll_offset = 0.0;
    app.reader.current_chapter.cache_clear();
    app.reader.current_chapter.loading.clear();
    app.reader.reader_webtoon_state = None;
    app.reader.current_chapter.heights.clear();
    app.reader.prev_chapter.clear();
    app.reader.joined_chapter.clear();
    match app.reader.nav_stack.last() {
        Some(crate::NavigationLevel::Titles) => {
            app.lib.clear_chapters();
            app.reader.current_chapter.pages.clear();
            app.covers.chapter_textures.clear();
            app.covers.chapter_cover_order.clear();
            app.covers.page_textures.clear();
            app.covers.page_cover_order.clear();
            app.current_archive = None;
            app.reader.current_chapter.archive = None;
            crate::archive::clear_archive_cache();
        }
        Some(crate::NavigationLevel::Chapters { .. }) => {
            app.reader.current_chapter.pages.clear();
            app.covers.page_textures.clear();
            app.covers.page_cover_order.clear();
            app.current_archive = None;
            app.reader.current_chapter.archive = None;
            crate::archive::clear_archive_cache();
        }
        Some(crate::NavigationLevel::ReadingWebtoon { .. }) => {
            app.current_archive = None;
            app.reader.current_chapter.clear();
            crate::archive::clear_archive_cache();
        }
        _ => {}
    }
}

fn reader_toolbar(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    use crate::NavigationLevel;

    let (archive_path, dir_path) = match app.reader.nav_stack.last() {
        Some(NavigationLevel::Reading { archive_path, dir_path, .. })
        | Some(NavigationLevel::ReadingWebtoon { archive_path, dir_path }) => {
            (archive_path.clone(), dir_path.clone())
        }
        _ => return,
    };

    let manga = app.lib.manga_list.iter().find(|m| m.directory == dir_path);
    let title = manga.map(|m| m.name.clone()).unwrap_or_default();
    let ch_idx = app.lib.chapter_idx.get(&archive_path).copied();
    let ch_total = app.lib.chapters.len();
    let page = app.reader.reader_page;
    let total = app.reader.reader_total;

    let popup_id = egui::Id::new("ch_popup");
    let scroll_id = egui::Id::new("ch_popup_scroll");
    let mut popup_open = ui.ctx().data_mut(|d| d.get_persisted::<bool>(popup_id)).unwrap_or(false);

    ui.horizontal(|ui| {
        ui.add_space(6.0);

        let resp = button::icon_btn(ui, "", &app.theme);
        if resp.clicked() {
            let fs = ui.ctx().data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "active")))).unwrap_or(false);
            if !fs {
                app.sidebar_visible = !app.sidebar_visible;
            }
        }

        let resp = button::icon_btn(ui, "󰌍", &app.theme);
        if resp.clicked() {
            handle_back(app);
            ui.ctx().data_mut(|d| d.insert_persisted(popup_id, false));
            ui.ctx().data_mut(|d| d.insert_persisted(scroll_id, false));
        }
        ui.add_space(4.0);

        let max_title_w = 200.0;
        let resp = button::text_btn_max(ui, &title, app.theme.toolbar_btn_fg, &app.theme, max_title_w).on_hover_text(&title);
        if resp.clicked() {
            popup_open = !popup_open;
            ui.ctx().data_mut(|d| d.insert_persisted(popup_id, popup_open));
        }
        ui.add_space(8.0);

        if let Some(idx) = ch_idx {
            let text = app.lang.gl_chapter(idx + 1, ch_total);
            let resp = button::text_btn(ui, &text, egui::Color32::GRAY, &app.theme);
            if resp.clicked() {
                popup_open = !popup_open;
                ui.ctx().data_mut(|d| d.insert_persisted(popup_id, popup_open));
            }
        }

        let seeker_w = (ui.available_width() - 200.0).max(60.0);
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        let seeker_rect = ui.allocate_exact_size(
            egui::vec2(seeker_w, 20.0),
            egui::Sense::hover(),
        ).0;
        seeker::draw(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(seeker_rect.left(), seeker_rect.top() + 2.0),
                egui::vec2(seeker_w, 16.0),
            ),
            total.max(1), page,
            &mut app.reader.reader_seeker, dt,
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);
            #[cfg(feature = "fx")]
            {
                let on = app.reader.fx_player.interactive();
                let fg = if on { app.theme.toolbar_btn_fg } else { egui::Color32::GRAY };
                if button::text_btn(ui, "FX", fg, &app.theme)
                    .on_hover_text(app.lang.tb_interactive)
                    .clicked()
                {
                    app.reader.toggle_fx(&app.lib, &app.settings);
                }
                ui.add_space(4.0);
                if button::text_btn(ui, "", app.theme.toolbar_btn_fg, &app.theme)
                    .on_hover_text(app.lang.tb_stop_snd)
                    .clicked()
                {
                    crate::audio::stop_playlist(crate::audio::Channel::Bgm, 0.3);
                    crate::audio::stop_playlist(crate::audio::Channel::Ambient, 0.3);
                    crate::audio::stop_frame(0.3);
                }
                ui.add_space(4.0);
            }
            if button::icon_btn(ui, "󰊓", &app.theme).on_hover_text(app.lang.fullscreen).clicked() {
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
            ui.add_space(6.0);
            if total > 0 {
                let pg = format!("{}/{}", (page + 1).min(total), total);
                button::text_btn(ui, &pg, egui::Color32::GRAY, &app.theme);
            }
            ui.add_space(6.0);
        });
        if popup_open {
            let scrolled = ui.ctx().data_mut(|d| d.get_persisted::<bool>(scroll_id)).unwrap_or(false);
            let current_ch = app.lib.chapters.iter()
                .position(|c| c.path == app.reader.current_chapter.archive_path);

            egui::Area::new("chapter_popup".into())
                .fixed_pos(ui.min_rect().left_bottom())
                .movable(false)
                .show(ui.ctx(), |ui| {
                    egui::Frame::default()
                        .fill(app.theme.reader_bg)
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(4, 4))
                        .show(ui, |ui| {
                            ui.set_min_width(300.0);
                            ui.set_max_height(400.0);

                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .id_salt("ch_popup_scroll")
                                .show(ui, |ui| {
                                    let mut target = None;
                                    for (i, ch) in app.lib.chapters.iter().enumerate() {
                                        let is_current = Some(i) == current_ch;
                                        let fg = if is_current {
                                            app.theme.toolbar_btn_fg
                                        } else {
                                            egui::Color32::GRAY
                                        };
                                        let resp = button::text_btn(ui, &ch.name, fg, &app.theme);
                                        if is_current && !scrolled {
                                            resp.scroll_to_me(Some(egui::Align::Center));
                                            ui.ctx().data_mut(|d| d.insert_persisted(scroll_id, true));
                                        }
                                        if resp.clicked() && !is_current {
                                            target = Some(ch.path.clone());
                                        }
                                    }
                                    if let Some(path) = target {
                                        app.reader.jump_to_chapter(&path, &app.settings, &app.lib);
                                        ui.ctx().data_mut(|d| d.insert_persisted(popup_id, false));
                                    }
                                });
                        });
                });
        } else {
            ui.ctx().data_mut(|d| d.insert_persisted(scroll_id, false));
        }
    });
}

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    use crate::NavigationLevel;

    let is_reader = matches!(
        app.reader.nav_stack.last(),
        Some(NavigationLevel::Reading { .. }) | Some(NavigationLevel::ReadingWebtoon { .. })
    );

    if is_reader {
        reader_toolbar(app, ui);
        return;
    }

    ui.horizontal(|ui| {
        ui.add_space(6.0);

        let resp = button::icon_btn(ui, "", &app.theme);
        if resp.clicked() {
            app.sidebar_visible = !app.sidebar_visible;
        }

        let resp = button::icon_btn(ui, "", &app.theme);
        if resp.clicked() {
            app.trigger_rescan();
        }

        if app.current_view == crate::AppView::Library && app.reader.nav_stack.len() > 1 {
            let resp = button::icon_btn(ui, "󰌍", &app.theme);
            if resp.clicked() {
                handle_back(app);
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.add_space(6.0);
            let addr = address_string(app);
            let addr_w = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(addr_w, ui.available_height()),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    egui::Frame::new()
                        .fill(app.theme.addres_bar_bg)
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(8, 9))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&addr)
                                        .color(app.theme.toolbar_btn_fg)
                                        .size(14.0),
                                )
                                .truncate(),
                            );
                        });
                },
            );
        });
    });
}
