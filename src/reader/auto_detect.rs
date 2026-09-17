use crate::reader::mode::ReaderMode;

pub fn detect_from_pages(dims: &[(u32, u32)], aspect_threshold: f32, majority_factor: u32) -> ReaderMode {
    if dims.is_empty() { return ReaderMode::Manga; }
    let webtoon_votes = dims.iter().filter(|&&(w, h)| {
        w > 0 && (h as f32 / w as f32) > aspect_threshold
    }).count();
    if webtoon_votes * majority_factor as usize >= dims.len() {
        ReaderMode::Webtoon
    } else {
        ReaderMode::Manga
    }
}
