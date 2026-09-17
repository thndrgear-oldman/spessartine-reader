use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowSettings {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub lag_threshold_ms: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub left_panel_width: f32,
    pub page_margin_x: u32,
    pub page_top_spacing: f32,
    pub library_frame_margin: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridSettings {
    pub cell_height: f32,
    pub cell_base_width: f32,
    pub cell_min_width: f32,
    pub placeholder_font_size: f32,
    pub overlay_height: f32,
    pub overlay_font_size: f32,
    pub max_concurrent_loads: u32,
    pub extra_visible_rows: u32,
    pub preload_rows: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReaderSettingsInner {
    pub spinner_radius: f32,
    pub spinner_speed: f32,
    pub spinner_segments: u32,
    pub spinner_stroke: f32,
    pub status_font_size: f32,
    pub resize_step: i8,
    pub min_edge_margin: f32,
    pub min_content_width: f32,
    pub resize_threshold: f32,
    pub scroll_cooldown_frames: u32,
    pub min_container_width: f32,
    pub double_page_gap: f32,
    pub double_page_shadow_alpha: u32,
    pub double_page_shadow_width: f32,
    pub max_decode_width: u32,
    pub dimension_header_read_size: u32,
    pub double_width_multiplier: f32,
    pub auto_detect_sample_count: u32,
    pub auto_detect_cache_threshold: u32,
    pub panorama_threshold: f32, 
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PreloadSettings {
    pub single_before: u32,
    pub single_after: u32,
    pub reader_ahead: u32,
    pub max_concurrent: u32, 
    pub double_spread_start: i32,
    pub double_spread_end: i32,
    pub page_cache_max: u32,
    pub texture_budget_mb: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebtoonSettings {
    pub default_page_height: f32,
    pub min_content_width: f32,
    pub min_viewport_height: f32,
    pub default_height_fraction: f32,
    pub page_spacing: i8,
    pub rotation_lock_frames: u32,
    pub preload_count: u32,
    pub joined_preload_threshold: u32,
    pub min_width: f32,
    pub resize_step: i8,
    pub resize_threshold: f32,
    pub aspect_ratio_threshold: f32,
    pub auto_detect_majority_factor: u32,
    pub default_ref_width: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollSettings {
    pub pull_resistance: f32,
    pub max_velocity: f32,
    pub wheel_scroll_factor: f32,
    pub keys_scroll_factor: f32,
    pub page_scroll_fraction: f32,
    pub inertia_decay: f32,
    pub inertia_decay_fps: f32,
    pub velocity_repaint_threshold: f32,
    pub pull_overscroll_fraction: f32,
    pub pull_progress_max: f32,
    pub near_threshold: u32,
    pub visible_buffer: u32,
    pub wheel_sensitivity: f32,
    pub dt_min_clamp: f32,
    pub seek_initial_change_threshold: f32,
    pub seek_bias_threshold: f32,
    pub height_change_threshold: f32,
    pub velocity_inertia_threshold: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSettings {
    pub cover_crop_width: u32,
    pub cover_crop_height: u32,
    pub cover_thumbnail_size: u32,
    pub thumbnail_max_dimension: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveSettings {
    pub cache_max_entries: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoverCacheSettings {
    pub cover_max: u32,
    pub chapter_cover_max: u32,
    pub page_cover_max: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioSettings {
    pub sound_count: u32,
    pub muted: bool,
    pub volume: f32,
    // папка звуков (Soundpacks); пустая → по умолчанию <exe_dir>/Soundpacks (15.1)
    #[serde(default)]
    pub soundpack_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolbarSettings {
    pub seeker_reserve: f32,
    pub seeker_min_width: f32,
    pub seeker_alloc_height: f32,
    pub seeker_inner_height: f32,
    pub chapter_popup_min_width: f32,
    pub chapter_popup_max_height: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EffectsSettings {
    pub enabled: bool,
}

impl Default for EffectsSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReaderSettings {
    pub webtoon_width: f32,
    pub single_width: f32,
    pub double_width: f32,
    pub window: WindowSettings,
    pub layout: LayoutSettings,
    pub grid: GridSettings,
    pub reader: ReaderSettingsInner,
    pub preload: PreloadSettings,
    pub webtoon: WebtoonSettings,
    pub scroll: ScrollSettings,
    pub image: ImageSettings,
    pub archive: ArchiveSettings,
    pub caches: CoverCacheSettings,
    pub audio: AudioSettings,
    pub toolbar: ToolbarSettings,
    #[serde(default)]
    pub effects: EffectsSettings,
    #[serde(default)]
    pub lang_code: String,
}

impl Default for PreloadSettings {
    fn default() -> Self {
        Self {
            single_before: 2,
            single_after: 5,
            reader_ahead: 2,
            max_concurrent: 4,
            double_spread_start: -2,
            double_spread_end: 3,
            page_cache_max: 9,
            texture_budget_mb: 150,
        }
    }
}

impl Default for ReaderSettings {
    fn default() -> Self {
        Self {
            webtoon_width: 0.0,
            single_width: 670.0,
            double_width: 0.0,
            window: WindowSettings {
                viewport_width: 600.0,
                viewport_height: 800.0,
                lag_threshold_ms: 100.0,
            },
            layout: LayoutSettings {
                left_panel_width: 200.0,
                page_margin_x: 20,
                page_top_spacing: 40.0,
                library_frame_margin: 8,
            },
            grid: GridSettings {
                cell_height: 250.0,
                cell_base_width: 170.0,
                cell_min_width: 150.0,
                placeholder_font_size: 24.0,
                overlay_height: 30.0,
                overlay_font_size: 14.0,
                max_concurrent_loads: 4,
                extra_visible_rows: 2,
                preload_rows: 2,
            },
            reader: ReaderSettingsInner {
                spinner_radius: 30.0,
                spinner_speed: 0.1,
                spinner_segments: 32,
                spinner_stroke: 4.0,
                status_font_size: 18.0,
                resize_step: 2,
                min_edge_margin: 1.0,
                min_content_width: 600.0,
                resize_threshold: 0.5,
                scroll_cooldown_frames: 5,
                min_container_width: 100.0,
                double_page_gap: 0.0,
                double_page_shadow_alpha: 230,
                double_page_shadow_width: 300.0,
                max_decode_width: 1920,
                dimension_header_read_size: 8192,
                double_width_multiplier: 2.0,
                auto_detect_sample_count: 3,
                auto_detect_cache_threshold: 3,
                panorama_threshold: 1.1,
            },
            preload: PreloadSettings {
                single_before: 2,
                single_after: 5,
                reader_ahead: 2,
                max_concurrent: 4,
                double_spread_start: -2,
                double_spread_end: 3,
                page_cache_max: 9,
                texture_budget_mb: 150,
            },
            webtoon: WebtoonSettings {
                default_page_height: 736.0,
                min_content_width: 100.0,
                min_viewport_height: 500.0,
                default_height_fraction: 0.92,
                page_spacing: 0,
                rotation_lock_frames: 5,
                preload_count: 7,
                joined_preload_threshold: 2,
                min_width: 600.0,
                resize_step: 2,
                resize_threshold: 0.5,
                aspect_ratio_threshold: 1.8,
                auto_detect_majority_factor: 2,
                default_ref_width: 800.0,
            },
            scroll: ScrollSettings {
                pull_resistance: 300.0,
                max_velocity: 2000.0,
                wheel_scroll_factor: 0.08,
                keys_scroll_factor: 6.0,
                page_scroll_fraction: 0.8,
                inertia_decay: 0.92,
                inertia_decay_fps: 60.0,
                velocity_repaint_threshold: 1.0,
                pull_overscroll_fraction: 0.4,
                pull_progress_max: 1.2,
                near_threshold: 2,
                visible_buffer: 3,
                wheel_sensitivity: 1.0,
                dt_min_clamp: 0.1,
                seek_initial_change_threshold: 0.5,
                seek_bias_threshold: 0.5,
                height_change_threshold: 0.5,
                velocity_inertia_threshold: 0.1,
            },
            image: ImageSettings {
                cover_crop_width: 550,
                cover_crop_height: 500,
                cover_thumbnail_size: 550,
                thumbnail_max_dimension: 5000,
            },
            archive: ArchiveSettings {
                cache_max_entries: 5,
            },
            caches: CoverCacheSettings {
                cover_max: 50,
                chapter_cover_max: 50,
                page_cover_max: 50,
            },
            audio: AudioSettings {
                sound_count: 7,
                muted: false,
                volume: 1.0,
                soundpack_dir: String::new(),
            },
            toolbar: ToolbarSettings {
                seeker_reserve: 120.0,
                seeker_min_width: 60.0,
                seeker_alloc_height: 20.0,
                seeker_inner_height: 16.0,
                chapter_popup_min_width: 300.0,
                chapter_popup_max_height: 400.0,
            },
            effects: EffectsSettings { enabled: true },
            lang_code: String::new(),
        }
    }
}

impl ReaderSettings {
    pub fn load() -> Self {
        let path = super::base_dir().join("settings.toml");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = toml::from_str(&content) {
                return settings;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = super::base_dir().join("settings.toml");
        if let Ok(toml_string) = toml::to_string_pretty(self) {
            let _ = fs::write(path, toml_string);
        }
    }
}
