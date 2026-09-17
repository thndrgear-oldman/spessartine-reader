use std::path::Path;
use std::fs;

pub mod reciever;
pub mod cover_system;

pub fn get_title_cover(archive_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let dir = Path::new(archive_path)
        .parent()
        .ok_or("No parent directory")?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    if file_name_str.to_lowercase().starts_with("cover.") {
                        return fs::read(path).map_err(|e| e.into());
                    }
                }
            }
        }
    }
    Err("Cover not found".into())
}

pub fn get_cover_data(manga_path: &str) -> Option<(Vec<u8>, bool)> {
    if let Ok(data) = get_title_cover(manga_path) {
        return Some((data, false));
    }
    let archive = crate::archive::Archive::new(manga_path).ok()?;
    let data = archive.get_first_image().ok()?;
    Some((data, true))
}
