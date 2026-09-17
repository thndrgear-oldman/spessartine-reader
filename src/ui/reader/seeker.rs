use eframe::egui;

pub struct SeekerAnim {
    pub anim_t: f32,
    pub prev_page: usize,
    pub prev_total: usize,
    pub active: bool,
}

impl SeekerAnim {
    pub fn new() -> Self {
        Self { anim_t: 1.0, prev_page: 0, prev_total: 0, active: false }
    }
}

pub fn draw(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    total: usize,
    page: usize,
    anim: &mut SeekerAnim,
    dt: f32,
) {
    if total < 2 { return; }

    let page = page.min(total.saturating_sub(1));

    if total != anim.prev_total {
        anim.prev_total = total;
        anim.prev_page = 0;
        anim.anim_t = 1.0;
        anim.active = false;
    }

    if page != anim.prev_page && !anim.active {
        anim.active = true;
        anim.anim_t = 0.0;
    }
    if anim.active {
        anim.anim_t = (anim.anim_t + dt * 5.0).min(1.0);
        if anim.anim_t >= 1.0 {
            anim.active = false;
            anim.prev_page = page;
        }
        ui.ctx().request_repaint();
    }

    let float_pos = if anim.active {
        anim.prev_page as f32 + (page as f32 - anim.prev_page as f32) * anim.anim_t
    } else {
        page as f32
    };

    let painter = ui.painter();
    let mid_y = rect.center().y;
    let step = rect.width() / (total - 1) as f32;
    let tick_top = mid_y - 7.0;
    let tick_bot = mid_y + 7.0;
    let gap = 1.5;

    painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(80));

    let sep_y = mid_y;
    painter.line_segment(
        [egui::pos2(rect.left(), sep_y), egui::pos2(rect.right(), sep_y)],
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
    );

    for i in 0..total {
        let x = rect.left() + i as f32 * step;
        let is_trans = anim.active && i == anim.prev_page;

        if is_trans {
            let t = anim.anim_t;
            let ua = ((1.0 - t) * 255.0) as u8;
            painter.line_segment(
                [egui::pos2(x, sep_y - gap), egui::pos2(x, tick_top)],
                egui::Stroke::new(2.0, egui::Color32::from_gray(ua)),
            );
            let la = (t * 255.0) as u8;
            painter.line_segment(
                [egui::pos2(x, sep_y + gap), egui::pos2(x, tick_bot)],
                egui::Stroke::new(2.0, egui::Color32::from_gray(la)),
            );
        } else if i < page {
            painter.line_segment(
                [egui::pos2(x, sep_y + gap), egui::pos2(x, tick_bot)],
                egui::Stroke::new(2.0, egui::Color32::from_gray(160)),
            );
        } else {
            painter.line_segment(
                [egui::pos2(x, tick_top), egui::pos2(x, sep_y - gap)],
                egui::Stroke::new(2.0, egui::Color32::from_gray(80)),
            );
        }
    }

    let vx = rect.left() + (float_pos / (total - 1) as f32) * rect.width();
    let vt = rect.top() + 2.0;
    let vb = vt + 5.0;
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(vx - 3.0, vt),
            egui::pos2(vx + 3.0, vt),
            egui::pos2(vx, vb),
        ],
        egui::Color32::from_gray(200),
        egui::Stroke::NONE,
    ));
}
