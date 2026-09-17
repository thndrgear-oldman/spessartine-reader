use eframe::egui;

pub mod library_stack;
mod fav_tabs;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    if matches!(app.reader.nav_stack.last(), Some(crate::NavigationLevel::Reading { .. }) | Some(crate::NavigationLevel::ReadingWebtoon { .. })) {
        crate::ui::reader::draw(app, ui);
        return;
    }

    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, app.theme.library_bg);

    //let fav_height = 36.0;

    //ui.add_space(fav_height);

    library_stack::draw(app, ui);

    // Draw fav_tabs overlay on top
    let top_rect = egui::Rect::from_min_max(
        ui.max_rect().left_top(),
        egui::pos2(ui.max_rect().right(), ui.max_rect().top()), //+ fav_height),
    );
    ui.painter().rect_filled(top_rect, 0.0, app.theme.library_bg);
    ui.scope_builder(egui::UiBuilder::new().max_rect(top_rect), |ui| {
        let centering = (ui.available_height() - 28.0) / 2.0;
        ui.add_space(centering.max(0.0));
        //fav_tabs::draw(ui, &app.theme);
    });
}
