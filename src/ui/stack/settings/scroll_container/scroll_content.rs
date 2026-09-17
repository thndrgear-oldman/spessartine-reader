use eframe::egui;
use crate::cache;
use crate::config::storage::{DirectoriesConfig, default_cache_dir};
use crate::ui::button;
use crate::ui::file_picker;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    ui.heading(egui::RichText::new(app.lang.heading_settings).size(36.0));
    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_dirs).size(20.0));
    ui.add_space(5.0);

    let mut config = DirectoriesConfig::load();
    let fw = ui.available_width() - 20.0;

    egui::Frame::default()
        .fill(app.theme.placeholder_bg)
        .corner_radius(egui::CornerRadius::same(app.theme.cell_radius))
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.set_min_width(fw);

            let mut remove_idx = None;

            ui.horizontal_wrapped(|ui| {
                for (i, dir) in config.get_directories().iter().enumerate() {
                    let text = format!("{} 󰖭", dir.path);
                    let resp = button::tag_btn(ui, &text, app.theme.cell_fg, &app.theme, 14.0);
                    if resp.clicked() {
                        remove_idx = Some(i);
                    }
                    ui.add_space(4.0);
                }
            });

            if let Some(i) = remove_idx {
                config.remove_directory(i);
                config.save().ok();
            }

            ui.add_space(6.0);
        });

    ui.add_space(4.0);
    if let Some(path) = file_picker::pick_folder(
        ui,
        egui::Id::new("add_library_dir"),
        &app.theme,
        &app.lang,
        app.lang.btn_add_dir,
    ) {
        config.add_directory(path);
        config.save().ok();
    }

    ui.add_space(16.0);
    ui.label(egui::RichText::new(app.lang.sec_cache).size(20.0));
    ui.add_space(5.0);

    let fw = ui.available_width() - 20.0;

    egui::Frame::default()
        .fill(app.theme.placeholder_bg)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.set_min_width(fw);

            let text = format!("{} 󰖭", config.get_cache_dir());
            let resp = button::tag_btn(ui, &text, app.theme.cell_fg, &app.theme, 16.0);
            if resp.clicked() {
                let default = default_cache_dir();
                config.set_cache_dir(default);
                config.save().ok();
                cache::set_cache_dir(&config.get_cache_dir());
            }

            ui.add_space(6.0);
        });

    ui.add_space(4.0);
    if let Some(path) = file_picker::pick_folder(
        ui,
        egui::Id::new("change_cache_dir"),
        &app.theme,
        &app.lang,
        app.lang.btn_change_cache,
    ) {
        config.set_cache_dir(path);
        config.save().ok();
        cache::set_cache_dir(&config.get_cache_dir());
    }

    let defaults = crate::config::settings::ReaderSettings::default();
    let fg = app.theme.toolbar_btn_fg;
    let pb = app.theme.btn_press_bg;
    let hb = app.theme.btn_hover_bg;

    fn setting_label(ui: &mut egui::Ui, text: &str, fg: egui::Color32, w: f32) -> egui::Response {
        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 20.0), egui::Sense::hover());
        ui.painter_at(r).text(
            egui::pos2(r.left(), r.center().y),
            egui::Align2::LEFT_CENTER, text,
            egui::FontId::proportional(16.0), fg,
        );
        resp
    }

    fn reset_icon_btn(ui: &mut egui::Ui, press_bg: egui::Color32, hover_bg: egui::Color32) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
        let bg = if resp.is_pointer_button_down_on() { press_bg }
            else if resp.hovered() { hover_bg }
            else { egui::Color32::TRANSPARENT };
        ui.painter_at(rect).rect_filled(rect, 6, bg);
        ui.painter_at(rect).text(
            rect.center(), egui::Align2::CENTER_CENTER, "↺",
            egui::FontId::proportional(14.0), egui::Color32::GRAY,
        );
        resp
    }

    fn label_with_hover(ui: &mut egui::Ui, text: &str, tips: &[&str], fg: egui::Color32, w: f32) {
        let resp = setting_label(ui, text, fg, w);
        if !tips.is_empty() {
            resp.on_hover_ui(|ui| {
                ui.label(egui::RichText::new(text).size(14.0).color(egui::Color32::LIGHT_BLUE));
                for (i, tip) in tips.iter().enumerate() {
                    if i > 0 { ui.separator(); }
                    ui.label(egui::RichText::new(*tip).size(14.0));
                }
            });
        }
    }

    fn row_horizontal<F, R>(
        ui: &mut egui::Ui,
        settings: &mut crate::config::settings::ReaderSettings,
        text: &str, tips: &[&str],
        fg: egui::Color32, press_bg: egui::Color32, hover_bg: egui::Color32,
        slider: F, reset: R,
    ) where
        F: FnOnce(&mut egui::Ui, &mut crate::config::settings::ReaderSettings, f32) -> egui::Response,
        R: FnOnce(&mut crate::config::settings::ReaderSettings),
    {
        ui.horizontal(|ui| {
            label_with_hover(ui, text, tips, fg, ui.available_width().min(280.0));
            let slider_w = (ui.available_width() - 80.0).max(120.0);
            if slider(ui, settings, slider_w).changed() { settings.save(); }
            if reset_icon_btn(ui, press_bg, hover_bg).clicked() { reset(settings); }
        });
    }

    fn row_vertical<F, R>(
        ui: &mut egui::Ui,
        settings: &mut crate::config::settings::ReaderSettings,
        text: &str, tips: &[&str],
        fg: egui::Color32, press_bg: egui::Color32, hover_bg: egui::Color32,
        slider: F, reset: R,
    ) where
        F: FnOnce(&mut egui::Ui, &mut crate::config::settings::ReaderSettings, f32) -> egui::Response,
        R: FnOnce(&mut crate::config::settings::ReaderSettings),
    {
        label_with_hover(ui, text, tips, fg, ui.available_width());
        ui.horizontal(|ui| {
            let slider_w = (ui.available_width() - 36.0).max(80.0);
            if slider(ui, settings, slider_w).changed() { settings.save(); }
            if reset_icon_btn(ui, press_bg, hover_bg).clicked() { reset(settings); }
        });
    }

    fn setting_row<F, R>(
        ui: &mut egui::Ui,
        settings: &mut crate::config::settings::ReaderSettings,
        text: &str, tips: &[&str],
        fg: egui::Color32, press_bg: egui::Color32, hover_bg: egui::Color32,
        slider: F, reset: R,
    ) where
        F: FnOnce(&mut egui::Ui, &mut crate::config::settings::ReaderSettings, f32) -> egui::Response,
        R: FnOnce(&mut crate::config::settings::ReaderSettings),
    {
        if ui.available_width() >= 340.0 {
            row_horizontal(ui, settings, text, tips, fg, press_bg, hover_bg, slider, reset);
        } else {
            row_vertical(ui, settings, text, tips, fg, press_bg, hover_bg, slider, reset);
        }
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_image).size(28.0));
    ui.add_space(10.0);

    setting_row(ui, &mut app.settings, app.lang.label_decode_q, &[app.lang.tip_decode_q1, app.lang.tip_decode_q2], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.reader.max_decode_width, 720u32..=3840))
        },
        |s| {
            s.reader.max_decode_width = defaults.reader.max_decode_width;
            s.save();
        });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_layout).size(28.0));
    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_single).size(20.0));
    ui.add_space(10.0);

    setting_row(ui, &mut app.settings, app.lang.label_single_w, &[app.lang.tip_single_w], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.single_width, 300.0..=1200.0))
        },
        |s| {
            s.single_width = defaults.single_width;
            s.save();
        });

    setting_row(ui, &mut app.settings, app.lang.label_width_step, &[app.lang.tip_width_step1, app.lang.tip_width_step2], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.reader.resize_step, 1..=20))
        },
        |s| {
            s.reader.resize_step = defaults.reader.resize_step;
            s.save();
        });

    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_double).size(20.0));
    ui.add_space(10.0);

    setting_row(ui, &mut app.settings, app.lang.label_double_gap, &[app.lang.tip_double_gap1, app.lang.tip_double_gap2], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.reader.double_page_gap, 0.0..=20.0))
        },
        |s| {
            s.reader.double_page_gap = defaults.reader.double_page_gap;
            s.save();
        });

    setting_row(ui, &mut app.settings, app.lang.label_shadow_w, &[app.lang.tip_shadow_w1, app.lang.tip_shadow_w2], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.reader.double_page_shadow_width, 0.0..=600.0))
        },
        |s| {
            s.reader.double_page_shadow_width = defaults.reader.double_page_shadow_width;
            s.save();
        });

