use eframe::egui;

mod title_grid;
mod chapters_grid;
mod pages_grid;

pub fn draw(app: &mut crate::SpessartineApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            let level = app.reader.nav_stack.last().cloned();
            match level {
                Some(crate::NavigationLevel::Titles) => title_grid::draw(app, ui),
                Some(crate::NavigationLevel::Chapters { .. }) => chapters_grid::draw(app, ui),
                Some(crate::NavigationLevel::Pages { .. }) => pages_grid::draw(app, ui),
                Some(crate::NavigationLevel::Reading { .. }) | Some(crate::NavigationLevel::ReadingWebtoon { .. }) => {}
                _none => { ui.label(app.lang.nav_error); }
            }
        });
}
