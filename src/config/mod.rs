use serde::Deserialize;
use eframe::egui;
use crate::theme::AppTheme;
use std::path::PathBuf;

pub mod storage;
pub mod settings;

#[derive(Deserialize)]
struct ThemeName {
    theme: String,
}

pub fn base_dir() -> PathBuf {
    if let Some(cfg) = dirs::config_dir() {
        let path = cfg.join(".spessartine");
        if path.join("config.toml").exists() {
            return path;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let path = parent.join(".spessartine");
            if path.join("config.toml").exists() {
                return path;
            }
        }
    }
    PathBuf::from(".spessartine")
}

#[derive(Deserialize)]
pub struct ThemeConfig {
    pub colors: ThemeColors,
    pub radii: ThemeRadii,
    pub layout: ThemeLayout,
    pub sizes: ThemeSizes,
}

#[derive(Deserialize)]
pub struct ThemeColors {
    pub window_bg: String,
    pub window_fg: String,
    pub left_panel_bg: String,
    pub cell_bg: String,
    pub cell_fg: String,
    pub label_overlay_bg: String,
    pub label_overlay_fg: String,
    pub toolbar_bg: String,
    pub toolbar_btn_bg: String,
    pub toolbar_btn_fg: String,
    pub toolbar_btn_press_bg: String,
    pub toolbar_btn_hover_bg: String,
    pub addres_bar_bg: String,
    pub addres_bar_fg: String,
    pub reader_bg: String,
    pub btn_press_bg: String,
    pub btn_hover_bg: String,
    pub fav_btn_bg: String,
    pub fav_btn_fg: String,
    pub library_bg: String,
    pub grid_bg: String,
    pub lp_btn_bg: String,
    pub lp_btn_hover_bg: String,
    pub lp_btn_active_bg: String,
    pub lp_btn_fg: String,
    pub placeholder_bg: String,
    pub scroll_bg: String,
}

#[derive(Deserialize)]
pub struct ThemeRadii {
    pub cell_radius: u8,
    pub cover_radius: f32,
    pub label_overlay_radius: f32,
    pub icon_button_radius: f32,
    pub text_button_radius: f32,
    pub left_panel_button_radius: f32,
    pub settings_tag_radius: f32,
    pub chapter_popup_radius: f32,
    pub address_bar_radius: f32,
}

#[derive(Deserialize)]
pub struct ThemeLayout {
    pub row_gap: u8,
}

#[derive(Deserialize)]
pub struct ThemeSizes {
    pub toolbar_margin_y: u32,
    pub icon_button_size: f32,
    pub icon_button_font_size: f32,
    pub text_button_font_size: f32,
    pub text_button_padding_x: f32,
    pub text_button_text_offset: f32,
    pub text_button_height: f32,
    pub left_panel_button_height: f32,
    pub left_panel_button_margin: f32,
    pub left_panel_text_offset: f32,
    pub left_panel_font_size: f32,
    pub left_panel_button_spacing: f32,
    pub left_panel_top_padding: f32,
    pub settings_tag_padding_x: f32,
    pub settings_tag_height: f32,
    pub settings_tag_text_offset: f32,
    pub settings_tag_spacing: f32,
    pub seeker_tick_height: f32,
    pub seeker_tick_gap: f32,
    pub seeker_sep_stroke: f32,
    pub seeker_tick_stroke: f32,
    pub seeker_indicator_top: f32,
    pub seeker_indicator_height: f32,
    pub seeker_indicator_half_width: f32,
    pub chapter_popup_margin: u32,
    pub address_bar_margin_x: u32,
    pub address_bar_margin_y: u32,
    pub address_bar_font_size: f32,
    pub mode_badge_size: f32,
    pub mode_badge_offset: f32,
    pub mode_badge_font_size: f32,
    pub seeker_alloc_height: f32,
    pub seeker_inner_height: f32,
}

fn hex(s: &str) -> egui::Color32 {
    egui::Color32::from_hex(s).unwrap_or(egui::Color32::GRAY)
}

