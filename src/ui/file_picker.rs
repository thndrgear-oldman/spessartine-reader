use eframe::egui;
use crate::theme::AppTheme;
use crate::lang::Lang;

#[derive(Default, Clone)]
struct InputState {
    show_input: bool,
    buf: String,
    suggestions: Vec<String>,
    selected_index: usize,
    pending_confirm: bool,
}

fn completions(path: &str) -> Vec<String> {
    let (parent, prefix) = if path.is_empty() || path == "/" {
        ("/".to_string(), String::new())
    } else if path.ends_with('/') {
        (path.to_string(), String::new())
    } else {
        let p = std::path::Path::new(path);
        let par = p.parent()
            .map(|p| p.display().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/".to_string());
        let pre = p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let par = if par.ends_with('/') { par } else { format!("{}/", par) };
        (par, pre)
    };

    let mut entries: Vec<String> = match std::fs::read_dir(&parent) {
        Ok(rd) => rd.filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !prefix.starts_with('.') {
                return None;
            }
            if !name.starts_with(&prefix) {
                return None;
            }
            e.file_type().ok().filter(|t| t.is_dir())?;
            Some(format!("{}{}/", parent, name))
        }).collect(),
        Err(_) => vec![],
    };

    entries.sort();
    entries.truncate(20);
    entries
}

fn show_suggestions(
    ctx: &egui::Context,
    pos: egui::Pos2,
    width: f32,
    suggestions: &[String],
    selected: usize,
    theme: &AppTheme,
) -> Option<usize> {
    let mut clicked = None;
    egui::Area::new(egui::Id::new("path_suggestions"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(theme.toolbar_bg)
                .corner_radius(egui::CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(4, 2))
                .show(ui, |ui| {
                    let w = width.max(200.0);
                    ui.set_min_width(w);
                    for (i, sug) in suggestions.iter().enumerate() {
                        let is_selected = i == selected;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(w, 20.0),
                            egui::Sense::click(),
                        );
                        let painter = ui.painter_at(rect);
                        if is_selected {
                            painter.rect_filled(rect, 2.0, theme.cell_bg);
                        }
                        painter.text(
                            egui::pos2(rect.min.x + 4.0, rect.min.y),
                            egui::Align2::LEFT_TOP,
                            sug.as_str(),
                            egui::FontId::proportional(13.0),
                            theme.cell_fg,
                        );
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                    }
                });
        });

    clicked
}

fn cursor_at_end(ctx: &egui::Context, id: egui::Id, buf: &str) {
    let text_len = buf.chars().count();
    if let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) {
        state.cursor.set_char_range(Some(
            egui::text::CCursorRange::one(egui::text::CCursor::new(text_len)),
        ));
        state.store(ctx, id);
    }
}

pub fn pick_folder(
    ui: &mut egui::Ui,
    id: egui::Id,
    theme: &AppTheme,
    lang: &Lang,
    label: &str,
) -> Option<String> {
    let mut result = None;
    let mut state: InputState = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());

    if !state.show_input {
        let resp = crate::ui::button::tag_btn(ui, label, theme.cell_fg, theme, 14.0);
        if resp.clicked() {
            #[cfg(target_os = "windows")]
            {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    let p = path.display().to_string();
                    if !p.is_empty() {
                        result = Some(p);
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                match std::process::Command::new("zenity")
                    .args(["--file-selection", "--directory"])
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !path.is_empty() {
                            result = Some(path);
                        } else {
                            state.show_input = true;
                        }
                    }
                    _ => state.show_input = true,
                }
            }
        }
    }

    if state.show_input && result.is_none() {
        ui.ctx().memory_mut(|m| m.move_focus(egui::FocusDirection::None));

        let old_buf = state.buf.clone();

        let text_resp = ui.add_sized(
            egui::vec2(ui.available_width(), 24.0),
            egui::TextEdit::singleline(&mut state.buf)
                .hint_text(lang.fp_hint)
                .font(egui::TextStyle::Body)
                .desired_width(f32::INFINITY)
                .text_color(theme.cell_fg),
        );

        text_resp.request_focus();

        let tab = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
        let shift_tab = ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
        let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let esc = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

        if esc {
            state.show_input = false;
            state.buf.clear();
            state.suggestions.clear();
            state.selected_index = 0;
            state.pending_confirm = false;
        } else if tab || shift_tab {
            if state.suggestions.is_empty() {
                state.suggestions = completions(&state.buf);
                state.selected_index = 0;
            }
            if !state.suggestions.is_empty() {
                let n = state.suggestions.len();
                state.selected_index = if tab {
                    (state.selected_index + 1) % n
                } else {
                    (state.selected_index + n - 1) % n
                };
            }
        } else if enter {
            if state.buf.is_empty() {
                state.show_input = false;
            } else if state.pending_confirm {
                result = Some(state.buf.clone());
                state.show_input = false;
                state.buf.clear();
                state.suggestions.clear();
                state.selected_index = 0;
                state.pending_confirm = false;
            } else if !state.suggestions.is_empty()
                && state.selected_index < state.suggestions.len()
            {
                state.buf.clone_from(&state.suggestions[state.selected_index]);
                state.pending_confirm = true;
                cursor_at_end(ui.ctx(), text_resp.id, &state.buf);
            } else {
                result = Some(state.buf.clone());
                state.show_input = false;
                state.buf.clear();
                state.suggestions.clear();
                state.selected_index = 0;
                state.pending_confirm = false;
            }
        }

        if state.buf != old_buf && !tab && !shift_tab && !enter {
            state.suggestions = completions(&state.buf);
            state.selected_index = 0;
            state.pending_confirm = false;
        }

        if !state.suggestions.is_empty() {
            let pos = text_resp.rect.left_bottom();
            let w = text_resp.rect.width();
            if let Some(idx) = show_suggestions(
                ui.ctx(), pos, w, &state.suggestions, state.selected_index, theme,
            ) {
                state.buf.clone_from(&state.suggestions[idx]);
                state.pending_confirm = true;
                cursor_at_end(ui.ctx(), text_resp.id, &state.buf);
            } else if ui.input_mut(|i| i.pointer.any_click()) {
                state.suggestions.clear();
            }
        }
    }

    ui.data_mut(|d| d.insert_temp(id, state));
    result
}
