use image::{DynamicImage, GenericImageView, imageops::FilterType};
use std::io::Cursor;

fn decode_webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 20 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return None;
    }
    let tag = &data[12..16];
    let extra = &data[20..];
    match tag {
        b"VP8 " => {
            if extra.len() < 10 || extra[3..6] != [0x9D, 0x01, 0x2A] { return None; }
            let w = (u32::from(extra[7]) << 8 | u32::from(extra[6])) & 0x3FFF;
            let h = (u32::from(extra[9]) << 8 | u32::from(extra[8])) & 0x3FFF;
            Some((w, h))
        }
        b"VP8L" => {
            if extra.len() < 5 || extra[0] != 0x2F { return None; }
            let raw = u32::from_le_bytes([extra[1], extra[2], extra[3], extra[4]]);
            Some(((raw & 0x3FFF) + 1, ((raw >> 14) & 0x3FFF) + 1))
        }
        b"VP8X" => {
            if extra.len() < 10 { return None; }
            let w = u32::from_le_bytes([extra[4], extra[5], extra[6], 0]) + 1;
            let h = u32::from_le_bytes([extra[7], extra[8], extra[9], 0]) + 1;
            Some((w & 0xFFFFFF, h & 0xFFFFFF))
        }
        _ => None,
    }
}

pub fn decode_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return decode_webp_dimensions(data);
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(data));
    let reader = reader.with_guessed_format().ok()?;
    reader.into_dimensions().ok()
}

fn apply_exif_orientation(img: DynamicImage, data: &[u8]) -> DynamicImage {
    if !data.starts_with(b"\xFF\xD8") { return img; }
    let reader = exif::Reader::new();
    if let Ok(exif) = reader.read_from_container(&mut Cursor::new(data)) {
        if let Some(field) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
            let orientation = field.value.get_uint(0).unwrap_or(1);
            return match orientation {
                2 => img.fliph(),
                3 => img.rotate180(),
                4 => img.flipv(),
                5 => img.fliph().rotate270(),
                6 => img.rotate90(),
                7 => img.fliph().rotate90(),
                8 => img.rotate270(),
                _ => img,
            };
        }
    }
    img
}

fn try_decode(data: &[u8]) -> Option<DynamicImage> {
    match image::load_from_memory(data) {
        Ok(img) => Some(img),
        Err(_) => {
            let fmt = if data.starts_with(b"\xFF\xD8") { image::ImageFormat::Jpeg }

            else if data.starts_with(b"\x89PNG") { image::ImageFormat::Png }
            else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" { image::ImageFormat::WebP }
            else { return None; };
            image::load_from_memory_with_format(data, fmt).ok()
        }
    }
}

fn resize_to_thumbnail(img: DynamicImage, max_size: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let scale = (max_size as f32 / w as f32).min(max_size as f32 / h as f32);
    let nw = (w as f32 * scale) as u32;
    let nh = (h as f32 * scale) as u32;
    img.resize_exact(nw.max(1), nh.max(1), FilterType::Nearest)
}

fn to_rgba(img: DynamicImage) -> (Vec<u8>, i32, i32) {
    let rgba = img.to_rgba8();
    let width = rgba.width() as i32;
    let height = rgba.height() as i32;
    (rgba.into_vec(), width, height)
}

pub struct ImageProcessor;

impl ImageProcessor {
    pub fn decode_raw(data: &[u8], max_side: u32) -> Option<(Vec<u8>, i32, i32)> {
        let img = try_decode(data)?;
        let img = apply_exif_orientation(img, data);
        let (w, h) = img.dimensions();
        if w > max_side || h > max_side {
            let scale = (max_side as f32 / w as f32).min(max_side as f32 / h as f32);
            let nw = (w as f32 * scale) as u32;
            let nh = (h as f32 * scale) as u32;
            let thumb = img.resize_exact(nw.max(1), nh.max(1), FilterType::Nearest);
            Some(to_rgba(thumb))
        } else {
            Some(to_rgba(img))
        }
    }

    pub fn decode_raw_max_dimensions(
        data: &[u8],
        max_width: u32,
        max_height: u32,
    ) -> Option<(Vec<u8>, i32, i32)> {
        let img = try_decode(data)?;
        let img = apply_exif_orientation(img, data);
        let (w, h) = img.dimensions();
        let scale = (max_width as f32 / w as f32)
            .min(max_height as f32 / h as f32)
            .min(1.0);
        if scale < 1.0 {
            let nw = (w as f32 * scale) as u32;
            let nh = (h as f32 * scale) as u32;
            let thumb = img.resize_exact(nw.max(1), nh.max(1), FilterType::Triangle);
            Some(to_rgba(thumb))
        } else {
            Some(to_rgba(img))
        }
    }

    pub fn decode_and_process(cover_data: &[u8], from_archive: bool) -> Option<(Vec<u8>, i32, i32)> {
        match try_decode(cover_data) {
            Some(img) => {
                let img = apply_exif_orientation(img, cover_data);
                let processed = if from_archive {
                    Self::crop_to_aspect_ratio(img, 550, 500)
                } else {
                    img
                };
                let thumb = resize_to_thumbnail(processed, 550);
                Some(to_rgba(thumb))
            }
            _none => None,
        }
    }

    pub fn decode_and_process_from_top(cover_data: &[u8], from_archive: bool) -> Option<(Vec<u8>, i32, i32)> {
        match try_decode(cover_data) {
            Some(img) => {
                let img = apply_exif_orientation(img, cover_data);
                let processed = if from_archive {
                    Self::crop_from_top(img, 550, 500)
                } else {
                    img
                };
                let thumb = resize_to_thumbnail(processed, 550);
                Some(to_rgba(thumb))
            }
            _none => None,
        }
    }

    fn crop_to_aspect_ratio(
        img: DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> DynamicImage {
        let (current_width, current_height) = img.dimensions();
        let target_ratio = target_width as f32 / target_height as f32;
        let current_ratio = current_width as f32 / current_height as f32;

        let (crop_width, crop_height) = if current_ratio > target_ratio {
            let new_width = (current_height as f32 * target_ratio) as u32;
            (new_width, current_height)
        } else {
            let new_height = (current_width as f32 / target_ratio) as u32;
            (current_width, new_height)
        };

        let x = (current_width - crop_width) / 2;
        let y = (current_height - crop_height) / 2;

        img.crop_imm(x, y, crop_width, crop_height)
    }

    pub fn crop_from_top(
        img: DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> DynamicImage {
        let (current_width, current_height) = img.dimensions();
        let target_ratio = target_width as f32 / target_height as f32;
        let current_ratio = current_width as f32 / current_height as f32;

        let (crop_width, crop_height) = if current_ratio > target_ratio {
            let new_width = (current_height as f32 * target_ratio) as u32;
            (new_width, current_height)
        } else {
            let new_height = (current_width as f32 / target_ratio) as u32;
            (current_width, new_height)
        };

        let x = (current_width - crop_width) / 2;
        img.crop_imm(x, 0, crop_width, crop_height)
    }
}
