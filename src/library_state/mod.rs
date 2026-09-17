use std::collections::HashMap;
use crate::scanner::{ChapterItem, MangaItem};

pub struct LibraryState {
    pub manga_list: Vec<MangaItem>,
    pub chapters: Vec<ChapterItem>,
    pub chapter_idx: HashMap<String, usize>,
    pub current_manga_idx: Option<u64>,
}

impl LibraryState {
    pub fn new() -> Self {
        LibraryState {
            manga_list: Vec::new(),
            chapters: Vec::new(),
            chapter_idx: HashMap::new(),
            current_manga_idx: None,
        }
    }

    pub fn set_chapters(&mut self, chapters: Vec<ChapterItem>) {
        self.chapter_idx = chapters.iter()
            .enumerate()
            .map(|(i, c)| (c.path.clone(), i))
            .collect();
        self.chapters = chapters;
    }

    pub fn clear_chapters(&mut self) {
        self.chapters.clear();
        self.chapter_idx.clear();
    }

    pub fn manga_for_archive(&self, archive_path: &str) -> Option<&MangaItem> {
        self.manga_list
            .iter()
            .find(|m| m.archives.iter().any(|a| a.as_str() == archive_path))
    }

    pub fn is_last_chapter(&self, archive_path: &str) -> bool {
        self.chapter_idx
            .get(archive_path)
            .map_or(false, |&i| i + 1 >= self.chapters.len())
    }
}
