use eframe::egui;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    if app.lib.chapters.is_empty() {
        ui.label(app.lang.chapters_none);
        return;
    }

    if let Some(err) = &app.status_error {
        ui.colored_label(egui::Color32::from_rgb(0xe8, 0x6a, 0x6a), err);
        ui.add_space(8.0);
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

    let slots = 4usize.saturating_sub(app.covers.chapter_cover_loading.len());
    let to_load: Vec<(u64, String)> = if slots > 0 {
        let visible_row = (app.last_scroll_offset / cell_h).floor() as usize;
        let visible_rows = (ui.available_height() / cell_h).ceil() as usize + 2;
        let preload = 2;
        let start_row = visible_row.saturating_sub(preload);
        let end_row = visible_row + visible_rows + preload;
        let start_idx = start_row * columns;
        let end_idx = (end_row * columns).min(app.lib.chapters.len());

        let mut candidates: Vec<(usize, u64, String)> = app.lib.chapters[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, c)| (start_idx + i, c))
            .filter(|(_, c)| {
                !app.covers.chapter_textures.contains_key(&c.id)
                    && !app.covers.chapter_cover_loading.contains(&c.id)
            })
            .map(|(idx, c)| (idx, c.id, c.path.clone()))
            .collect();

        let vis_start = visible_row * columns;
        let vis_end = (visible_row + visible_rows) * columns;
        candidates.sort_by_key(|(idx, _, _)| {
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
            .map(|(_, id, p)| (id, p))
            .collect()
    } else {
        Vec::new()
    };

    for (id, path) in &to_load {
        app.covers.chapter_cover_loading.insert(*id);

        if let Some((rgba, w, h)) = crate::cache::get_thumbnail(path) {
            let img =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ui.ctx().load_texture("ch_cvr", img, egui::TextureOptions::default());
            app.covers.chapter_textures.insert(*id, tex);
            app.covers.chapter_cover_loading.remove(id);
        } else {
            let tx = app.covers.chapter_cover_tx.clone();
            let id = *id;
            let path = path.clone();
            crate::reader::worker::acquire_permit();
            std::thread::spawn(move || {
                let _guard = crate::reader::worker::PermitGuard::new();
                let result = (|| -> Option<(Vec<u8>, i32, i32)> {
                    let archive = crate::archive::Archive::new(&path).ok()?;
                    let data = archive.get_first_image().ok()?;
                    let (rgba, w, h) =
                        crate::image_processing::ImageProcessor::decode_and_process_from_top(
                            &data, true,
                        )?;
                    crate::cache::store_thumbnail(&path, &rgba, w, h);
                    Some((rgba, w, h))
                })();
                let _ = tx.send(match result {
                    Some((rgba, w, h)) => (id, rgba, w, h),
                    None => (id, Vec::new(), 0, 0),
                });
            });
        }
    }

    let mut clicked_idx: Option<usize> = None;
    let theme = &app.theme;

    let dir_path = match app.reader.nav_stack.last() {
        Some(crate::NavigationLevel::Chapters { dir_path }) => dir_path.clone(),
        _ => String::new(),
    };
    let cur_idx = if dir_path.is_empty() {
        None
    } else {
        app.db.as_ref()
            .and_then(|db| db.load_latest_progress(&dir_path).ok().flatten())
            .and_then(|(p, _, _)| app.lib.chapter_idx.get(&p).copied())
    };

    let scroll_out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        let mut i = 0;
        while i < app.lib.chapters.len() {
            ui.horizontal(|ui| {
                let remaining = app.lib.chapters.len() - i;
                let cols = columns.min(remaining);
                for j in 0..cols {
                    let idx = i + j;
                    let chapter = &app.lib.chapters[idx];

                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(cell_w, cell_h), egui::Sense::click());

                    ui.painter()
                        .rect_filled(rect, theme.cell_radius, theme.cell_bg);

                    if let Some(tex) = app.covers.chapter_textures.get(&chapter.id) {
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

                    if cur_idx.map_or(false, |ci| idx < ci) {
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
                            egui::RichText::new(app.lang.chapter_label(idx + 1))
                                .color(overlay_fg)
                                .size(14.0),
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
        crate::archive::clear_archive_cache();
        let path = app.lib.chapters[idx].path.clone();
        app.last_scroll_offset = 0.0;
        app.covers.page_textures.clear();
        app.covers.page_cover_loading.clear();
        app.covers.page_cover_order.clear();
        let dir_path = match app.reader.nav_stack.last() {
            Some(crate::NavigationLevel::Chapters { dir_path }) => dir_path.clone(),
            _ => String::new(),
        };
        let Some(archive) = crate::archive::Archive::new(&path).ok() else {
            app.status_error = Some(app.lang.msg_err_archive.to_string());
            return;
        };
        let pages = app.db.as_ref().and_then(|db| db.load_pages(&path).ok())
            .filter(|p| !p.is_empty())
            .or_else(|| {
                crate::scanner::get_pages(&path).ok().map(|p| {
                    if let Some(db) = app.db.as_ref() {
                        db.save_pages(&path, &p).ok();
                    }
                    p
                })
            });
        let Some(p) = pages else {
            app.status_error = Some(app.lang.msg_err_archive.to_string());
            return;
        };
        app.status_error = None;
        app.reader.next_switch = crate::reader::chapter_switch::NextChapterState::new();
        app.reader.prev_switch = crate::reader::chapter_switch::NextChapterState::new();
        app.current_archive = Some(archive);
        if let Some(ref archive) = app.current_archive {
            app.reader.current_chapter.archive = archive.shared_arcs().ok();
        }
        app.reader.current_chapter.archive_path = path.clone();
        app.reader.current_chapter.loading.clear();
        app.reader.current_chapter.cache_clear(); 
        app.reader.nav_stack
            .push(crate::NavigationLevel::Pages {
                archive_path: path.clone(),
                dir_path,
            });
        let page_count = p.len();
        let s = crate::config::settings::ReaderSettings::load();
        app.reader.current_chapter.heights = vec![s.webtoon.default_page_height; page_count];
        app.reader.current_chapter.aspect_ratios = vec![0.0; page_count];
        if let Some(ref archive) = app.current_archive {
            for (i, page) in p.iter().enumerate() {
                if let Ok(header) = archive.get_image_data_limited(&page.name, 256) {
                    if let Some((w, h)) = crate::image_processing::decode_dimensions(&header) {
                        let rw = if s.webtoon_width > 0.0 { s.webtoon_width } else { s.webtoon.min_width };
                        app.reader.current_chapter.aspect_ratios[i] = w as f32 / h as f32;
                        app.reader.current_chapter.heights[i] = rw * h as f32 / w as f32;
                    }
                }
            }
        }
        app.reader.current_chapter.pages = p;
        #[cfg(feature = "fx")]
        app.reader.fx_load_chapter(&app.lib, &path);
    }
}
