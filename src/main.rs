mod theme;
mod archive;
mod cache;
mod config;
mod cover;
mod database;
mod image_processing;
mod scanner;
mod services;
mod ui;
mod reader;
mod audio;
#[cfg(feature = "fx")]
mod fx;
mod lang;

pub use reader::ChapterBundle;

mod library_state;

use eframe::egui;
use crate::theme::AppTheme;
use crate::services::{start_scan, ScanEvent};
use crate::config::storage::DirectoriesConfig;
use crate::config::settings::ReaderSettings;
use crate::cover::cover_system::CoverSystem;
use crate::library_state::LibraryState;
use std::sync::mpsc;
use std::sync::Arc;
use std::collections::HashMap;

use crate::reader::ReaderState;

#[derive(Clone, Copy, PartialEq)]
pub enum AppView {
    Library, Recent, Language, Theme, Settings,
}

#[derive(Clone)]
pub enum NavigationLevel {
    Titles,
    Chapters { dir_path: String },
    Pages { archive_path: String, dir_path: String },
    Reading { archive_path: String, page_index: usize, dir_path: String },
    ReadingWebtoon { archive_path: String, dir_path: String },
}

pub struct SpessartineApp {
    pub current_view: AppView,
    pub sidebar_visible: bool,
    pub scan_receiver: Option<mpsc::Receiver<ScanEvent>>,
    pub scan_started: bool,
    pub scan_status: String,
    pub lib: LibraryState,
    pub covers: CoverSystem,
    pub lang: crate::lang::Lang,
    pub theme: AppTheme,
    pub settings: ReaderSettings,
    pub theme_applied: bool,
    pub last_scroll_offset: f32,
    pub current_archive: Option<crate::archive::Archive>,
    pub reader: ReaderState,
    pub new_dir_input: String,
    pub db: Option<crate::database::MangaDb>,
    pub pending_reader_mode: Option<(u64, Option<crate::reader::mode::ReaderMode>)>,
    pub recent_chapters_cache: HashMap<String, (std::time::Instant, crate::ui::stack::recent::ChapterSizes)>,
    pub recent_items_cache: Option<(std::time::Instant, Vec<crate::database::RecentItem>)>,
    pub status_error: Option<String>,
}

impl SpessartineApp {
    pub fn new() -> Self {
        let settings = ReaderSettings::load();
        #[cfg(feature = "fx")]
        {
            if settings.audio.soundpack_dir.is_empty() {
                crate::fx::resolve::reset_dir();
            } else {
                crate::fx::resolve::set_dir(settings.audio.soundpack_dir.clone().into());
            }
            crate::audio::set_master_volume(settings.audio.volume);
            crate::audio::set_master_muted(settings.audio.muted);
        }
        let lang = crate::lang::Lang::from_code(&settings.lang_code);
        Self {
            current_view: AppView::Library,
            sidebar_visible: true,
            scan_receiver: None,
            scan_started: false,
            scan_status: lang.status_empty.to_string(),
            lib: LibraryState::new(),
            covers: CoverSystem::new(),
            theme: crate::config::load_theme(),
            settings,
            theme_applied: false,
            lang,
            last_scroll_offset: 0.0,
            current_archive: None,
            reader: ReaderState::new(),
            new_dir_input: String::new(),
            db: crate::database::MangaDb::open().ok(),
            pending_reader_mode: None,
            recent_chapters_cache: HashMap::new(),
            recent_items_cache: None,
            status_error: None,
        }
    }

    pub fn trigger_rescan(&mut self) {
        self.scan_started = false;
        self.lib.manga_list.clear();
    }
}

