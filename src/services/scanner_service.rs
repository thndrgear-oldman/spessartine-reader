use std::sync::mpsc;
use std::thread;
use crate::scanner::{scan_all_libraries, MangaItem};
use crate::config::storage::DirectoryItem;

pub enum ScanEvent {
    Completed(Vec<MangaItem>),
    Failed(String),
}

pub fn start_scan(tx: mpsc::Sender<ScanEvent>, directories: Vec<DirectoryItem>) {
    thread::spawn(move || {
        match scan_all_libraries(directories) {
            Ok(list) => {
                let _ = tx.send(ScanEvent::Completed(list));
            }
            Err(e) => {
                let _ = tx.send(ScanEvent::Failed(e.to_string()));
            }
        }
    });
}
