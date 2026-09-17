use std::path::Path;
use std::fs;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::services::natcmp;

#[derive(Clone, Debug)]
pub struct MangaItem {
    pub id: u64,
    pub name: String,
    pub directory: String,
    pub archives: Vec<String>,
    pub is_single_archive: bool,
    pub manual_mode: Option<super::reader::mode::ReaderMode>,
    pub auto_mode: Option<super::reader::mode::ReaderMode>,
    pub manual_rtl: Option<bool>,
    pub auto_rtl: Option<bool>,
}

impl MangaItem {
    pub fn effective_mode(&self) -> super::reader::mode::ReaderMode {
        self.manual_mode
            .or(self.auto_mode)
            .unwrap_or(super::reader::mode::ReaderMode::Manga)
    }

    pub fn effective_rtl(&self) -> bool {
        self.manual_rtl.or(self.auto_rtl).unwrap_or(false)
    }
}

#[derive(Clone, Debug)]
pub struct ChapterItem {
    pub id: u64,
    pub name: String, 
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct PageItem {
    pub id: u64,
    pub name: String,
    pub archive_path: String,
}

pub fn generate_id(path: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn get_manga_name(path: &Path, root_scan_dir: &Path) -> String {
    if path.parent() == Some(root_scan_dir) {
        return path.file_stem().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string();
    }
    path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Unknown").to_string()
}

pub fn scan_directory(directory: impl AsRef<Path>, root_scan_dir: &Path) -> Result<Vec<MangaItem>, Box<dyn std::error::Error>> {
    let dir = directory.as_ref();
    let mut manga_groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut manga_items = Vec::new();
    let archive_exts = ["zip", "cbz", "cbr", "rar"];

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Ok(mut sub_manga) = scan_directory(&path, root_scan_dir) {
                manga_items.append(&mut sub_manga);
            }
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    if archive_exts.contains(&ext_str.to_lowercase().as_str()) {
                        let dir_key = path.parent().unwrap_or(root_scan_dir).to_string_lossy().to_string();
                        let path_str = path.to_string_lossy().to_string();
                        
                        manga_groups.entry(dir_key).or_insert_with(Vec::new).push(path_str);
                    }
                }
            }
        }
    }

    for (dir_key, archives) in manga_groups {
        let name = if Path::new(&dir_key) == root_scan_dir {
            Path::new(&archives[0])
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string()
        } else {
            get_manga_name(Path::new(&dir_key), root_scan_dir)
        };
        
        let id = generate_id(&dir_key);
        let is_single_archive = Path::new(&dir_key) == root_scan_dir;
        
        let manga_item = MangaItem {
            id,
            name,
            directory: dir_key,
            archives,
            is_single_archive,
            manual_mode: None,
            auto_mode: None,
            manual_rtl: None,
            auto_rtl: None,
        };
        manga_items.push(manga_item);
    }
    manga_items.sort_by(|a, b| natcmp(&a.name, &b.name));
    Ok(manga_items)
}

pub fn scan_all_libraries(directories: Vec<crate::config::storage::DirectoryItem>) -> Result<Vec<MangaItem>, Box<dyn std::error::Error>> {
    let mut all_manga = Vec::new();
    for dir_item in directories {
        let dir_path = Path::new(&dir_item.path);
        if let Ok(mut manga) = scan_directory(dir_path, dir_path) {
            all_manga.append(&mut manga);
        }
    }
    
    all_manga.sort_by(|a, b| natcmp(&a.name, &b.name));
    
    Ok(all_manga)
}

pub fn get_chapters(dir_path: &str) -> Result<Vec<ChapterItem>, Box<dyn std::error::Error>> {
    let entries = std::fs::read_dir(dir_path)?;
    let mut chapters = Vec::new();
    let archive_exts = ["zip", "cbz", "cbr", "rar"];

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if archive_exts.contains(&ext.to_lowercase().as_str()) {
                let id = generate_id(&path.to_string_lossy());
                chapters.push(ChapterItem {
                    id,
                    name: path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }
    chapters.sort_by(|a, b| natcmp(&a.name, &b.name));
    Ok(chapters)
}

pub fn get_pages(archive_path: &str) -> Result<Vec<PageItem>, Box<dyn std::error::Error>> {
    let archive = crate::archive::Archive::new(archive_path)?;
    let images = archive.get_images()?;
    
    Ok(images
        .into_iter()
        .enumerate()
        .map(|(idx, name)| PageItem {
            id: idx as u64,
            name,
            archive_path: archive_path.to_string(),
        })
        .collect())
}

