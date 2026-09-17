use eframe::egui;
use std::collections::HashMap;

mod title_context_menu;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    if app.lib.manga_list.is_empty() {
        ui.label(&app.scan_status);
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

    let slots = 4usize.saturating_sub(app.covers.cover_loading.len());
    let to_load: Vec<(u64, String)> = if slots > 0 {
        let visible_row = (app.last_scroll_offset / cell_h).floor() as usize;
        let visible_rows = (ui.available_height() / cell_h).ceil() as usize + 2;
        let preload = 2;
        let start_row = visible_row.saturating_sub(preload);
        let end_row = visible_row + visible_rows + preload;
        let start_idx = start_row * columns;
        let end_idx = (end_row * columns).min(app.lib.manga_list.len());

        let mut candidates: Vec<(usize, u64, String)> = app.lib.manga_list[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, m)| (start_idx + i, m))
            .filter(|(_, m)| {
                !app.covers.cover_textures.contains_key(&m.id)
                    && !app.covers.cover_loading.contains(&m.id)
            })
            .filter_map(|(idx, m)| m.archives.first().map(|p| (idx, m.id, p.clone())))
            .collect();

        let vis_start = visible_row * columns;
        let vis_end = (visible_row + visible_rows) * columns;
        candidates.sort_by_key(|(idx, _, _)| {
            if *idx >= vis_start && *idx < vis_end {
                0 // visible
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
        app.covers.cover_loading.insert(*id);

        let cache_key = format!("title_{}", path);
        if let Some((rgba, w, h)) = crate::cache::get_thumbnail(&cache_key) {
            let img =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ui.ctx().load_texture("cvr", img, egui::TextureOptions::default());
            app.covers.cover_textures.insert(*id, tex);
            app.covers.cover_loading.remove(id);
        } else {
            let tx = app.covers.cover_tx.clone();
            let id = *id;
            let path = path.clone();
            crate::reader::worker::acquire_permit();
            std::thread::spawn(move || {
                let _guard = crate::reader::worker::PermitGuard::new();
                let cache_key = format!("title_{}", path);
                let result = (|| -> Option<(Vec<u8>, i32, i32)> {
                    let (data, from_archive) = crate::cover::get_cover_data(&path)?;
                    crate::image_processing::ImageProcessor::decode_and_process(&data, from_archive)
                })();
                let _ = tx.send(match result {
                    Some((rgba, w, h)) => {
                        crate::cache::store_thumbnail(&cache_key, &rgba, w, h);
                        (id, rgba, w, h)
                    }
                    None => (id, Vec::new(), 0, 0),
                });
            });
        }
    }

    let mut clicked_idx: Option<usize> = None;
    let cell_radius = app.theme.cell_radius;
    let cover_radius = app.theme.cover_radius;
    let cell_bg = app.theme.cell_bg;
    let label_overlay_bg = app.theme.label_overlay_bg;
    let label_overlay_fg = app.theme.label_overlay_fg;
    let label_overlay_radius = app.theme.label_overlay_radius;
    let placeholder_bg = app.theme.placeholder_bg;
    let row_gap = app.theme.row_gap;

    let recent_items: Vec<crate::database::RecentItem> =
        crate::ui::stack::recent::recent_items(app);
    let recent_by_dir: HashMap<String, crate::database::RecentItem> = recent_items
        .into_iter()
        .map(|it| (it.manga_directory.clone(), it))
        .collect();

    let ws = (app.last_scroll_offset / cell_h).floor() as usize;
    let we = (ws + (ui.available_height() / cell_h).ceil() as usize + 4) * columns;
    let we = we.min(app.lib.manga_list.len());
    let window: Vec<(String, crate::database::RecentItem)> = app.lib.manga_list[ws * columns..we]
        .iter()
        .filter_map(|m| recent_by_dir.get(&m.directory).map(|it| (m.directory.clone(), it.clone())))
        .collect();
    let mut pct_map: HashMap<String, f32> = HashMap::new();
    for (dir, it) in window {
        pct_map.insert(dir, crate::ui::stack::recent::manga_progress(app, &it));
    }

    let scroll_out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        let mut i = 0;
        while i < app.lib.manga_list.len() {
            ui.horizontal(|ui| {
                let remaining = app.lib.manga_list.len() - i;
                let cols = columns.min(remaining);
                for j in 0..cols {
                    let idx = i + j;

                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(cell_w, cell_h), egui::Sense::click());

                    response.context_menu(|ui| {
                        title_context_menu::draw(app, idx, ui);
                    });

                    ui.painter()
                        .rect_filled(rect, cell_radius, cell_bg);

                    if let Some(tex) = app.covers.cover_textures.get(&app.lib.manga_list[idx].id) {
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
                            egui::epaint::RectShape::filled(rect, cell_radius, egui::Color32::WHITE)
                                .with_texture(tex.id(), uv),
                        );
                    } else {
                        ui.painter()
                            .rect_filled(rect, cover_radius, placeholder_bg);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "📖",
                            egui::FontId::proportional(24.0),
                            egui::Color32::GRAY,
                        );
                    }

                    crate::ui::reader::mode_badge::draw(
                        ui.painter(),
                        rect,
                        app.lib.manga_list[idx].effective_mode(),
                        app.lib.manga_list[idx].manual_mode.is_some(),
                    );

                    let p = pct_map.get(&app.lib.manga_list[idx].directory).copied().unwrap_or(0.0);
                    if p >= 0.995 {
                        ui.painter().rect_filled(
                            rect,
                            cell_radius,
                            egui::Color32::from_black_alpha(120),
                        );
                    }
                    if p > 0.0 {
                        let pct_i = (p * 100.0).round() as i32;
                        let galley = ui.painter().layout_no_wrap(
                            format!("{pct_i}%"),
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );
                        let pad = 3.0;
                        let margin = 5.0;
                        let badge_rect = egui::Rect::from_min_max(
                            egui::pos2(
                                rect.left() + margin,
                                rect.top() + margin,
                            ),
                            egui::pos2(
                                rect.left() + margin + galley.size().x + pad * 2.0,
                                rect.top() + margin + galley.size().y + pad * 2.0,
                            ),
                        );
                        ui.painter()
                            .rect_filled(badge_rect, 4.0, egui::Color32::from_black_alpha(150));
                        ui.painter().galley(
                            egui::pos2(badge_rect.left() + pad, badge_rect.top() + pad),
                            galley,
                            egui::Color32::WHITE,
                        );
                    }

                    let overlay_h = 30.0;
                    let overlay_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - overlay_h),
                        rect.right_bottom(),
                    );
                    ui.painter().rect_filled(
                        overlay_rect,
                        label_overlay_radius,
                        label_overlay_bg,
                    );

                    let overlay_fg = if response.hovered() {
                        egui::Color32::from_rgb(0xad, 0xb1, 0xb8)
                    } else {
                        label_overlay_fg
                    };

                    ui.put(
                        overlay_rect,
                        egui::Label::new(
                            egui::RichText::new(&app.lib.manga_list[idx].name).color(overlay_fg).size(14.0),
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
            ui.add_space(row_gap as f32);
            i += columns;
        }
    });

    app.last_scroll_offset = scroll_out.state.offset.y;

    if let Some(idx) = clicked_idx {
        crate::archive::clear_archive_cache();
        let dir = app.lib.manga_list[idx].directory.clone();
        app.last_scroll_offset = 0.0;
        app.status_error = None;
        app.covers.chapter_textures.clear();
        app.covers.chapter_cover_loading.clear();
        app.covers.chapter_cover_order.clear();
        app.covers.page_textures.clear();
        app.covers.page_cover_loading.clear();
        app.covers.page_cover_order.clear();
        app.reader.nav_stack
            .push(crate::NavigationLevel::Chapters { dir_path: dir.clone() });
        let chapters = app.db.as_ref().and_then(|db| db.load_chapters(&dir).ok())
            .filter(|c| !c.is_empty())
            .or_else(|| {
                crate::scanner::get_chapters(&dir).ok().map(|c| {
                    if let Some(db) = app.db.as_ref() {
                        db.save_chapters(&dir, &c).ok();
                    }
                    c
                })
            });
        if let Some(c) = chapters {
            app.lib.set_chapters(c);
        }
    }
}
