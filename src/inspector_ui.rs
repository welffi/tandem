use crate::effects::{hex_rgb, rgb_hex};
use crate::inspector_keys::{KeyAction, Keying, Outcome, Spot, apply, keying, value_at};
use crate::keyframes::{Keyed, edit_field};
use crate::theme::*;
use crate::types::AnimField;
use crate::widgets::{Pill, icon_button, pill, row_label};
use eframe::egui::{Align, Color32, DragValue, Label, Layout, Painter, Pos2, Response, RichText, Sense, Shape, Slider, Stroke, Ui, vec2};

pub struct Row<'a> {
    pub label: &'a str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub scale: f64,
    pub suffix: &'a str,
    pub reset: f64,
}

impl<'a> Row<'a> {
    pub fn percent(label: &'a str, min: f64, max: f64, reset: f64) -> Row<'a> {
        Row { label, min, max, step: 1.0, scale: 100.0, suffix: "%", reset }
    }

    pub fn plain(label: &'a str, min: f64, max: f64, reset: f64, suffix: &'a str) -> Row<'a> {
        Row { label, min, max, step: 1.0, scale: 1.0, suffix, reset }
    }

    pub fn step(self, step: f64) -> Row<'a> {
        Row { step, ..self }
    }
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(title).color(DIM).size(11.0));
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.add(Label::new(RichText::new(text).color(DIM).size(11.0)).wrap());
}

fn rounded(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

fn triangle(painter: &Painter, center: Pos2, facing: f32, color: Color32) {
    let points = vec![center + vec2(-3.0 * facing, -4.5), center + vec2(-3.0 * facing, 4.5), center + vec2(4.0 * facing, 0.0)];
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn diamond(painter: &Painter, center: Pos2, filled: bool, color: Color32) {
    let points = vec![center + vec2(0.0, -5.0), center + vec2(5.0, 0.0), center + vec2(0.0, 5.0), center + vec2(-5.0, 0.0)];
    painter.add(Shape::convex_polygon(points, if filled { color } else { Color32::TRANSPARENT }, Stroke::new(1.4, color)));
}

fn stopwatch(painter: &Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    painter.circle_stroke(center, 6.0, stroke);
    painter.line_segment([center, center + vec2(0.0, -4.0)], stroke);
    painter.line_segment([center, center + vec2(3.0, 1.0)], stroke);
}

fn key_button(ui: &mut Ui, enabled: bool, lit: bool, tip: &str, paint: impl FnOnce(&Painter, Pos2, Color32)) -> bool {
    let color = if lit { ACCENT } else { DIM };
    icon_button(ui, vec2(20.0, 20.0), enabled, |painter, rect, _| paint(painter, rect.center(), color)).on_hover_text(tip).clicked()
}

fn key_buttons(ui: &mut Ui, keying: &Keying) -> Option<KeyAction> {
    let mut action = None;
    if keying.animated {
        if key_button(ui, keying.has_next, false, "Go to the next key", |p, c, color| triangle(p, c, 1.0, color)) {
            action = Some(KeyAction::Next);
        }
        let tip = if keying.here { "Remove the key at the playhead" } else { "Add a key at the playhead" };
        if key_button(ui, !keying.outside, keying.here, tip, |p, c, color| diamond(p, c, keying.here, color)) {
            action = Some(KeyAction::Key);
        }
        if key_button(ui, keying.has_prev, false, "Go to the previous key", |p, c, color| triangle(p, c, -1.0, color)) {
            action = Some(KeyAction::Prev);
        }
    }
    let tip = if keying.animated { "Stop animating, keeping the value at the playhead" } else { "Animate: add a key at the playhead" };
    if key_button(ui, keying.animated || !keying.outside, keying.animated, tip, |p, c, color| stopwatch(p, c, color)) {
        action = Some(KeyAction::Animate);
    }
    action
}

fn is_touched(response: &Response) -> bool {
    response.interact_pointer_pos().is_some() || response.has_focus()
}

pub fn slider_row(ui: &mut Ui, row: &Row, raw: f64, keying: Option<&Keying>) -> (Option<f64>, Option<KeyAction>) {
    let shown = raw * row.scale;
    let mut next = shown;
    let mut action = None;
    let mut reset = false;
    let mut touched = false;
    ui.horizontal(|ui| {
        let label = ui.add(Label::new(RichText::new(row.label)).sense(Sense::click())).on_hover_text("Double-click to reset");
        reset = label.double_clicked();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(keying) = keying {
                action = key_buttons(ui, keying);
            }
            touched |= is_touched(&ui.add(DragValue::new(&mut next).range(row.min..=row.max).speed(row.step).max_decimals(1).suffix(row.suffix)));
        });
    });
    ui.spacing_mut().slider_width = ui.available_width() - 4.0;
    touched |= is_touched(&ui.add(Slider::new(&mut next, row.min..=row.max).step_by(row.step).show_value(false)));
    let change = if reset {
        Some(row.reset)
    } else if touched && (next - shown).abs() > 1e-9 {
        Some(rounded(next / row.scale))
    } else {
        None
    };
    (change, action)
}

pub fn keyed_row<T: Keyed>(ui: &mut Ui, row: &Row, item: &T, field: AnimField, spot: &Spot, fallback: f64) -> Option<Outcome<T>> {
    let value = value_at(item, field, spot, fallback);
    let (change, action) = slider_row(ui, row, value, Some(&keying(item, field, spot)));
    match (change, action) {
        (Some(value), _) => Some(Outcome::Item(edit_field(item, field, spot.source_sec, value))),
        (None, Some(action)) => Some(apply(item, field, spot, fallback, action)),
        _ => None,
    }
}

pub fn number_row(ui: &mut Ui, label: &str, value: f64, range: (f64, f64), step: f64, suffix: &str) -> Option<f64> {
    let mut next = value;
    ui.horizontal(|ui| {
        row_label(ui, label);
        ui.add(DragValue::new(&mut next).range(range.0..=range.1).speed(step).max_decimals(3).suffix(suffix));
    });
    ((next - value).abs() > 1e-9).then_some(rounded(next))
}

pub fn color_row(ui: &mut Ui, label: &str, hex: &str, fallback: &str) -> Option<String> {
    let mut rgb = hex_rgb(hex).or_else(|| hex_rgb(fallback)).unwrap_or([0, 0, 0]);
    let before = rgb;
    ui.horizontal(|ui| {
        row_label(ui, label);
        ui.color_edit_button_srgb(&mut rgb);
    });
    (rgb != before).then(|| rgb_hex(rgb))
}

pub fn choice<T: PartialEq + Copy>(ui: &mut Ui, label: &str, options: &[(T, &str)], current: T) -> Option<T> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        row_label(ui, label);
        for (value, text) in options {
            if pill(ui, text, if *value == current { Pill::On } else { Pill::Plain }).clicked() {
                picked = Some(*value);
            }
        }
    });
    picked
}

pub fn toggle(ui: &mut Ui, label: &str, on: bool, tip: &str) -> Option<bool> {
    let clicked = pill(ui, label, if on { Pill::On } else { Pill::Plain }).on_hover_text(tip).clicked();
    clicked.then_some(!on)
}
