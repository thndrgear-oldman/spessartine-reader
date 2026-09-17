use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

static CACHE_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn cache_dir() -> PathBuf {
    let mut guard = CACHE_DIR.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref dir) = *guard {
        return dir.clone();
    }
    let cfg = crate::config::storage::DirectoriesConfig::load();
    let dir = cfg.get_cache_dir();
    let pb = PathBuf::from(&dir);
    std::fs::create_dir_all(&pb).ok();
    *guard = Some(pb.clone());
    pb
}

fn path_hash(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn cache_path(key: &str) -> PathBuf {
    cache_dir().join(format!("{}.thumb", path_hash(key)))
}

pub fn get_thumbnail(key: &str) -> Option<(Vec<u8>, i32, i32)> {
    let cache = cache_path(key);
    if cache.exists() {
        let raw = std::fs::read(&cache).ok()?;
        if raw.len() < 8 { return None; }
        let width = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
        let height = i32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
        if width <= 0 || height <= 0 || width > 5000 || height > 5000 { return None; }
        Some((raw[8..].to_vec(), width, height))
    } else {
        None
    }
}

pub fn store_thumbnail(key: &str, data: &[u8], width: i32, height: i32) {
    let cache = cache_path(key);
    let mut raw = Vec::with_capacity(8 + data.len());
    raw.extend_from_slice(&width.to_le_bytes());
    raw.extend_from_slice(&height.to_le_bytes());
    raw.extend_from_slice(data);
    std::fs::write(&cache, raw).ok();
}

pub fn set_cache_dir(path: &str) {
    let pb = PathBuf::from(path);
    std::fs::create_dir_all(&pb).ok();
    *CACHE_DIR.lock().unwrap_or_else(|e| e.into_inner()) = Some(pb);
}
