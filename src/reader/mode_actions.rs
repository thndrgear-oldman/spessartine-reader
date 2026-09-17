use crate::reader::mode::ReaderMode;
use crate::database::MangaDb;
use crate::library_state::LibraryState;
use crate::reader::state::ReaderState;
use crate::SpessartineApp;

impl ReaderState {
    pub fn set_manga_mode(&mut self, lib: &mut LibraryState, db: &mut Option<MangaDb>, id: u64, mode: Option<ReaderMode>) -> bool {
        let Some(m) = lib.manga_list.iter_mut().find(|m| m.id == id) else { return false; };
        m.manual_mode = mode;
        if let Some(ref db) = *db {
            let _ = db.save_manga_settings(&m.directory, m.manual_mode, m.auto_mode, m.manual_rtl, m.auto_rtl);
        }
        true
    }

    pub fn set_manga_mode_with_switch(&mut self, lib: &mut LibraryState, db: &mut Option<MangaDb>, id: u64, mode: Option<ReaderMode>, pending: &mut Option<(u64, Option<ReaderMode>)>) -> bool {
        if !self.set_manga_mode(lib, db, id, mode) { return false; }
        *pending = Some((id, mode));
        true
    }

    pub fn set_manga_rtl(&mut self, lib: &mut LibraryState, db: &mut Option<MangaDb>, id: u64, rtl: bool) -> bool {
        let Some(m) = lib.manga_list.iter_mut().find(|m| m.id == id) else { return false; };
        m.manual_rtl = Some(rtl);
        if let Some(ref db) = *db {
            let _ = db.save_manga_settings(&m.directory, m.manual_mode, m.auto_mode, m.manual_rtl, m.auto_rtl);
        }
        true
    }
}

impl SpessartineApp {
    pub fn manga_by_archive(&self, ap: &str) -> Option<&crate::scanner::MangaItem> {
        self.lib.manga_list.iter().find(|m| m.archives.iter().any(|a| a == ap))
    }
}
