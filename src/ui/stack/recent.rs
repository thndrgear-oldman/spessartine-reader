use eframe::egui;
use std::collections::HashMap;
use std::time::Instant;
use crate::database::RecentItem;

const CELL_H: f32 = 250.0;
const CELL_BASE_W: f32 = 170.0;
const CELL_MIN_W: f32 = 150.0;
const SIZES_CACHE_CAP: usize = 1024;
const RECENT_CACHE_TTL_MS: u128 = 500;

#[derive(Clone)]
pub struct ChapterSizes {
    pub order: Vec<String>,
    pub sizes: Vec<u64>,
    pub total: u64,
}

impl ChapterSizes {
    fn build(manga_dir: &str) -> Option<ChapterSizes> {
        let chapters = crate::scanner::get_chapters(manga_dir).ok()?;
        if chapters.is_empty() {
            return None;
        }
        let mut cs = ChapterSizes {
            order: Vec::with_capacity(chapters.len()),
            sizes: Vec::with_capacity(chapters.len()),
            total: 0,
        };
        for c in &chapters {
            cs.order.push(c.path.clone());
            let len = std::fs::metadata(&c.path).map(|m| m.len()).unwrap_or(0);
            cs.sizes.push(len);
            cs.total = cs.total.saturating_add(len);
        }
        if cs.total == 0 {
            return None;
        }
        Some(cs)
    }

    pub fn pct(&self, chapter_path: &str, page_index: usize, total_pages: usize) -> Option<f32> {
        let pos = self.order.iter().position(|p| p == chapter_path)?;
        let before: u64 = self.sizes[..pos].iter().sum();
        let cur = self.sizes[pos];
        let frac = if total_pages > 0 {
            ((page_index + 1) as f32 / total_pages as f32).min(1.0)
        } else {
            0.0
        };
        let read = before as f64 + cur as f64 * frac as f64;
        Some((read / self.total as f64) as f32)
    }
}

pub(crate) fn manga_progress(app: &mut crate::SpessartineApp, item: &crate::database::RecentItem) -> f32 {
    let fallback = if item.total_pages > 0 {
        ((item.page_index + 1) as f32 / item.total_pages as f32).min(1.0)
    } else {
        0.0
    };
    get_manga_sizes(app, &item.manga_directory)
        .and_then(|cs| cs.pct(&item.chapter_path, item.page_index, item.total_pages))
        .unwrap_or(fallback)
}

pub(crate) fn recent_items(app: &mut crate::SpessartineApp) -> Vec<crate::database::RecentItem> {
    let now = Instant::now();
    if let Some((t, items)) = &app.recent_items_cache {
        if now.duration_since(*t).as_millis() < RECENT_CACHE_TTL_MS {
            return items.clone();
        }
    }
    let items = app
        .db
        .as_ref()
        .and_then(|db| db.load_recent().ok())
        .unwrap_or_default();
    app.recent_items_cache = Some((now, items.clone()));
    items
}

fn get_manga_sizes<'a>(app: &'a mut crate::SpessartineApp, manga_dir: &str) -> Option<&'a ChapterSizes> {
    if app.recent_chapters_cache.contains_key(manga_dir) {
        return app.recent_chapters_cache.get(manga_dir).map(|(_, c)| c);
    }
    let cs = ChapterSizes::build(manga_dir)?;
    if app.recent_chapters_cache.len() >= SIZES_CACHE_CAP {
        app.recent_chapters_cache.clear();
    }
    app.recent_chapters_cache.insert(manga_dir.to_string(), (Instant::now(), cs));
    app.recent_chapters_cache.get(manga_dir).map(|(_, c)| c)
}

fn grid_dims(ui: &egui::Ui) -> (usize, f32) {
    let spacing = ui.spacing().item_spacing.x;
    let avail_w = ui.available_width();

    let cols_noshrink = ((avail_w + spacing) / (CELL_BASE_W + spacing)).floor().max(1.0) as usize;
    let cell_w_noshrink = (avail_w - (cols_noshrink as f32 - 1.0) * spacing) / cols_noshrink as f32;
    let cols_shrink = cols_noshrink + 1;
    let cell_w_shrink = (avail_w - (cols_shrink as f32 - 1.0) * spacing) / cols_shrink as f32;

    if cell_w_shrink >= CELL_MIN_W {
        (cols_shrink, cell_w_shrink)
    } else {
        (cols_noshrink, cell_w_noshrink)
    }
}

