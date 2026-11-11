use crate::app::App;
use crate::edits::update_clips;
use crate::inspector_clip::seek_source;
use crate::inspector_keys::{Outcome, Spot, spot};
use crate::inspector_ui::{Row, choice, color_row, keyed_row, toggle};
use crate::keyframes::edit_field;
use crate::types::{AnimField, Clip, TextAlign, TextStyle};
use crate::widgets::{choose, row_label, small_button};
use eframe::egui::{TextEdit, Ui};

const FONTS: [&str; 11] = ["Segoe UI", "Arial", "Arial Black", "Impact", "Verdana", "Trebuchet MS", "Georgia", "Times New Roman", "Courier New", "Consolas", "Comic Sans MS"];
const MIN_SIZE: f64 = 2.0;
const MAX_SIZE: f64 = 30.0;
const CENTER: (f64, f64) = (0.5, 0.5);
const BOTTOM: (f64, f64) = (0.5, 0.85);

pub fn placed(text: &TextStyle, source_sec: f64, (x, y): (f64, f64)) -> TextStyle {
    edit_field(&edit_field(text, AnimField::X, source_sec, x), AnimField::Y, source_sec, y)
}

struct Panel<'a> {
    text: &'a TextStyle,
    spot: Spot,
    next: Option<(TextStyle, Option<String>)>,
    seek: Option<f64>,
}

impl Panel<'_> {
    fn set(&mut self, next: TextStyle, key: &str) {
        self.next = Some((next, Some(key.to_string())));
    }

    fn field(&mut self, ui: &mut Ui, row: Row, field: AnimField, fallback: f64) {
        match keyed_row(ui, &row, self.text, field, &self.spot, fallback) {
            Some(Outcome::Item(next)) => self.set(next, &format!("text:{:?}", field)),
            Some(Outcome::Seek(source_sec)) => self.seek = Some(source_sec),
            None => {}
        }
    }

    fn content(&mut self, ui: &mut Ui) {
        let mut content = self.text.content.clone();
        ui.add(TextEdit::multiline(&mut content).desired_rows(3).desired_width(f32::INFINITY));
        if content != self.text.content {
            self.set(TextStyle { content, ..self.text.clone() }, "text-content");
        }
    }

    fn font(&mut self, ui: &mut Ui) {
        choose(ui, "text-font", "Font", true, self.text.font.clone(), |ui| {
            let others = (!FONTS.contains(&self.text.font.as_str())).then(|| self.text.font.clone());
            for name in others.iter().map(String::as_str).chain(FONTS) {
                if ui.selectable_label(self.text.font == name, name).clicked() {
                    self.set(TextStyle { font: name.into(), ..self.text.clone() }, "text-font");
                }
            }
        });
        if let Some(color) = color_row(ui, "Color", &self.text.color, "#ffffff") {
            self.set(TextStyle { color, ..self.text.clone() }, "text-color");
        }
    }

    fn style(&mut self, ui: &mut Ui) {
        let text = self.text;
        ui.horizontal_wrapped(|ui| {
            row_label(ui, "Style");
            let flags = [("Bold", text.bold, "bold"), ("Italic", text.italic, "italic"), ("Outline", text.outline, "outline"), ("Box", text.boxed, "box")];
            for (label, on, name) in flags {
                let tip = if name == "box" { "Dark box behind the text" } else { "" };
                if let Some(on) = toggle(ui, label, on, tip) {
                    let next = match name {
                        "bold" => TextStyle { bold: on, ..text.clone() },
                        "italic" => TextStyle { italic: on, ..text.clone() },
                        "outline" => TextStyle { outline: on, ..text.clone() },
                        _ => TextStyle { boxed: on, ..text.clone() },
                    };
                    self.set(next, name);
                }
            }
        });
        let aligns = [(TextAlign::Left, "Left"), (TextAlign::Center, "Center"), (TextAlign::Right, "Right")];
        if let Some(align) = choice(ui, "Align", &aligns, text.align) {
            self.set(TextStyle { align, ..text.clone() }, "align");
        }
    }

    fn position(&mut self, ui: &mut Ui) {
        self.field(ui, Row::percent("Left / right", 0.0, 100.0, 0.5).step(0.5), AnimField::X, self.text.x);
        self.field(ui, Row::percent("Up / down", 0.0, 100.0, 0.5).step(0.5), AnimField::Y, self.text.y);
        ui.horizontal(|ui| {
            row_label(ui, "");
            for (label, spot) in [("Center", CENTER), ("Bottom", BOTTOM)] {
                if small_button(ui, label).clicked() {
                    self.next = Some((placed(self.text, self.spot.source_sec, spot), None));
                }
            }
        });
    }
}

pub fn show(ui: &mut Ui, app: &mut App, clip: &Clip, text: &TextStyle) {
    let mut panel = Panel { text, spot: spot(clip, app.store.playhead_sec), next: None, seek: None };
    panel.content(ui);
    panel.font(ui);
    panel.field(ui, Row::plain("Size", MIN_SIZE, MAX_SIZE, 8.0, "%").step(0.5), AnimField::Size, text.size);
    panel.style(ui);
    panel.position(ui);
    if let Some(source_sec) = panel.seek {
        seek_source(app, clip, source_sec);
    }
    if let Some((next, key)) = panel.next {
        let project = update_clips(app.store.project(), &[clip.id.clone()], |c| Clip { text: Some(next.clone()), ..c.clone() });
        app.store.edit(project, key.as_deref());
    }
}
