use std::path::Path;
use zip::ZipArchive;
use std::io::{Read, BufReader};
use std::sync::{Mutex, Arc, LazyLock};
use std::collections::{HashMap, VecDeque};
use crate::services::natcmp;

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
// SAFETY: posix_fadvise is a standard POSIX function with no special safety requirements
// beyond a valid fd. Declared here for direct FFI use without the libc crate dependency.
unsafe extern "C" {
    fn posix_fadvise(fd: std::os::raw::c_int, offset: i64, len: i64, advice: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(target_os = "linux")]
const POSIX_FADV_DONTNEED: std::os::raw::c_int = 4;

#[cfg(target_os = "linux")]
pub struct DropPageCacheFile(std::fs::File);

#[cfg(target_os = "linux")]
impl DropPageCacheFile {
    pub fn new(file: std::fs::File) -> Self { Self(file) }
    pub fn raw_fd(&self) -> std::os::raw::c_int { self.0.as_raw_fd() }
}

#[cfg(target_os = "linux")]
impl std::io::Read for DropPageCacheFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.0.read(buf) }
}

#[cfg(target_os = "linux")]
impl std::io::Seek for DropPageCacheFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> { self.0.seek(pos) }
}

#[cfg(target_os = "linux")]
impl Drop for DropPageCacheFile {
    fn drop(&mut self) {
        // SAFETY: self.0 is an owned File, so as_raw_fd() returns a valid fd.
        // offset=0, len=0 means the entire file; advice is read-only (DONTNEED).
        unsafe { posix_fadvise(self.0.as_raw_fd(), 0, 0, POSIX_FADV_DONTNEED); }
    }
}

pub struct IndexMap {
    pub entries: Vec<(usize, String)>,
    pub lookup: HashMap<String, usize>,
}

static INDEX_CACHE: LazyLock<Mutex<HashMap<String, Arc<IndexMap>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_os = "linux")]
pub type ArchiveReader = std::io::BufReader<DropPageCacheFile>;
#[cfg(not(target_os = "linux"))]
pub type ArchiveReader = std::io::BufReader<std::fs::File>;

pub type SharedArchive = (Arc<IndexMap>, Arc<Mutex<ZipArchive<ArchiveReader>>>);

static ARCHIVE_CACHE: LazyLock<Mutex<HashMap<String, Arc<Mutex<ZipArchive<ArchiveReader>>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_os = "linux")]
static ARCHIVE_FD_CACHE: LazyLock<Mutex<HashMap<String, std::os::raw::c_int>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static ARCHIVE_ORDER: LazyLock<Mutex<VecDeque<String>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