setting_row(ui, &mut app.settings, app.lang.label_shadow_op, &[app.lang.tip_shadow_op1, app.lang.tip_shadow_op2], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.reader.double_page_shadow_alpha, 0u32..=255))
        },
        |s| {
            s.reader.double_page_shadow_alpha = defaults.reader.double_page_shadow_alpha;
            s.save();
        });

    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_webtoon).size(20.0));
    ui.add_space(10.0);

setting_row(ui, &mut app.settings, app.lang.label_web_gap, &[app.lang.tip_web_gap], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.webtoon.page_spacing, 0..=20))
        },
        |s| {
            s.webtoon.page_spacing = defaults.webtoon.page_spacing;
            s.save();
        });

    setting_row(ui, &mut app.settings, app.lang.label_web_w, &[app.lang.tip_web_w], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.webtoon_width, 600.0..=1200.0))
        },
        |s| {
            s.webtoon_width = defaults.webtoon_width;
            s.save();
        });

    setting_row(ui, &mut app.settings, app.lang.label_web_w_step, &[app.lang.tip_web_w_step], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.webtoon.resize_step, 1..=20))
        },
        |s| {
            s.webtoon.resize_step = defaults.webtoon.resize_step;
            s.save();
        });

    ui.add_space(10.0);
    ui.label(egui::RichText::new(app.lang.sec_scroll).size(28.0));
    ui.add_space(10.0);

