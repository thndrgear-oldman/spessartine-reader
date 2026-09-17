use eframe::egui;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    if app.reader.current_chapter.pages.is_empty() {
        ui.label(app.lang.pages_none);
        return;
    }

    let spacing = ui.spacing().item_spacing.x;
    let cell_h = 250.0;
    let avail_w = ui.available_width();

    let cols_noshrink = ((avail_w + spacing) / (170.0 + spacing)).floor().max(1.0) as usize;
    let cell_w_noshrink = (avail_w - (cols_noshrink as f32 - 1.0) * spacing) / cols_noshrink as f32;
    let cols_shrink = cols_noshrink + 1;
    let cell_w_shrink = (avail_w - (cols_shrink as f32 - 1.0) * spacing) / cols_shrink as f32;

    let (columns, cell_w) = if cell_w_shrink >= 150.0 {
        (cols_shrink, cell_w_shrink)
    } else {
        (cols_noshrink, cell_w_noshrink)
    };

    let slots = 4usize.saturating_sub(app.covers.page_cover_loading.len());
    let to_load: Vec<(String, String, String)> = if slots > 0 {
        let visible_row = (app.last_scroll_offset / cell_h).floor() as usize;
        let visible_rows = (ui.available_height() / cell_h).ceil() as usize + 2;
        let preload = 2;
        let start_row = visible_row.saturating_sub(preload);
        let end_row = visible_row + visible_rows + preload;
        let start_idx = start_row * columns;
        let end_idx = (end_row * columns).min(app.reader.current_chapter.pages.len());

        let mut candidates: Vec<(usize, String, String, String)> = app.reader.current_chapter.pages[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, p)| (start_idx + i, p))
            .filter(|(_, p)| {
                let key = format!("{}::{}", p.archive_path, p.name);
                !app.covers.page_textures.contains_key(&key) && !app.covers.page_cover_loading.contains(&key)
            })
            .map(|(idx, p)| {
                let key = format!("{}::{}", p.archive_path, p.name);
                (idx, key, p.archive_path.clone(), p.name.clone())
            })
            .collect();

        let vis_start = visible_row * columns;
        let vis_end = (visible_row + visible_rows) * columns;
        candidates.sort_by_key(|(idx, _, _, _)| {
            if *idx >= vis_start && *idx < vis_end {
                0
            } else if *idx < vis_start {
                vis_start - idx
            } else {
                idx - vis_end
            }
        });

        candidates.into_iter()
            .take(slots)
            .map(|(_, key, archive_path, name)| (key, archive_path, name))
            .collect()
    } else {
        Vec::new()
    };

    for (key, archive_path, name) in &to_load {
        app.covers.page_cover_loading.insert(key.clone());
        let tx = app.covers.page_cover_tx.clone();
        let key = key.clone();
        let archive_path = archive_path.clone();
        let name = name.clone();
        crate::reader::worker::acquire_permit();
        std::thread::spawn(move || {
            let _guard = crate::reader::worker::PermitGuard::new();
            let result = if let Some((rgba, w, h)) = crate::cache::get_thumbnail(&key) {
                Some((rgba, w, h))
            } else {
                (|| -> Option<(Vec<u8>, i32, i32)> {
                    let archive = crate::archive::Archive::new(&archive_path).ok()?;
                    let data = archive.get_image_data(&name).ok()?;
                    let (rgba, w, h) =
                        crate::image_processing::ImageProcessor::decode_and_process(&data, true)?;
                    crate::cache::store_thumbnail(&key, &rgba, w, h);
                    Some((rgba, w, h))
                })()
            };
            let _ = tx.send(match result {
                Some((rgba, w, h)) => (key, rgba, w, h),
                None => (key, Vec::new(), 0, 0),
            });
        });
    }

    let mut clicked_idx: Option<usize> = None;
    let theme = &app.theme;

    let dir_path = match app.reader.nav_stack.last() {
        Some(crate::NavigationLevel::Pages { dir_path, .. }) => dir_path.clone(),
        _ => String::new(),
    };
    let cur_page = if dir_path.is_empty() || app.reader.current_chapter.archive_path.is_empty() {
        None
    } else {
        let latest = app.db.as_ref()
            .and_then(|db| db.load_latest_progress(&dir_path).ok().flatten());
        let ahead = match (&latest, app.lib.chapter_idx.get(&app.reader.current_chapter.archive_path)) {
            (Some((lap, _, _)), Some(&ci)) => app.lib.chapter_idx.get(lap).map_or(false, |&li| ci > li),
            _ => false,
        };
        if ahead {
            None
        } else {
            app.db.as_ref()
                .and_then(|db| db.load_progress(&dir_path, &app.reader.current_chapter.archive_path).ok().flatten())
                .map(|(pi, _)| pi)
        }
    };

    let scroll_out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        let mut i = 0;
        while i < app.reader.current_chapter.pages.len() {
            ui.horizontal(|ui| {
                let remaining = app.reader.current_chapter.pages.len() - i;
                let cols = columns.min(remaining);
                for j in 0..cols {
                    let idx = i + j;
                    let page = &app.reader.current_chapter.pages[idx];
                    let key = format!("{}::{}", page.archive_path, page.name);

                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(cell_w, cell_h), egui::Sense::click());

                    ui.painter()
                        .rect_filled(rect, theme.cell_radius, theme.cell_bg);

                    if let Some(tex) = app.covers.page_textures.get(&key) {
                        let tex_aspect = tex.aspect_ratio();
                        let cell_aspect = rect.width() / rect.height();

                        let uv = if tex_aspect > cell_aspect {
                            let uv_w = cell_aspect / tex_aspect;
                            let offset = (1.0 - uv_w) / 2.0;
                            egui::Rect::from_min_max(
                                egui::pos2(offset, 0.0),
                                egui::pos2(1.0 - offset, 1.0),
                            )
                        } else {
                            let uv_h = tex_aspect / cell_aspect;
                            let offset = (1.0 - uv_h) / 2.0;
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, offset),
                                egui::pos2(1.0, 1.0 - offset),
                            )
                        };

                        ui.painter().add(
                            egui::epaint::RectShape::filled(rect, theme.cell_radius, egui::Color32::WHITE)
                                .with_texture(tex.id(), uv),
                        );
                    } else {
                        ui.painter()
                            .rect_filled(rect, theme.cover_radius, theme.placeholder_bg);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "📄",
                            egui::FontId::proportional(24.0),
                            egui::Color32::GRAY,
                        );
                    }

                    if cur_page.map_or(false, |pi| idx < pi) {
                        ui.painter().rect_filled(
                            rect,
                            theme.cell_radius,
                            egui::Color32::from_black_alpha(120),
                        );
                    }

                    let overlay_h = 30.0;
                    let overlay_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - overlay_h),
                        rect.right_bottom(),
                    );
                    ui.painter().rect_filled(
                        overlay_rect,
                        theme.label_overlay_radius,
                        theme.label_overlay_bg,
                    );

                    let overlay_fg = if response.hovered() {
                        egui::Color32::from_rgb(0xad, 0xb1, 0xb8)
                    } else {
                        theme.label_overlay_fg
                    };

                    ui.put(
                        overlay_rect,
                        egui::Label::new(
                            egui::RichText::new(app.lang.page_label(idx + 1)).color(overlay_fg).size(14.0),
                        )
                        .halign(egui::Align::Center)
                        .truncate()
                        .selectable(false),
                    );

                    if response.clicked() {
                        clicked_idx = Some(idx);
                    }
                }
            });
            ui.add_space(theme.row_gap as f32);
            i += columns;
        }
    });

    app.last_scroll_offset = scroll_out.state.offset.y;

    if let Some(idx) = clicked_idx {
        let dir_path = match app.reader.nav_stack.last() {
            Some(crate::NavigationLevel::Pages { dir_path, .. }) => dir_path.clone(),
            _ => String::new(),
        };
        let archive_path = &app.reader.current_chapter.pages[idx].archive_path;
        let start_idx = idx;
        if let Some(ref db) = app.db {
            let _ = db.save_progress(&dir_path, archive_path, start_idx, app.reader.current_chapter.pages.len());
        }
        let detected = if app.lib.manga_list.iter().any(|m| {
            m.auto_mode.is_none() && m.archives.iter().any(|a| *a == *archive_path)
        }) {
            app.current_archive.as_ref().and_then(|archive| {
                let images = archive.get_images().ok()?;
                let dims: Vec<(u32, u32)> = images.iter().take(3)
                    .filter_map(|name| archive.get_image_dimensions(name))
                    .collect();
                if dims.len() >= 3 {
                    Some(crate::reader::auto_detect::detect_from_pages(
                            &dims,
                            app.settings.webtoon.aspect_ratio_threshold,
                            app.settings.webtoon.auto_detect_majority_factor,
                    ))
                } else { None }
            })
        } else { None };

        if let Some(mode) = detected {
            if let Some(manga) = app.lib.manga_list.iter_mut().find(|m| {
                m.archives.iter().any(|a| *a == *archive_path)
            }) {
                manga.auto_mode = Some(mode);
                if let Some(ref db) = app.db {
                    let _ = db.save_manga_settings(&manga.directory, manga.manual_mode, manga.auto_mode, manga.manual_rtl, manga.auto_rtl);
                }
            }
        }

        let mode = app.lib.manga_list.iter()
            .find(|m| m.archives.iter().any(|a| *a == *archive_path))
            .map(|m| m.effective_mode())
            .unwrap_or(crate::reader::mode::ReaderMode::Manga);

        if mode == crate::reader::mode::ReaderMode::Webtoon {
            app.reader.nav_stack.push(crate::NavigationLevel::ReadingWebtoon {
                archive_path: archive_path.clone(),
                dir_path,
            });
        } else {
            app.reader.nav_stack.push(crate::NavigationLevel::Reading {
                archive_path: archive_path.clone(),
                page_index: start_idx,
                dir_path,
            });
        }
        if mode == crate::reader::mode::ReaderMode::Webtoon {
            let seek_id = egui::Id::new(("webtoon_seek", archive_path.as_str()));
            ui.ctx().data_mut(|d| d.insert_persisted(seek_id, start_idx));
        }

        app.last_scroll_offset = 0.0;
        app.last_scroll_offset = 0.0;
        if !app.reader.current_chapter.cache.contains_key(&start_idx) && !app.reader.current_chapter.loading.contains(&start_idx) {
            app.reader.current_chapter.loading.insert(start_idx);
            let _ = crate::reader::worker::decode_tx().send(
                crate::reader::worker::DecodeJob {
                    ap: archive_path.to_string(),
                    n: start_idx,
                    page_name: app.reader.current_chapter.pages[start_idx].name.clone(),
                    max_decode_width: 1920,
                    arcs: app.reader.current_chapter.archive.clone(),
                    tx: app.reader.reader_tx.clone(),
                    use_max_width: true,
                }
            );
        }
    }
}
