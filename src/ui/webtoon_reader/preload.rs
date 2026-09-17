use std::sync::mpsc;
use crate::reader::ChapterBundle;

pub fn preload_pages(
    bundle: &mut ChapterBundle,
    archive_path: &str,
    reader_tx: &mpsc::Sender<(String, usize, Vec<u8>, i32, i32, bool)>,
    start: usize,
    count: usize,
    max_concurrent: usize,
    max_decode: u32,
) {
    let capacity = max_concurrent.saturating_sub(bundle.loading.len());
    for di in 0..count.min(capacity) {
        let local = start + di;
        if local < bundle.pages.len()
            && !bundle.decode_full.contains(&local)
            && !bundle.loading.contains(&local)
        {
            let name = bundle.pages[local].name.clone();
            bundle.loading.insert(local);
            let _ = crate::reader::worker::decode_tx().send(
                crate::reader::worker::DecodeJob {
                    ap: archive_path.to_string(),
                    n: local,
                    page_name: name.clone(),
                    max_decode_width: max_decode,
                    arcs: bundle.archive.clone(),
                    tx: reader_tx.clone(),
                    use_max_width: true,
                }
            );
        }
    }
}