setting_row(ui, &mut app.settings, app.lang.label_scroll_sens, &[app.lang.tip_scroll_sens], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.scroll.wheel_scroll_factor, 0.01..=0.5))
        },
        |s| {
            s.scroll.wheel_scroll_factor = defaults.scroll.wheel_scroll_factor;
            s.save();
        });

    setting_row(ui, &mut app.settings, app.lang.label_scroll_inert, &[app.lang.tip_scroll_inert], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.scroll.inertia_decay, 0.5..=0.99))
        },
        |s| {
            s.scroll.inertia_decay = defaults.scroll.inertia_decay;
            s.save();
        });

    //TODO: Заменить эту дичь на комбобокс с выбором фиксированных значений
    setting_row(ui, &mut app.settings, app.lang.label_scroll_inert_f, &[app.lang.tip_scroll_inert_f], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.scroll.inertia_decay_fps, 30.0..=144.0))
        },
        |s| {
            s.scroll.inertia_decay_fps = defaults.scroll.inertia_decay_fps;
            s.save();
        });

    setting_row(ui, &mut app.settings, app.lang.label_scroll_mult, &[app.lang.tip_scroll_mult], fg, pb, hb,
        |ui, s, w| {
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(&mut s.scroll.wheel_sensitivity, 0.1..=3.0))
        },
        |s| {
            s.scroll.wheel_sensitivity = defaults.scroll.wheel_sensitivity;
            s.save();
        });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
    egui::CollapsingHeader::new(egui::RichText::new(app.lang.sec_sound).size(28.0))
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add(egui::Checkbox::new(
                    &mut app.settings.audio.muted,
                    app.lang.label_mute,
                ));
            });
            ui.add_space(5.0);
            let label_fg = if app.settings.audio.muted {
                egui::Color32::DARK_GRAY
            } else {
                fg
            };
            setting_row(ui, &mut app.settings, app.lang.label_volume, &[], label_fg, pb, hb,
                |ui, s, w| {
                    ui.spacing_mut().slider_width = w;
                    ui.add(egui::Slider::new(&mut s.audio.volume, 0.0..=1.0))
                },
                |s| {
                    s.audio.volume = defaults.audio.volume;
                    s.save();
                });
            ui.add_space(5.0);

            #[cfg(feature = "fx")]
            {
                // папка звуков (Soundpacks): пустая строка = <exe_dir>/Soundpacks (15.1)
                ui.horizontal_wrapped(|ui| {
                    let current = if app.settings.audio.soundpack_dir.is_empty() {
                        crate::fx::resolve::dir().display().to_string()
                    } else {
                        app.settings.audio.soundpack_dir.clone()
                    };
                    let text = format!("Soundpacks 󰖭  {current}");
                    let resp = button::tag_btn(ui, &text, app.theme.cell_fg, &app.theme, 14.0);
                    if resp.clicked() {
                        app.settings.audio.soundpack_dir = String::new();
                        app.settings.save();
                        crate::fx::resolve::reset_dir();
                    }
                    ui.add_space(4.0);
                });
                if let Some(path) = file_picker::pick_folder(
                    ui,
                    egui::Id::new("soundpack_dir"),
                    &app.theme,
                    &app.lang,
                    app.lang.btn_change_soundpack,
                ) {
app.settings.audio.soundpack_dir = path.clone();
                app.settings.save();
                crate::fx::resolve::set_dir(path.into());
                }

                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Checkbox::new(
                            &mut app.settings.effects.enabled,
                            app.lang.label_effects,
                        ))
                        .changed()
                    {
                        app.settings.save();
                    }
                });
            }
            ui.add_space(5.0);
        });
    ui.add_space(200.0);
}
