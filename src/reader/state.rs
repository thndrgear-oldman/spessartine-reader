use std::sync::mpsc;

use crate::reader::chapter_bundle::ChapterBundle;
use crate::reader::chapter_switch::NextChapterState;
use crate::ui::reader::seeker::SeekerAnim;
use crate::ui::webtoon_reader::state::WebtoonState;
use crate::NavigationLevel;

pub struct ReaderState {
    pub nav_stack: Vec<NavigationLevel>,
    pub reader_tx: mpsc::Sender<(String, usize, Vec<u8>, i32, i32, bool)>,
    pub reader_rx: mpsc::Receiver<(String, usize, Vec<u8>, i32, i32, bool)>,
    pub next_switch: NextChapterState,
    pub prev_switch: NextChapterState,
    pub reader_webtoon_state: Option<WebtoonState>,
    pub prev_chapter: ChapterBundle,
    pub current_chapter: ChapterBundle,
    pub joined_chapter: ChapterBundle,
    pub reader_seeker: SeekerAnim,
    pub reader_page: usize,
    pub reader_total: usize,
    pub scroll_cooldown: u8,
    #[cfg(feature = "fx")]
    pub fx_player: crate::fx::FxPlayer,
}

impl ReaderState {
    pub fn new() -> Self {
        let (reader_tx, reader_rx) = mpsc::channel();
        ReaderState {
            nav_stack: vec![NavigationLevel::Titles],
            reader_tx,
            reader_rx,
            next_switch: NextChapterState::new(),
            prev_switch: NextChapterState::new(),
            reader_webtoon_state: None,
            prev_chapter: ChapterBundle::default(),
            current_chapter: ChapterBundle::default(),
            joined_chapter: ChapterBundle::default(),
            reader_seeker: SeekerAnim::new(),
            reader_page: 0,
            reader_total: 0,
            scroll_cooldown: 0,
            #[cfg(feature = "fx")]
            fx_player: crate::fx::FxPlayer::new(),
        }
    }

    #[cfg(feature = "fx")]
    pub fn fx_load_chapter(&mut self, lib: &crate::library_state::LibraryState, chapter_path: &str) {
        let chapter_num = lib
            .chapters
            .iter()
            .position(|c| c.path == *chapter_path)
            .map(|i| i as u32 + 1)
            .unwrap_or(1);
        let _ = self.fx_player.open(chapter_path, chapter_num);
    }

    #[cfg(feature = "fx")]
    pub fn toggle_fx(
        &mut self,
        lib: &crate::library_state::LibraryState,
        settings: &crate::config::settings::ReaderSettings,
    ) {
        if let Some(NavigationLevel::Reading { page_index, archive_path, .. }) = self.nav_stack.last() {
            if !self.current_chapter.pages.is_empty() {
                let manga = lib.manga_for_archive(archive_path);
                let mode = manga
                    .map(|m| m.effective_mode())
                    .unwrap_or(crate::reader::mode::ReaderMode::Manga);
                let rtl = manga.map(|m| m.effective_rtl()).unwrap_or(false);
                crate::ui::reader::fx_hooks::update_manga_spread(
                    &mut self.fx_player,
                    *page_index,
                    self.current_chapter.pages.len(),
                    &self.current_chapter.aspect_ratios,
                    settings.reader.panorama_threshold,
                    rtl,
                    lib.is_last_chapter(archive_path),
                    mode == crate::reader::mode::ReaderMode::Double,
                );
            }
        }
        let on = !self.fx_player.interactive();
        let sp = self.fx_player.cur_spread();
        self.fx_player.set_interactive(on, sp);
    }
}
