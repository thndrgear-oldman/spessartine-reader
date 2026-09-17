use eframe::egui;

pub mod left_panel;
pub mod toolbar;
pub mod stack;
pub mod reader;
pub mod webtoon_reader;
pub mod button;
pub mod file_picker;

pub fn draw(app: &mut crate::SpessartineApp, ctx: &egui::Context) {
    if app.sidebar_visible && !ctx.data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "active")))).unwrap_or(false) {
        egui::SidePanel::left("left_panel")
            .resizable(false)
            .default_width(app.settings.layout.left_panel_width)
            .frame(egui::Frame::new().fill(app.theme.left_panel_bg))
            .show_separator_line(false)
            .show(ctx, |ui| {
                left_panel::draw(app, ui);
            });
    }

    let fs = ctx.data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "active")))).unwrap_or(false);
    let fs_toolbar = ctx.data_mut(|d| d.get_temp::<bool>(egui::Id::new(("fs", "toolbar")))).unwrap_or(false);
    if !fs || fs_toolbar {
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::new()
                .fill(app.theme.toolbar_bg)
                .inner_margin(egui::Margin::symmetric(0, 4)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                toolbar::draw(app, ui);
            });
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(app.theme.library_bg))
        .show(ctx, |ui| {
            stack::draw(app, ui);
        });
}