impl eframe::App for SpessartineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let start = std::time::Instant::now();

        if !self.theme_applied {
            self.theme.apply_to_ctx(ctx);
            self.theme_applied = true;
        }

        #[cfg(feature = "fx")]
        {
            crate::audio::set_master_volume(self.settings.audio.volume);
            crate::audio::set_master_muted(self.settings.audio.muted);
        }

        if self.current_view == AppView::Library && !self.scan_started {
            let (tx, rx) = mpsc::channel();
            let directories = DirectoriesConfig::load().get_directories();
            start_scan(tx, directories);
            self.scan_receiver = Some(rx);
            self.scan_started = true;
            self.scan_status = self.lang.status_scanning.to_string();
        }

        if let Some(ref rx) = self.scan_receiver {
            if let Ok(event) = rx.try_recv() {
                match event {
                    ScanEvent::Completed(list) => {
                        if let Some(ref db) = self.db {
                            let settings = db.load_manga_settings().ok().unwrap_or_default();
                            let merged: Vec<crate::scanner::MangaItem> = list.into_iter().map(|mut m| {
                                if let Some((_, man_mode, auto_mode, man_rtl, auto_rtl)) = settings.iter().find(|(dir, _, _, _, _)| *dir == m.directory) {
                                    m.manual_mode = *man_mode;
                                    m.auto_mode = *auto_mode;
                                    m.manual_rtl = *man_rtl;
                                    m.auto_rtl = *auto_rtl;
                                }
                                m
                            }).collect();
                            db.save_manga_list(&merged).ok();
                            self.lib.manga_list = merged;
                        } else {
                            self.lib.manga_list = list;
                        }
                        self.scan_status = self.lang.found_mangas(self.lib.manga_list.len());
                        self.recent_chapters_cache.clear();
                        self.scan_receiver = None;
                    }
                    ScanEvent::Failed(e) => {
                        self.scan_status = self.lang.error_msg(&e);
                    }
                }
            }
        }

        let in_reading = matches!(self.reader.nav_stack.last(),
            Some(&NavigationLevel::Reading { .. }) | Some(&NavigationLevel::ReadingWebtoon { .. }));

        let t1 = start.elapsed().as_micros() as f32 / 1000.0;
        if in_reading {
            self.reader.handle_textures(ctx, &self.settings, &mut self.lib, &mut self.db);
            self.reader.preload_adjacent(&self.settings, &self.lib);
            self.reader.handle_keyboard(ctx, &self.settings, &self.lib, &mut self.db, &mut self.sidebar_visible);
            self.reader.preload_chapters(&self.settings, &self.lib);
        } else {
            self.covers.handle_textures(ctx, &self.settings);
        }
        let t2 = start.elapsed().as_micros() as f32 / 1000.0;

        self.reader.switch_mode(ctx, &mut self.settings, &mut self.pending_reader_mode);
        let t0 = start.elapsed().as_micros() as f32 / 1000.0;

        if !self.covers.cover_loading.is_empty()
            || !self.covers.chapter_cover_loading.is_empty()
            || !self.covers.page_cover_loading.is_empty()
            || !self.reader.current_chapter.loading.is_empty()
            || self.scan_receiver.is_some()
            || !self.reader.prev_chapter.loading.is_empty()
            || !self.reader.joined_chapter.loading.is_empty()
            || {
                #[cfg(feature = "fx")]
                {
                    self.reader.fx_player.active()
                }
                #[cfg(not(feature = "fx"))]
                {
                    false
                }
            }
        {
            ctx.request_repaint();
        }

        crate::ui::draw(self, ctx);

        let frame_ms = start.elapsed().as_micros() as f32 / 1000.0;
        if frame_ms > 100.0 {
            println!("LAG: {:.0}ms reader={:.0} mode={:.0} draw={:.0} nav={:?}",
                frame_ms, t2-t1, t0-t2, frame_ms - t0,
                self.reader.nav_stack.last().map(|l| match l {
                    NavigationLevel::Titles => "Titles",
                    NavigationLevel::Chapters { .. } => "Chapters",
                    NavigationLevel::Pages { .. } => "Pages",
                    NavigationLevel::Reading { .. } => "Reading",
                    NavigationLevel::ReadingWebtoon { .. } => "RW",
                }));
            println!("LAG: .loading: cur={} prev={} joined={} pcover={}",
                self.reader.current_chapter.loading.len(),
                self.reader.prev_chapter.loading.len(),
                self.reader.joined_chapter.loading.len(),
                self.covers.page_cover_loading.len());
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::archive::clear_archive_cache();
        self.reader.current_chapter.cache_clear();
        self.reader.prev_chapter.clear();
        self.reader.joined_chapter.clear();
        self.reader.next_switch = crate::reader::chapter_switch::NextChapterState::new();
        self.reader.prev_switch = crate::reader::chapter_switch::NextChapterState::new();
        self.covers.cover_textures.clear();
        self.covers.chapter_textures.clear();
        self.covers.page_textures.clear();
        #[cfg(feature = "fx")]
        crate::audio::stop_all();
        self.db.take();
    }
}

fn load_icon() -> Arc<egui::IconData> {
    let png = include_bytes!("../.spessartine/assets/icons/spessartine.png");
    let img = image::load_from_memory(png).expect("failed to decode icon PNG");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Arc::new(egui::IconData { rgba: rgba.into_vec(), width, height })
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 800.0])
            .with_title(concat!("Spessartine ", env!("CARGO_PKG_VERSION")))
            .with_icon(load_icon()),
        renderer: eframe::Renderer::Glow,
        vsync: cfg!(target_os = "windows"),
        depth_buffer: 0,
        stencil_buffer: 0,
        multisampling: 0,
        ..Default::default()
    };

    eframe::run_native(
        "Spessartine",
        options,
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals.dark_mode = true;
            cc.egui_ctx.set_style(style);
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "nerd".into(),
                Arc::new(egui::FontData::from_static(
                    include_bytes!("../.spessartine/subsets/SymbolsNerdFontMono-Sub.ttf")
                )),
            );
            for family in fonts.families.values_mut() {
                family.insert(0, "nerd".into());
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(SpessartineApp::new()))
        }),
    )
}
