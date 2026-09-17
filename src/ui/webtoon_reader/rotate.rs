use crate::config::settings::ReaderSettings;
use crate::reader::state::ReaderState;

impl ReaderState {
    pub fn rotate_up(&mut self, state: &mut crate::ui::webtoon_reader::state::WebtoonState, settings: &ReaderSettings) {
        let spacing = settings.webtoon.page_spacing;
        let old_prev_h = self.prev_chapter.heights.iter().sum::<f32>()
            + self.prev_chapter.pages.len() as f32 * spacing as f32;
        state.frame_offset = (state.frame_offset - old_prev_h).max(0.0);

        self.joined_chapter = std::mem::take(&mut self.current_chapter);
        self.current_chapter = std::mem::take(&mut self.prev_chapter);
    }

    pub fn rotate_down(&mut self, state: &mut crate::ui::webtoon_reader::state::WebtoonState, settings: &ReaderSettings) {
        let spacing = settings.webtoon.page_spacing;
        let old_joined_h = self.joined_chapter.heights.iter().sum::<f32>()
            + self.joined_chapter.pages.len() as f32 * spacing as f32;
        state.frame_offset += old_joined_h;

        self.prev_chapter = std::mem::take(&mut self.current_chapter);
        self.current_chapter = std::mem::take(&mut self.joined_chapter);
    }
}