pub fn clear_archive_cache() {
    let mut ac = ARCHIVE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    ac.clear();
    #[cfg(target_os = "linux")]
    ARCHIVE_FD_CACHE.lock().unwrap_or_else(|e| e.into_inner()).clear();
    ARCHIVE_ORDER.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[allow(dead_code)]
pub enum CoverState {
    NotLoaded,
    Loading,
    Loaded(Vec<u8>),
    Failed,
}

pub struct Archive {
    path: std::path::PathBuf,
}

impl Archive {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Archive {
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn get_images(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let (idx, _) = self.get_or_build()?;
        Ok(idx.entries.iter().map(|(_, n)| n.clone()).collect())
    }

    pub fn get_image_data(&self, image_name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (idx, arc) = self.get_or_build()?;
        let zip_idx = idx.lookup.get(image_name)
            .copied()
            .ok_or("Image not found")?;
        let mut archive = arc.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = archive.by_index(zip_idx)?;
        let mut buffer = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn get_first_image(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (idx, arc) = self.get_or_build()?;
        let zip_idx = idx.first_zip_index().ok_or("No images found")?;
        let mut archive = arc.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = archive.by_index(zip_idx)?;
        let mut buffer = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn get_image_data_limited(&self, image_name: &str, limit: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (idx, arc) = self.get_or_build()?;
        let zip_idx = idx.lookup.get(image_name)
            .copied()
            .ok_or("Image not found")?;
        let mut archive = arc.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = archive.by_index(zip_idx)?;
        let mut header = vec![0u8; limit];
        let n = file.read(&mut header)?;
        header.truncate(n);
        Ok(header)
    }

    pub fn get_image_dimensions(&self, image_name: &str) -> Option<(u32, u32)> {
        let header = self.get_image_data_limited(image_name, 8192).ok()?;
        crate::image_processing::decode_dimensions(&header)
    }

    pub fn shared_arcs(&self) -> Result<SharedArchive, Box<dyn std::error::Error>> {
        self.get_or_build()
    }

    fn get_or_build(&self) -> Result<(Arc<IndexMap>, Arc<Mutex<ZipArchive<ArchiveReader>>>), Box<dyn std::error::Error>> {
        let path = self.path.to_string_lossy().to_string();
        {
            let ac = ARCHIVE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(arc) = ac.get(&path) {
                let mut order = ARCHIVE_ORDER.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(pos) = order.iter().position(|p| p == &path) {
                    order.remove(pos);
                    order.push_back(path.clone());
                }
                let ic = INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
                let idx = ic.get(&path).ok_or_else(|| "index not cached for archive")?;
                return Ok((idx.clone(), arc.clone()));
            }
        }
        let file = std::fs::File::open(&self.path)?;
        #[cfg(target_os = "linux")]
        let file = DropPageCacheFile::new(file);
        #[cfg(target_os = "linux")]
        let fd = file.raw_fd();
        let mut archive = ZipArchive::new(BufReader::new(file))?;
        let idx = Arc::new(build_index(&mut archive)?);
        let arc = Arc::new(Mutex::new(archive));
        let mut ac = ARCHIVE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let mut order = ARCHIVE_ORDER.lock().unwrap_or_else(|e| e.into_inner());
        if ac.len() >= 3 {
            if let Some(old) = order.pop_front() {
                ac.remove(&old);
                INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner()).remove(&old);
                #[cfg(target_os = "linux")]
                ARCHIVE_FD_CACHE.lock().unwrap_or_else(|e| e.into_inner()).remove(&old);
            }
        }
        order.push_back(path.clone());
        let mut ic = INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        ic.insert(path.clone(), idx.clone());
        ac.insert(path.clone(), arc.clone());
        #[cfg(target_os = "linux")]
        ARCHIVE_FD_CACHE.lock().unwrap_or_else(|e| e.into_inner()).insert(path, fd);
        Ok((idx, arc))
    }
}

pub fn read_image_direct(
    idx: &IndexMap,
    archive: &Mutex<ZipArchive<ArchiveReader>>,
    image_name: &str,
    archive_path: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let zip_idx = idx
        .lookup
        .get(image_name)
        .copied()
        .ok_or("Image not found")?;
    let mut guard = archive.lock().unwrap_or_else(|e| e.into_inner());
    let mut file = guard.by_index(zip_idx)?;
    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)?;
    drop(file);
    drop(guard);
    #[cfg(target_os = "linux")]
    if let Some(fd) = ARCHIVE_FD_CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(archive_path).copied() {
        // SAFETY: fd comes from ARCHIVE_FD_CACHE which only stores valid fds
        // from opened archives. offset=0, len=0 covers the whole file;
        // DONTNEED is a read-only hint with no safety impact on other threads.
        unsafe { posix_fadvise(fd, 0, 0, POSIX_FADV_DONTNEED); }
    }
    Ok(buffer)
}

fn build_index(archive: &mut ZipArchive<ArchiveReader>) -> Result<IndexMap, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    let mut lookup = HashMap::new();
    let exts = ["jpg", "jpeg", "png", "gif", "webp"];

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.ends_with('/') || name.contains("__MACOSX") {
            continue;
        }
        if let Some(ext) = name.rsplit('.').next() {
            if exts.contains(&ext.to_lowercase().as_str()) {
                lookup.insert(name.clone(), i);
                entries.push((i, name));
            }
        }
    }
    entries.sort_by(|a, b| natcmp(&a.1, &b.1));
    Ok(IndexMap { entries, lookup })
}

impl IndexMap {
    fn first_zip_index(&self) -> Option<usize> {
        self.entries.first().map(|(idx, _)| *idx)
    }
}
