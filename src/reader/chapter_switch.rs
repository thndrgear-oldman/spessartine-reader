use std::sync::mpsc;
use crate::archive::SharedArchive;
use crate::scanner::PageItem;
use crate::reader::mode::ReaderMode;
use crate::reader::chapter_bundle::ChapterBundle;
use crate::NavigationLevel;
use crate::config::settings::ReaderSettings;
use crate::library_state::LibraryState;
use crate::reader::state::ReaderState;

pub struct NextChapterState {
    pub status: Status,
    tx: mpsc::Sender<Result<ReadyData, String>>,
    rx: mpsc::Receiver<Result<ReadyData, String>>,
}

 pub enum Status {
    Idle,
    Loading { progress: f32 },
    Ready(ReadyData),
    Error(String),
}

impl std::fmt::Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Idle => write!(f, "Idle"),
            Status::Loading { .. } => write!(f, "Loading"),
            Status::Ready(_) => write!(f, "Ready"),
            Status::Error(_) => write!(f, "Error"),
        }
    }
}

pub struct ReadyData {
    pub shared: SharedArchive,
    pub pages: Vec<PageItem>,
    pub archive_path: String,
    pub dir_path: String,
    pub target_page: usize,
    pub heights: Vec<f32>,
    pub aspect_ratios: Vec<f32>,
}

impl NextChapterState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { status: Status::Idle, tx, rx }
    }

pub fn trigger(&mut self, path: &str, dir_path: &str, ref_width: f32,
    header_size: u32, min_height: f32, def_height: f32) {
        self.status = Status::Loading { progress: 0.0 };
        let tx = self.tx.clone();
        let p = path.to_string();
        let d = dir_path.to_string();
        let hs = header_size;
        let mh = min_height;
        let dh = def_height;
        crate::reader::worker::acquire_permit();
        std::thread::spawn(move || {
            let result = (|| -> Result<ReadyData, String> {
                let archive = crate::archive::Archive::new(&p).map_err(|e| e.to_string())?;
                let pages = crate::scanner::get_pages(&p).map_err(|e| e.to_string())?;
                let shared = archive.shared_arcs().map_err(|e| e.to_string())?;
                let mut heights = vec![0.0f32; pages.len()];
                let mut aspect_ratios = vec![0.0f32; pages.len()];
                for (i, page) in pages.iter().enumerate() {
                    if let Ok(header) = archive.get_image_data_limited(&page.name, hs as usize) {
                        if let Some((w, h)) = crate::image_processing::decode_dimensions(&header) {
                            aspect_ratios[i] = w as f32 / h as f32;
                            heights[i] = ref_width * h as f32 / w as f32;
                        }
                    }
                    if heights[i] < mh { heights[i] = dh; }
                }
                Ok(ReadyData { shared, pages, archive_path: p, dir_path: d, target_page: 0, heights, aspect_ratios })
            })();
            let _ = tx.send(result);
            crate::reader::worker::release_permit();
        });
    }

pub fn trigger_prev(&mut self, path: &str, dir_path: &str, ref_width: f32,
    header_size: u32, min_height: f32, def_height: f32) {
        self.status = Status::Loading { progress: 0.0 };
        let tx = self.tx.clone();
        let p = path.to_string();
        let d = dir_path.to_string();
        let hs = header_size;
        let mh = min_height;
        let dh = def_height;
        crate::reader::worker::acquire_permit();
        std::thread::spawn(move || {
            let result = (|| -> Result<ReadyData, String> {
                let archive = crate::archive::Archive::new(&p).map_err(|e| e.to_string())?;
                let pages = crate::scanner::get_pages(&p).map_err(|e| e.to_string())?;
                let last = pages.len().saturating_sub(1);
                let shared = archive.shared_arcs().map_err(|e| e.to_string())?;
                let mut heights = vec![0.0f32; pages.len()];
                let mut aspect_ratios = vec![0.0f32; pages.len()];
                for (i, page) in pages.iter().enumerate() {
                    if let Ok(header) = archive.get_image_data_limited(&page.name, hs as usize) {
                        if let Some((w, h)) = crate::image_processing::decode_dimensions(&header) {
                            aspect_ratios[i] = w as f32 / h as f32;
                            heights[i] = ref_width * h as f32 / w as f32;
                        }
                    }
                    if heights[i] < mh { heights[i] = dh; }
                }
                Ok(ReadyData { shared, pages, archive_path: p, dir_path: d, target_page: last, heights, aspect_ratios })
            })();
            let _ = tx.send(result);
            crate::reader::worker::release_permit();
        });
    }

    pub fn poll(&mut self) {
        match self.rx.try_recv() {
            Ok(Ok(data)) => { self.status = Status::Ready(data); }
            Ok(Err(e)) => { self.status = Status::Error(e); }
            Err(_) => {}
        }
    }

    pub fn take_ready(&mut self) -> Option<ReadyData> {
        match std::mem::replace(&mut self.status, Status::Idle) {
            Status::Ready(data) => Some(data),
            other => { self.status = other; None }
        }
    }
}

