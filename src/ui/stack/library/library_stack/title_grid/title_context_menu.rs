use eframe::egui;

pub fn draw(app: &mut crate::SpessartineApp, idx: usize, ui: &mut egui::Ui) {
    use crate::ui::reader::reader_context_menu::MenuAction;
    ui.menu_button(app.lang.reading_mode, |ui| {
        let manual_mode = app.lib.manga_list[idx].manual_mode;
        let rtl = app.lib.manga_list[idx].effective_rtl();
        let (ma, ms, md, mw) = (app.lang.mode_auto, app.lang.mode_single, app.lang.mode_double, app.lang.mode_webtoon);
        crate::ui::reader::reader_context_menu::draw(ui, manual_mode, rtl, ma, ms, md, mw, |action| {
            let id = app.lib.manga_list[idx].id;
            match action {
                MenuAction::SetMode(mode) => { app.reader.set_manga_mode(&mut app.lib, &mut app.db, id, mode); }
                MenuAction::SetRtl(rtl) => { app.reader.set_manga_rtl(&mut app.lib, &mut app.db, id, rtl); }
            }
        });
    });
    ui.separator();
    // TODO: "Переименовать", "Удалить" и т.д.
}
