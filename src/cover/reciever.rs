use eframe::egui;
use crate::config::settings::ReaderSettings;
use crate::cover::cover_system::CoverSystem;

const MAX_COVERS_PER_FRAME: u32 = 6;

impl CoverSystem {
    pub fn handle_textures(&mut self, ctx: &egui::Context, settings: &ReaderSettings) {
        let mut budget = MAX_COVERS_PER_FRAME;
        while budget > 0 {
            match self.cover_rx.try_recv() {
                Ok((id, rgba, w, h)) => {
                    budget -= 1;
                    if w > 0 && h > 0 {
                        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                        let tex = ctx.load_texture("cover", img, egui::TextureOptions::default());
                        self.cover_textures.insert(id, tex);
                        self.cover_order.push_back(id);
                        if self.cover_textures.len() as u32 > settings.caches.cover_max {
                            while let Some(old) = self.cover_order.pop_front() {
                                if self.cover_textures.remove(&old).is_some() {
                                    break;
                                }
                            }
                        }
                    }
                    self.cover_loading.remove(&id);
                }
                Err(_) => break,
            }
        }
        while budget > 0 {
            match self.chapter_cover_rx.try_recv() {
                Ok((id, rgba, w, h)) => {
                    budget -= 1;
                    if w > 0 && h > 0 {
                        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                        let tex = ctx.load_texture("ch_cover", img, egui::TextureOptions::default());
                        self.chapter_textures.insert(id, tex);
                        self.chapter_cover_order.push_back(id);
                        if self.chapter_textures.len() as u32 > settings.caches.chapter_cover_max {
                            while let Some(old) = self.chapter_cover_order.pop_front() {
                                if self.chapter_textures.remove(&old).is_some() {
                                    break;
                                }
                            }
                        }
                    }
                    self.chapter_cover_loading.remove(&id);
                }
                Err(_) => break,
            }
        }
        while budget > 0 {
            match self.page_cover_rx.try_recv() {
                Ok((key, rgba, w, h)) => {
                    budget -= 1;
                    if w > 0 && h > 0 {
                        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                        let tex = ctx.load_texture("pg_cover", img, egui::TextureOptions::default());
                        self.page_textures.insert(key.clone(), tex);
                        self.page_cover_order.push_back(key.clone());
                        if self.page_textures.len() as u32 > settings.caches.page_cover_max {
                            while let Some(old) = self.page_cover_order.pop_front() {
                                if self.page_textures.remove(&old).is_some() {
                                    break;
                                }
                            }
                        }
                    }
                    self.page_cover_loading.remove(&key);
                }
                Err(_) => break,
            }
        }
    }
}
