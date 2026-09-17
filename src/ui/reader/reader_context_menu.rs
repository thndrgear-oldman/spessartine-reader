use eframe::egui;
use crate::reader::mode::ReaderMode;

pub enum MenuAction {
    SetMode(Option<ReaderMode>),
    SetRtl(bool),
}

pub fn draw(
    ui: &mut egui::Ui,
    manual_mode: Option<ReaderMode>,
    rtl: bool,
    mode_auto: &str,
    mode_single: &str,
    mode_double: &str,
    mode_webtoon: &str,
    mut on_action: impl FnMut(MenuAction),
) {
    ui.horizontal(|ui| {
        let modes = [
            ("", None, mode_auto),
            ("󰧮", Some(ReaderMode::Manga), mode_single),
            ("󰗚", Some(ReaderMode::Double), mode_double),
            ("", Some(ReaderMode::Webtoon), mode_webtoon),
        ];
        for (icon, mode, label) in modes {
            let selected = manual_mode == mode;
            let txt = egui::RichText::new(format!("{} {}", icon, label))
                .size(14.0)
                .color(if selected { egui::Color32::from_rgb(0x4a, 0x9e, 0xff) } else { egui::Color32::GRAY });
            if ui.selectable_label(selected, txt).clicked() {
                on_action(MenuAction::SetMode(mode));
                ui.close();
            }
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        let ltr_txt = egui::RichText::new("󰮱 LTR")
            .size(14.0)
            .color(if !rtl { egui::Color32::from_rgb(0x4a, 0x9e, 0xff) } else { egui::Color32::GRAY });
        if ui.selectable_label(!rtl, ltr_txt).clicked() {
            on_action(MenuAction::SetRtl(false));
            ui.close();
        }
        let rtl_txt = egui::RichText::new("RTL 󰮳")
            .size(14.0)
            .color(if rtl { egui::Color32::from_rgb(0x4a, 0x9e, 0xff) } else { egui::Color32::GRAY });
        if ui.selectable_label(rtl, rtl_txt).clicked() {
            on_action(MenuAction::SetRtl(true));
            ui.close();
        }
    });
}
