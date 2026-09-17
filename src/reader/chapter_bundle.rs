use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::archive::SharedArchive;
use crate::scanner::PageItem;

pub struct ChapterBundle {
    pub pages: Vec<PageItem>,
    pub archive: Option<SharedArchive>,
    pub archive_path: String,
    pub dir_path: String,
    pub heights: Vec<f32>,
    pub aspect_ratios: Vec<f32>,
    pub cache: HashMap<usize, egui::TextureHandle>,
    pub cache_order: VecDeque<usize>,
    pub cache_sizes: HashMap<usize, usize>,
    pub total_bytes: usize,
    pub loading: HashSet<usize>,
    pub decode_full: HashSet<usize>,
}

impl Default for ChapterBundle {
    fn default() -> Self {
        Self {
            pages: Vec::new(),
            archive: None,
            archive_path: String::new(),
            dir_path: String::new(),
            heights: Vec::new(),
            aspect_ratios: Vec::new(),
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            cache_sizes: HashMap::new(),
            total_bytes: 0,
            loading: HashSet::new(),
            decode_full: HashSet::new(),
        }
    }
}

impl ChapterBundle {
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.archive = None;
        self.archive_path.clear();
        self.dir_path.clear();
        self.heights.clear();
        self.aspect_ratios.clear();
        self.cache_clear();
        self.loading.clear();
        self.decode_full.clear();
    }

    pub fn cache_push(&mut self, idx: usize, tex: egui::TextureHandle, size_bytes: usize, max: usize, budget_bytes: usize, pinned: [usize; 2]) {
        if let Some(&old_sz) = self.cache_sizes.get(&idx) {
            self.total_bytes = self.total_bytes.saturating_sub(old_sz);
        }
        self.cache_sizes.insert(idx, size_bytes);
        self.total_bytes += size_bytes;
        self.cache.insert(idx, tex);
        if let Some(pos) = self.cache_order.iter().position(|&i| i == idx) {
            self.cache_order.remove(pos);
        }
        self.cache_order.push_back(idx);
        while (self.cache_order.len() > max || self.total_bytes > budget_bytes) && !self.cache_order.is_empty() {
            let mut best: Option<(usize, usize)> = None;
            let mut best_dist = 0usize;
            for (pos, &i) in self.cache_order.iter().enumerate() {
                if i == pinned[0] || i == pinned[1] { continue; }
                let d = i.abs_diff(pinned[0]);
                if d > best_dist { best_dist = d; best = Some((pos, i)); }
            }
            let Some((pos, old)) = best else { break };
            self.cache_order.remove(pos);
            if let Some(sz) = self.cache_sizes.remove(&old) {
                self.total_bytes = self.total_bytes.saturating_sub(sz);
            }
            self.cache.remove(&old);
        }
    }

    pub fn cache_clear(&mut self) {
        self.cache.clear();
        self.cache_order.clear();
        self.cache_sizes.clear();
        self.total_bytes = 0;
        self.decode_full.clear();
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VirtualPage {
    Real(usize),
    PanoramaLeft(usize),
    PanoramaRight(usize),
    PanoramaPlaceholder,
    EndPlaceholder(bool),
}

pub fn build_virtual_pages(real_total: usize, aspect_ratios: &[f32], threshold: f32, rtl: bool, is_last_chapter: bool) -> Vec<VirtualPage> {
    let mut v = Vec::new();
    for r in 0..real_total {
        if aspect_ratios.get(r).copied().unwrap_or(0.0) > threshold {
            if v.len() % 2 == 0 {
                if rtl {
                    v.push(VirtualPage::PanoramaRight(r));
                    v.push(VirtualPage::PanoramaLeft(r));
                } else {
                    v.push(VirtualPage::PanoramaLeft(r));
                    v.push(VirtualPage::PanoramaRight(r));
                }
            } else {
                if rtl {
                    v.push(VirtualPage::PanoramaPlaceholder);
                    v.push(VirtualPage::PanoramaRight(r));
                    v.push(VirtualPage::PanoramaLeft(r));
                } else {
                    v.push(VirtualPage::PanoramaPlaceholder);
                    v.push(VirtualPage::PanoramaLeft(r));
                    v.push(VirtualPage::PanoramaRight(r));
                }
            }
        } else {
            v.push(VirtualPage::Real(r));
        }
    }
    v.push(VirtualPage::EndPlaceholder(is_last_chapter));
    if v.len() % 2 == 1 {
        v.push(VirtualPage::EndPlaceholder(is_last_chapter));
    }
    v
}

pub fn compute_virtual_total(real_total: usize, aspect_ratios: &[f32], threshold: f32) -> usize {
    let mut len = 0usize;
    for r in 0..real_total {
        if aspect_ratios.get(r).copied().unwrap_or(0.0) > threshold {
            if len % 2 == 0 { len += 2; }
            else { len += 3; }
        } else {
            len += 1;
        }
    }
    len += 1;
    if len % 2 == 1 { len += 1; }
    len
}