pub fn is_next_section(pointer_x: f32, mid_x: f32, rtl: bool) -> bool {
    if rtl { pointer_x < mid_x } else { pointer_x >= mid_x }
}

pub fn is_prev_section(pointer_x: f32, mid_x: f32, rtl: bool) -> bool {
    if rtl { pointer_x >= mid_x } else { pointer_x < mid_x }
}

pub fn is_at_chapter_end(page_index: usize, total: usize, mode: ReaderMode) -> bool {
    match mode {
        ReaderMode::Double => page_index + 2 >= total,
        _ => page_index + 1 >= total,
    }
}

pub fn page_step(mode: ReaderMode) -> usize {
    match mode { ReaderMode::Double => 2, _ => 1 }
}

pub fn is_at_chapter_start(page_index: usize) -> bool {
    page_index == 0
}

pub fn take_switch(
    next: &mut NextChapterState,
    prev: &mut NextChapterState,
    current: &mut ChapterBundle,
    prev_ch: &mut ChapterBundle,
    joined: &mut ChapterBundle,
    reader_tx: &mpsc::Sender<(String, usize, Vec<u8>, i32, i32, bool)>,
    settings: &ReaderSettings,
    forward: bool,
) -> Option<(String, String, usize)> {
    let data = {
        let state = if forward { &mut *next } else { &mut *prev };
        state.take_ready()?
    };
    current.archive = Some(data.shared);
    current.pages = data.pages;
    current.aspect_ratios = data.aspect_ratios;
    crate::audio::page_turn(settings.audio.muted, settings.audio.volume);
    current.archive_path = data.archive_path.clone();
    current.cache_clear();
    current.loading.clear();
    prev_ch.clear();
    joined.clear();
    let n = data.target_page;
    if !current.cache.contains_key(&n) && !current.loading.contains(&n) {
        current.loading.insert(n);
        let _ = crate::reader::worker::decode_tx().send(crate::reader::worker::DecodeJob {
            ap: data.archive_path.clone(),
            n,
            page_name: current.pages[n].name.clone(),
            max_decode_width: settings.reader.max_decode_width,
            arcs: current.archive.clone(),
            tx: reader_tx.clone(),
            use_max_width: false,
        });
    }
    *next = NextChapterState::new();
    *prev = NextChapterState::new();
    Some((data.archive_path, data.dir_path, data.target_page))
}

pub fn request_switch(
    next: &mut NextChapterState,
    prev: &mut NextChapterState,
    lib: &LibraryState,
    settings: &ReaderSettings,
    archive_path: &str,
    dir_path: &str,
    forward: bool,
) {
    let neighbor = if forward {
        lib.chapter_idx.get(archive_path).copied()
            .and_then(|i| lib.chapters.get(i + 1))
            .map(|c| c.path.clone())
    } else {
        lib.chapter_idx.get(archive_path).copied()
            .and_then(|i| if i > 0 { lib.chapters.get(i - 1) } else { None })
            .map(|c| c.path.clone())
    };
    let Some(np) = neighbor else { return };
    if forward {
        if matches!(next.status, Status::Idle) {
            next.trigger(
                &np, dir_path,
                settings.reader.min_content_width,
                settings.reader.dimension_header_read_size,
                1.0,
                settings.webtoon.default_page_height,
            );
        }
    } else if matches!(prev.status, Status::Idle) {
        prev.trigger_prev(
            &np, dir_path,
            settings.reader.min_content_width,
            settings.reader.dimension_header_read_size,
            1.0,
            settings.webtoon.default_page_height,
        );
    }
}

