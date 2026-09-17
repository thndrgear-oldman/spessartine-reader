use rusqlite::Connection;
use std::path::PathBuf;
use crate::scanner::{MangaItem, ChapterItem, PageItem};

#[derive(Clone)]
pub struct RecentItem {
    pub manga_id: i64,
    pub manga_directory: String,
    pub manga_name: String,
    pub chapter_path: String,
    pub chapter_name: Option<String>,
    pub cover_path: Option<String>,
    pub page_index: usize,
    pub total_pages: usize,
    pub updated_at: String,
    pub is_today: bool,
    pub is_yesterday: bool,
}

pub struct MangaDb {
    conn: Connection,
}

fn i8_to_mode(v: i8) -> Option<crate::reader::mode::ReaderMode> {
    match v {
        1 => Some(crate::reader::mode::ReaderMode::Manga),
        2 => Some(crate::reader::mode::ReaderMode::Double),
        3 => Some(crate::reader::mode::ReaderMode::Webtoon),
        _ => None,
    }
}

fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("spessartine")
        .join("library.db")
}

impl MangaDb {
    pub fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let path = db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        let db = MangaDb { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS manga (
                id INTEGER PRIMARY KEY,
                directory TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS chapters (
                id INTEGER PRIMARY KEY,
                manga_id INTEGER NOT NULL,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS pages (
                chapter_path TEXT NOT NULL,
                name TEXT NOT NULL,
                page_order INTEGER NOT NULL,
                PRIMARY KEY (chapter_path, name)
            );",
        )?;
        let _ = self.conn.execute("ALTER TABLE manga ADD COLUMN manual_mode INTEGER DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE manga ADD COLUMN auto_mode INTEGER DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE manga ADD COLUMN manual_rtl INTEGER DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE manga ADD COLUMN auto_rtl INTEGER DEFAULT 0", []);
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reading_progress (
                    manga_directory TEXT NOT NULL,
                    chapter_path TEXT NOT NULL,
                    page_index INTEGER NOT NULL DEFAULT 0,
                    total_pages INTEGER NOT NULL DEFAULT 0,
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (manga_directory, chapter_path)
            );",
        )?;
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS manga_settings (
                directory TEXT PRIMARY KEY,
                manual_mode INTEGER NOT NULL DEFAULT 0,
                auto_mode INTEGER NOT NULL DEFAULT 0,
                manual_rtl INTEGER NOT NULL DEFAULT 0,
                auto_rtl INTEGER NOT NULL DEFAULT 0
            );",
        )?;
        Ok(())
    }

    pub fn save_manga_settings(&self, directory: &str, manual_mode: Option<crate::reader::mode::ReaderMode>, auto_mode: Option<crate::reader::mode::ReaderMode>, manual_rtl: Option<bool>, auto_rtl: Option<bool>) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO manga_settings (directory, manual_mode, auto_mode, manual_rtl, auto_rtl) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                directory,
                manual_mode.map(|m| match m {
                    crate::reader::mode::ReaderMode::Manga => 1,
                    crate::reader::mode::ReaderMode::Double => 2,
                    crate::reader::mode::ReaderMode::Webtoon => 3,
                }).unwrap_or(0),
                auto_mode.map(|m| match m {
                    crate::reader::mode::ReaderMode::Manga => 1,
                    crate::reader::mode::ReaderMode::Double => 2,
                    crate::reader::mode::ReaderMode::Webtoon => 3,
                }).unwrap_or(0),
                manual_rtl.map(|v| if v { 2 } else { 1 }).unwrap_or(0),
                auto_rtl.map(|v| if v { 2 } else { 1 }).unwrap_or(0),
            ],
        )?;
        Ok(())
    }

    pub fn load_manga_settings(&self) -> Result<Vec<(String, Option<crate::reader::mode::ReaderMode>, Option<crate::reader::mode::ReaderMode>, Option<bool>, Option<bool>)>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT directory, manual_mode, auto_mode, manual_rtl, auto_rtl FROM manga_settings"
        )?;
        let rows = stmt.query_map((), |row| {
            let dir: String = row.get(0)?;
            let man = i8_to_mode(row.get::<_, i8>(1).unwrap_or(0));
            let auto = i8_to_mode(row.get::<_, i8>(2).unwrap_or(0));
            let man_rtl = match row.get::<_, i8>(3).unwrap_or(0) { 0 => None, 1 => Some(false), _ => Some(true) };
            let auto_rtl = match row.get::<_, i8>(4).unwrap_or(0) { 0 => None, 1 => Some(false), _ => Some(true) };
            Ok((dir, man, auto, man_rtl, auto_rtl))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn has_data(&self) -> bool {
        self.conn
            .query_row("SELECT COUNT(*) FROM manga", (), |row| row.get::<_, i64>(0))
            .unwrap_or(0)
            > 0
    }

    pub fn save_manga_list(&self, list: &[MangaItem]) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM manga", ())?;
        for m in list {
            tx.execute(
                "INSERT INTO manga (id, directory, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![m.id as i64, m.directory, m.name],
            )?;
            for archive_path in &m.archives {
                let chapter_id = crate::scanner::generate_id(archive_path);
                let name = std::path::Path::new(archive_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                tx.execute(
                    "INSERT OR IGNORE INTO chapters (id, manga_id, path, name) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![chapter_id as i64, m.id as i64, archive_path, name],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_manga_list(&self) -> Result<Vec<MangaItem>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, directory, name FROM manga")?;
        let rows: Vec<(i64, String, String)> = stmt
            .query_map((), |row| Ok((
                row.get(0)?, row.get(1)?, row.get(2)?,
            )))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);

        let mut manga = Vec::with_capacity(rows.len());
        for (id, directory, name) in rows {
            let mut cs = self
                .conn
                .prepare("SELECT path FROM chapters WHERE manga_id = ?1")?;
            let mut archives: Vec<String> = cs
                .query_map(rusqlite::params![id], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            archives.sort_by(|a, b| crate::services::natcmp(a, b));
            let is_single = archives.len() <= 1;
            manga.push(MangaItem {
                id: id as u64,
                name,
                directory,
                archives,
                is_single_archive: is_single,
                manual_mode: None,
                auto_mode: None,
                manual_rtl: None,
                auto_rtl: None,
            });
        }
        Ok(manga)
    }

    pub fn load_chapters(&self, manga_dir: &str) -> Result<Vec<ChapterItem>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name, c.path FROM chapters c
             JOIN manga m ON c.manga_id = m.id
             WHERE m.directory = ?1",
        )?;
        let mut chapters: Vec<ChapterItem> = stmt
            .query_map(rusqlite::params![manga_dir], |row| {
                Ok(ChapterItem {
                    id: row.get::<_, i64>(0)? as u64,
                    name: row.get(1)?,
                    path: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        chapters.sort_by(|a, b| crate::services::natcmp(&a.name, &b.name));
        Ok(chapters)
    }

    pub fn save_chapters(&self, manga_dir: &str, chapters: &[ChapterItem]) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM chapters WHERE manga_id = (SELECT id FROM manga WHERE directory = ?1)",
            rusqlite::params![manga_dir],
        )?;
        for c in chapters {
            tx.execute(
                "INSERT INTO chapters (id, manga_id, path, name) VALUES (?1, (SELECT id FROM manga WHERE directory = ?2), ?3, ?4)",
                rusqlite::params![c.id as i64, manga_dir, c.path, c.name],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_pages(&self, chapter_path: &str) -> Result<Vec<PageItem>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, page_order FROM pages WHERE chapter_path = ?1 ORDER BY page_order",
        )?;
        let pages: Vec<PageItem> = stmt
            .query_map(rusqlite::params![chapter_path], |row| {
                Ok(PageItem {
                    id: row.get::<_, i64>(1)? as u64,
                    name: row.get(0)?,
                    archive_path: chapter_path.to_string(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(pages)
    }

    pub fn save_pages(&self, chapter_path: &str, pages: &[PageItem]) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM pages WHERE chapter_path = ?1", rusqlite::params![chapter_path])?;
        for p in pages {
            tx.execute(
                "INSERT INTO pages (chapter_path, name, page_order) VALUES (?1, ?2, ?3)",
                rusqlite::params![chapter_path, p.name, p.id as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn save_progress(&self, manga_directory: &str, chapter_path: &str, page_index: usize, total_pages: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT INTO reading_progress (manga_directory, chapter_path, page_index, total_pages)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(manga_directory, chapter_path) DO UPDATE SET
             page_index = ?3, total_pages = ?4, updated_at = datetime('now','localtime')",
            rusqlite::params![manga_directory, chapter_path, page_index as i64, total_pages as i64],
        )?;
        Ok(())
    }

    pub fn load_progress(&self, manga_directory: &str, chapter_path: &str) -> Result<Option<(usize, usize)>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT page_index, total_pages FROM reading_progress WHERE manga_directory = ?1 AND chapter_path = ?2"
        )?;
        let result = stmt.query_row(rusqlite::params![manga_directory, chapter_path], |row| {
            Ok((row.get::<_, i64>(0)? as usize, row.get::<_, i64>(1)? as usize))
        }).ok();
        Ok(result)
    }

    pub fn load_manga_progress(&self, manga_directory: &str) -> Result<f32, Box<dyn std::error::Error>> {
        let total: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(page_index + 1), 0.0) / COALESCE(NULLIF(SUM(total_pages), 0), 1.0)
             FROM reading_progress WHERE manga_directory = ?1",
            rusqlite::params![manga_directory],
            |row| row.get(0),
        )?;
        Ok(total as f32)
    }

    pub fn load_latest_progress(&self, manga_directory: &str) -> Result<Option<(String, usize, usize)>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT chapter_path, page_index, total_pages FROM reading_progress
             WHERE manga_directory = ?1
             ORDER BY updated_at DESC LIMIT 1"
        )?;
        let result = stmt.query_row(rusqlite::params![manga_directory], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as usize,
                row.get::<_, i64>(2)? as usize,
            ))
        }).ok();
        Ok(result)
    }

    pub fn load_recent(&self) -> Result<Vec<RecentItem>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, rp.manga_directory, m.name, rp.chapter_path, c.name,
                    fc.path, rp.page_index, rp.total_pages, rp.updated_at,
                    date(rp.updated_at) = date('now','localtime') AS is_today,
                    date(rp.updated_at) = date('now','localtime','-1 day') AS is_yesterday
             FROM reading_progress rp
             JOIN manga m ON m.directory = rp.manga_directory
             JOIN (
                 SELECT manga_directory, MAX(updated_at) AS max_at
                 FROM reading_progress
                 GROUP BY manga_directory
             ) best ON best.manga_directory = rp.manga_directory AND best.max_at = rp.updated_at
             LEFT JOIN chapters c ON c.path = rp.chapter_path
             LEFT JOIN chapters fc ON fc.id = (SELECT MIN(id) FROM chapters WHERE manga_id = m.id)
             GROUP BY rp.manga_directory
             ORDER BY rp.updated_at DESC",
        )?;
        let rows = stmt.query_map((), |row| {
            Ok(RecentItem {
                manga_id: row.get(0)?,
                manga_directory: row.get(1)?,
                manga_name: row.get(2)?,
                chapter_path: row.get(3)?,
                chapter_name: row.get(4)?,
                cover_path: row.get(5)?,
                page_index: row.get::<_, i64>(6)? as usize,
                total_pages: row.get::<_, i64>(7)? as usize,
                updated_at: row.get(8)?,
                is_today: row.get::<_, i64>(9)? != 0,
                is_yesterday: row.get::<_, i64>(10)? != 0,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
