use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc;

use eframe::egui;

pub struct CoverSystem {
    pub cover_textures: HashMap<u64, egui::TextureHandle>,
    pub cover_loading: HashSet<u64>,
    pub cover_tx: mpsc::Sender<(u64, Vec<u8>, i32, i32)>,
    pub cover_rx: mpsc::Receiver<(u64, Vec<u8>, i32, i32)>,
    pub cover_order: VecDeque<u64>,

    pub chapter_textures: HashMap<u64, egui::TextureHandle>,
    pub chapter_cover_loading: HashSet<u64>,
    pub chapter_cover_tx: mpsc::Sender<(u64, Vec<u8>, i32, i32)>,
    pub chapter_cover_rx: mpsc::Receiver<(u64, Vec<u8>, i32, i32)>,
    pub chapter_cover_order: VecDeque<u64>,

    pub page_textures: HashMap<String, egui::TextureHandle>,
    pub page_cover_loading: HashSet<String>,
    pub page_cover_tx: mpsc::Sender<(String, Vec<u8>, i32, i32)>,
    pub page_cover_rx: mpsc::Receiver<(String, Vec<u8>, i32, i32)>,
    pub page_cover_order: VecDeque<String>,
}

impl CoverSystem {
    pub fn new() -> Self {
        let (cover_tx, cover_rx) = mpsc::channel();
        let (chapter_cover_tx, chapter_cover_rx) = mpsc::channel();
        let (page_cover_tx, page_cover_rx) = mpsc::channel();
        CoverSystem {
            cover_textures: HashMap::new(),
            cover_loading: HashSet::new(),
            cover_tx,
            cover_rx,
            cover_order: VecDeque::new(),
            chapter_textures: HashMap::new(),
            chapter_cover_loading: HashSet::new(),
            chapter_cover_tx,
            chapter_cover_rx,
            chapter_cover_order: VecDeque::new(),
            page_textures: HashMap::new(),
            page_cover_loading: HashSet::new(),
            page_cover_tx,
            page_cover_rx,
            page_cover_order: VecDeque::new(),
        }
    }
}