impl ReaderState {
    pub fn jump_to_chapter(&mut self, target_path: &str, settings: &ReaderSettings, lib: &LibraryState) {
        if self.current_chapter.archive_path == target_path && !self.current_chapter.pages.is_empty() {
            return;
        }

        if let Ok(archive_file) = crate::archive::Archive::new(target_path) {
                if let Ok(shared) = archive_file.shared_arcs() {
                self.current_chapter.archive = Some(shared);
                let pages = crate::scanner::get_pages(target_path).unwrap_or_default();
                if pages.is_empty() { return; }

                let dir = lib.manga_list.iter()
                    .find(|m| m.archives.iter().any(|a| a == target_path))
                    .map(|m| m.directory.clone())
                    .unwrap_or_default();

                self.current_chapter.pages = pages;
                self.current_chapter.archive_path = target_path.to_string();
                self.current_chapter.dir_path = dir.clone();
                self.current_chapter.cache_clear();
                self.current_chapter.loading.clear();
                self.prev_chapter.clear();
                self.joined_chapter.clear();
                self.next_switch = NextChapterState::new();
                self.prev_switch = NextChapterState::new();

                #[cfg(feature = "fx")]
                self.fx_load_chapter(lib, target_path);

                while self.reader_rx.try_recv().is_ok() {}

                let page_count = self.current_chapter.pages.len();
                let ref_width = if settings.webtoon_width > 0.0 {
                    settings.webtoon_width
                } else {
                    settings.webtoon.min_width
                };
                let mut heights = Vec::with_capacity(page_count);
                let mut aspect_ratios = Vec::with_capacity(page_count);
                for page in &self.current_chapter.pages {
                    if let Ok(header) = archive_file.get_image_data_limited(&page.name, 256) {
                        if let Some((w, h)) = crate::image_processing::decode_dimensions(&header) {
                            aspect_ratios.push(w as f32 / h as f32);
                            heights.push(ref_width * h as f32 / w as f32);
                            continue;
                        }
                    }
                    aspect_ratios.push(0.0);
                    heights.push(settings.webtoon.default_page_height);
                }
                self.current_chapter.heights = heights;
                self.current_chapter.aspect_ratios = aspect_ratios;

                if let Some(nav) = self.nav_stack.last_mut() {
                    match nav {
                        NavigationLevel::Reading { page_index, archive_path, dir_path } => {
                            *page_index = 0;
                            *archive_path = target_path.to_string();
                            *dir_path = dir;
                        }
                        NavigationLevel::ReadingWebtoon { archive_path, dir_path } => {
                            *archive_path = target_path.to_string();
                            *dir_path = dir;
                            self.reader_webtoon_state = None;
                        }
                        _ => {}
                    }
                }

                if !self.current_chapter.cache.contains_key(&0)
                    && !self.current_chapter.loading.contains(&0)
                {
                    let page_name = self.current_chapter.pages[0].name.clone();
                    let tx = self.reader_tx.clone();
                    let ap = target_path.to_string();
                    let arcs = self.current_chapter.archive.clone();
                    let max_decode = settings.reader.max_decode_width;
                    let is_webtoon = matches!(self.nav_stack.last(), Some(NavigationLevel::ReadingWebtoon { .. }));
                    self.current_chapter.loading.insert(0);
                    let _ = crate::reader::worker::decode_tx().send(
                        crate::reader::worker::DecodeJob {
                            ap: ap.clone(),
                            n: 0,
                            page_name: page_name.clone(),
                            max_decode_width: max_decode,
                            arcs,
                            tx,
                            use_max_width: is_webtoon,
                        }
                    );
                }
            }
        }
    }
}