impl ThemeConfig {
    pub fn to_app_theme(&self) -> AppTheme {
        AppTheme {
            window_bg: hex(&self.colors.window_bg),
            window_fg: hex(&self.colors.window_fg),
            left_panel_bg: hex(&self.colors.left_panel_bg),
            cell_bg: hex(&self.colors.cell_bg),
            cell_fg: hex(&self.colors.cell_fg),
            label_overlay_bg: hex(&self.colors.label_overlay_bg),
            label_overlay_fg: hex(&self.colors.label_overlay_fg),
            toolbar_bg: hex(&self.colors.toolbar_bg),
            toolbar_btn_bg: hex(&self.colors.toolbar_btn_bg),
            toolbar_btn_fg: hex(&self.colors.toolbar_btn_fg),
            toolbar_btn_press_bg: hex(&self.colors.toolbar_btn_press_bg),
            toolbar_btn_hover_bg: hex(&self.colors.toolbar_btn_hover_bg),
            addres_bar_bg: hex(&self.colors.addres_bar_bg),
            addres_bar_fg: hex(&self.colors.addres_bar_fg),
            btn_hover_bg: hex(&self.colors.btn_hover_bg),
            btn_press_bg: hex(&self.colors.btn_press_bg),
            reader_bg: hex(&self.colors.reader_bg),
            fav_btn_bg: hex(&self.colors.fav_btn_bg),
            fav_btn_fg: hex(&self.colors.fav_btn_fg),
            library_bg: hex(&self.colors.library_bg),
            grid_bg: hex(&self.colors.grid_bg),
            lp_btn_bg: hex(&self.colors.lp_btn_bg),
            lp_btn_hover_bg: hex(&self.colors.lp_btn_hover_bg),
            lp_btn_active_bg: hex(&self.colors.lp_btn_active_bg),
            lp_btn_fg: hex(&self.colors.lp_btn_fg),
            placeholder_bg: hex(&self.colors.placeholder_bg),
            scroll_bg: hex(&self.colors.scroll_bg),
            cell_radius: self.radii.cell_radius,
            cover_radius: self.radii.cover_radius,
            label_overlay_radius: self.radii.label_overlay_radius,
            icon_button_radius: self.radii.icon_button_radius,
            text_button_radius: self.radii.text_button_radius,
            left_panel_button_radius: self.radii.left_panel_button_radius,
            settings_tag_radius: self.radii.settings_tag_radius,
            chapter_popup_radius: self.radii.chapter_popup_radius,
            address_bar_radius: self.radii.address_bar_radius,
            toolbar_margin_y: self.sizes.toolbar_margin_y,
            icon_button_size: self.sizes.icon_button_size,
            icon_button_font_size: self.sizes.icon_button_font_size,
            text_button_font_size: self.sizes.text_button_font_size,
            text_button_padding_x: self.sizes.text_button_padding_x,
            text_button_text_offset: self.sizes.text_button_text_offset,
            text_button_height: self.sizes.text_button_height,
            left_panel_button_height: self.sizes.left_panel_button_height,
            left_panel_button_margin: self.sizes.left_panel_button_margin,
            left_panel_text_offset: self.sizes.left_panel_text_offset,
            left_panel_font_size: self.sizes.left_panel_font_size,
            left_panel_button_spacing: self.sizes.left_panel_button_spacing,
            left_panel_top_padding: self.sizes.left_panel_top_padding,
            settings_tag_padding_x: self.sizes.settings_tag_padding_x,
            settings_tag_height: self.sizes.settings_tag_height,
            settings_tag_text_offset: self.sizes.settings_tag_text_offset,
            settings_tag_spacing: self.sizes.settings_tag_spacing,
            seeker_tick_height: self.sizes.seeker_tick_height,
            seeker_tick_gap: self.sizes.seeker_tick_gap,
            seeker_sep_stroke: self.sizes.seeker_sep_stroke,
            seeker_tick_stroke: self.sizes.seeker_tick_stroke,
            seeker_indicator_top: self.sizes.seeker_indicator_top,
            seeker_indicator_height: self.sizes.seeker_indicator_height,
            seeker_indicator_half_width: self.sizes.seeker_indicator_half_width,
            chapter_popup_margin: self.sizes.chapter_popup_margin,
            address_bar_margin_x: self.sizes.address_bar_margin_x,
            address_bar_margin_y: self.sizes.address_bar_margin_y,
            address_bar_font_size: self.sizes.address_bar_font_size,
            mode_badge_size: self.sizes.mode_badge_size,
            mode_badge_offset: self.sizes.mode_badge_offset,
            mode_badge_font_size: self.sizes.mode_badge_font_size,
            seeker_alloc_height: self.sizes.seeker_alloc_height,
            seeker_inner_height: self.sizes.seeker_inner_height,
            row_gap: self.layout.row_gap,
        }
    }
}

pub fn load_theme() -> AppTheme {
    let theme_name: ThemeName = match std::fs::read_to_string(base_dir().join("config.toml")) {
        Ok(content) => toml::from_str(&content).unwrap_or(ThemeName { theme: "Dark".into() }),
        Err(_) => ThemeName { theme: "Dark".into() },
    };
    match theme_name.theme.as_str() {
        "Flexoki" => AppTheme::flexoki(),
        _ => {
            let path = base_dir().join("themes").join(format!("{}.toml", theme_name.theme));
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<ThemeConfig>(&content) {
                    Ok(cfg) => cfg.to_app_theme(),
                    Err(_) => AppTheme::dark(),
                },
                Err(_) => AppTheme::dark(),
            }
        }
    }
}

pub fn save_theme(name: &str) {
    let dir = base_dir();
    std::fs::create_dir_all(dir.join("themes")).ok();
    let content = format!("theme = \"{}\"\n", name);
    std::fs::write(dir.join("config.toml"), content).ok();
}