fn load_covers(app: &mut crate::SpessartineApp, ui: &egui::Ui, items: &[RecentItem]) {
    let slots = 4usize.saturating_sub(app.covers.cover_loading.len());
    if slots == 0 {
        return;
    }

    let mut to_load: Vec<(u64, String)> = Vec::new();
    for it in items {
        if to_load.len() >= slots {
            break;
        }
        let key = it.manga_id as u64;
        if app.covers.cover_textures.contains_key(&key) || app.covers.cover_loading.contains(&key) {
            continue;
        }
        if let Some(path) = it.cover_path.clone() {
            to_load.push((key, path));
        }
    }

    for (id, path) in to_load {
        app.covers.cover_loading.insert(id);

        let cache_key = format!("title_{}", path);
        if let Some((rgba, w, h)) = crate::cache::get_thumbnail(&cache_key) {
            let img =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ui.ctx().load_texture("cvr", img, egui::TextureOptions::default());
            app.covers.cover_textures.insert(id, tex);
            app.covers.cover_loading.remove(&id);
        } else {
            let tx = app.covers.cover_tx.clone();
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
}

fn section(
    app: &mut crate::SpessartineApp,
    ui: &mut egui::Ui,
    title: &str,
    items: &[RecentItem],
    columns: usize,
    cell_w: f32,
    pct: &HashMap<String, f32>,
    clicked: &mut Option<RecentItem>,
) {
    if items.is_empty() {
        return;
    }

    let cell_radius = app.theme.cell_radius;
    let cover_radius = app.theme.cover_radius;
    let cell_bg = app.theme.cell_bg;
    let label_overlay_bg = app.theme.label_overlay_bg;
    let label_overlay_fg = app.theme.label_overlay_fg;
    let label_overlay_radius = app.theme.label_overlay_radius;
    let placeholder_bg = app.theme.placeholder_bg;
    let row_gap = app.theme.row_gap;

    ui.add_space(20.0);
    ui.label(
        egui::RichText::new(title)
            .size(16.0)
            .color(label_overlay_fg),
    );
    ui.add_space(8.0);

    let mut i = 0;
    while i < items.len() {
        ui.horizontal(|ui| {
            let remaining = items.len() - i;
            let cols = columns.min(remaining);
            for j in 0..cols {
                let item = &items[i + j];

                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(cell_w, CELL_H), egui::Sense::click());

                ui.painter()
                    .rect_filled(rect, cell_radius, cell_bg);

                if let Some(tex) = app.covers.cover_textures.get(&(item.manga_id as u64)) {
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

                let finished = pct.get(&item.chapter_path).copied().unwrap_or(0.0) >= 0.995;
                if finished {
                    ui.painter().rect_filled(
                        rect,
                        cell_radius,
                        egui::Color32::from_black_alpha(120),
                    );
                }

                let p = pct.get(&item.chapter_path).copied().unwrap_or(0.0);
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
                            rect.right() - galley.size().x - pad * 2.0 - margin,
                            rect.top() + margin,
                        ),
                        egui::pos2(
                            rect.right() - margin,
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
                        egui::RichText::new(&item.manga_name).color(overlay_fg).size(14.0),
                    )
                    .halign(egui::Align::Center)
                    .truncate()
                    .selectable(false),
                );

                if response.clicked() {
                    *clicked = Some(item.clone());
                }
            }
        });
        ui.add_space(row_gap as f32);
        i += columns;
    }
}

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            ui.add_space(40.0);
            ui.heading(app.lang.heading_recent);

            let items = crate::ui::stack::recent::recent_items(app);

            let mut today: Vec<RecentItem> = Vec::new();
            let mut yesterday: Vec<RecentItem> = Vec::new();
            let mut earlier: Vec<RecentItem> = Vec::new();
            for it in items {
                if it.is_today {
                    today.push(it);
                } else if it.is_yesterday {
                    yesterday.push(it);
                } else {
                    earlier.push(it);
                }
            }

            if today.is_empty() && yesterday.is_empty() && earlier.is_empty() {
                ui.add_space(20.0);
                ui.label(app.lang.msg_recent_empty);
                return;
            }

            load_covers(app, ui, &today);
            load_covers(app, ui, &yesterday);
            load_covers(app, ui, &earlier);

            let (columns, cell_w) = grid_dims(ui);

            let mut pct_map: HashMap<String, f32> = HashMap::new();
            for it in today.iter().chain(yesterday.iter()).chain(earlier.iter()) {
                let p = manga_progress(app, it);
                pct_map.insert(it.chapter_path.clone(), p);
            }

            let mut clicked: Option<RecentItem> = None;
            let scroll_out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                section(app, ui, app.lang.recent_today, &today, columns, cell_w, &pct_map, &mut clicked);
                section(app, ui, app.lang.recent_yesterday, &yesterday, columns, cell_w, &pct_map, &mut clicked);
                section(app, ui, app.lang.recent_earlier, &earlier, columns, cell_w, &pct_map, &mut clicked);
            });
            app.last_scroll_offset = scroll_out.state.offset.y;

            if let Some(clicked) = clicked {
                crate::archive::clear_archive_cache();
                app.covers.chapter_textures.clear();
                app.covers.chapter_cover_loading.clear();
                app.covers.chapter_cover_order.clear();
                app.covers.page_textures.clear();
                app.covers.page_cover_loading.clear();
                app.covers.page_cover_order.clear();
                app.last_scroll_offset = 0.0;
                app.current_view = crate::AppView::Library;

                let dir = clicked.manga_directory.clone();
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

                let mode = app
                    .lib
                    .manga_list
                    .iter()
                    .find(|m| m.directory == clicked.manga_directory)
                    .map(|m| m.effective_mode())
                    .unwrap_or(crate::reader::mode::ReaderMode::Manga);

                let ap = clicked.chapter_path.clone();
                let dp = clicked.manga_directory.clone();
                if !std::path::Path::new(&ap).exists() {
                    app.status_error = Some(app.lang.msg_err_archive.to_string());
                    return;
                }
                app.status_error = None;
                app.reader.next_switch = crate::reader::chapter_switch::NextChapterState::new();
                app.reader.prev_switch = crate::reader::chapter_switch::NextChapterState::new();
                app.reader.jump_to_chapter(&ap, &app.settings, &app.lib);

                app.reader.nav_stack.clear();
                app.reader.nav_stack.push(crate::NavigationLevel::Titles);
                app.reader.nav_stack.push(crate::NavigationLevel::Chapters { dir_path: dp.clone() });
                app.reader.nav_stack.push(crate::NavigationLevel::Pages { archive_path: ap.clone(), dir_path: dp.clone() });

                if mode == crate::reader::mode::ReaderMode::Webtoon {
                    app.reader.nav_stack.push(crate::NavigationLevel::ReadingWebtoon {
                        archive_path: ap.clone(),
                        dir_path: dp,
                    });
                    ui.ctx().data_mut(|d| {
                        d.insert_persisted(
                            egui::Id::new(("webtoon_seek", ap.as_str())),
                            clicked.page_index,
                        );
                    });
                } else {
                    app.reader.nav_stack.push(crate::NavigationLevel::Reading {
                        archive_path: ap.clone(),
                        page_index: clicked.page_index,
                        dir_path: dp,
                    });
                }

                let idx = clicked.page_index;
                if idx < app.reader.current_chapter.pages.len()
                    && !app.reader.current_chapter.cache.contains_key(&idx)
                    && !app.reader.current_chapter.loading.contains(&idx)
                {
                    app.reader.current_chapter.loading.insert(idx);
                    let _ = crate::reader::worker::decode_tx().send(
                        crate::reader::worker::DecodeJob {
                            ap: ap.clone(),
                            n: idx,
                            page_name: app.reader.current_chapter.pages[idx].name.clone(),
                            max_decode_width: 1920,
                            arcs: app.reader.current_chapter.archive.clone(),
                            tx: app.reader.reader_tx.clone(),
                            use_max_width: true,
                        }
                    );
                }
            }
        });
}
