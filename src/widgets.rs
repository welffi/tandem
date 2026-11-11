use crate::theme::{ACCENT, ACCENT_HOVER, ACCENT_SOFT, DIM, HOVER, PANEL_2, PILL, PILL_HOVER, PILL_LINE, SEEK_TRACK, TEXT, WASH_HOVER};
use eframe::egui::{Align, Align2, Button, Color32, ComboBox, CornerRadius, FontId, Layout, Painter, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

const KNOB: f32 = 6.0;

pub fn slider_value(rect: Rect, x: f32) -> f64 {
    let span = (rect.width() - KNOB * 2.0).max(1.0);
    (((x - rect.left() - KNOB) / span).clamp(0.0, 1.0) * 100.0).round() as f64 / 100.0
}

pub fn slider(ui: &mut Ui, value: &mut f64, width: f32) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, 16.0), Sense::click_and_drag());
    if response.is_pointer_button_down_on()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let next = slider_value(rect, pointer.x);
        if next != *value {
            *value = next;
            response.mark_changed();
        }
    }
    let painter = ui.painter();
    let track = Rect::from_center_size(rect.center(), vec2(rect.width() - KNOB * 2.0, 4.0));
    let x = track.left() + track.width() * value.clamp(0.0, 1.0) as f32;
    painter.rect_filled(track, 2.0, SEEK_TRACK);
    painter.rect_filled(Rect::from_min_max(track.min, pos2(x, track.bottom())), 2.0, ACCENT);
    let knob = if response.hovered() || response.dragged() { ACCENT_HOVER } else { ACCENT };
    painter.circle_filled(pos2(x, rect.center().y), KNOB, knob);
    response
}

pub fn icon_button(ui: &mut Ui, size: Vec2, enabled: bool, paint: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 6.0, WASH_HOVER);
    }
    let color = if enabled { TEXT } else { TEXT.gamma_multiply(0.4) };
    paint(ui.painter(), rect, color);
    response
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pill {
    Plain,
    On,
    Accent,
}

pub fn pill(ui: &mut Ui, text: &str, kind: Pill) -> Response {
    let font = FontId::proportional(12.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, TEXT);
    let size = galley.size() + vec2(24.0, 10.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let (fill, line, color) = match kind {
        Pill::Accent => (if hovered { ACCENT_HOVER } else { ACCENT }, ACCENT, Color32::WHITE),
        Pill::On => (if hovered { PILL_HOVER } else { PILL }, ACCENT, ACCENT_SOFT),
        Pill::Plain => (if hovered { PILL_HOVER } else { PILL }, PILL_LINE, TEXT),
    };
    ui.painter().rect(rect, CornerRadius::same(14), fill, Stroke::new(1.0, line), StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

#[derive(Clone, Copy, PartialEq)]
pub enum Look {
    Plain,
    Primary,
    Quiet,
}

pub fn button(ui: &mut Ui, text: &str, look: Look, enabled: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(13.0), TEXT);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(24.0, 10.0), if enabled { Sense::click() } else { Sense::hover() });
    let hovered = enabled && response.hovered();
    let (fill, color) = match look {
        Look::Primary => (if hovered { ACCENT_HOVER } else { ACCENT }, Color32::WHITE),
        Look::Plain => (if hovered { HOVER } else { PANEL_2 }, TEXT),
        Look::Quiet => (if hovered { PANEL_2 } else { Color32::TRANSPARENT }, if hovered { TEXT } else { DIM }),
    };
    let alpha = if enabled { 1.0 } else { 0.4 };
    ui.painter().rect_filled(rect, 6.0, fill.gamma_multiply(alpha));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color.gamma_multiply(alpha));
    response
}

pub fn truncated(painter: &Painter, rect: Rect, align: Align2, text: &str, font: FontId, color: Color32) {
    let fits = |s: &str| painter.layout_no_wrap(s.to_string(), font.clone(), color).size().x <= rect.width();
    let shown = if fits(text) {
        text.to_string()
    } else {
        let chars: Vec<char> = text.chars().collect();
        (0..chars.len()).rev().map(|n| format!("{}…", chars[..n].iter().collect::<String>().trim_end())).find(|s| fits(s)).unwrap_or_default()
    };
    painter.with_clip_rect(rect).text(align.pos_in_rect(&rect), align, shown, font, color);
}

pub fn small_button(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(11.0), DIM);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(12.0, 6.0), Sense::click());
    let hovered = response.hovered();
    ui.painter().rect_filled(rect, 4.0, if hovered { HOVER } else { PANEL_2 });
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, if hovered { TEXT } else { DIM });
    response
}

pub fn row_label(ui: &mut Ui, text: &str) {
    ui.allocate_ui_with_layout(vec2(90.0, 20.0), Layout::left_to_right(Align::Center), |ui| ui.label(RichText::new(text).color(DIM)));
}

pub fn choose(ui: &mut Ui, id: &str, label: &str, enabled: bool, selected: String, add_items: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        row_label(ui, label);
        ui.add_enabled_ui(enabled, |ui| {
            ComboBox::from_id_salt(id).width(ui.available_width().min(230.0)).selected_text(selected).show_ui(ui, add_items);
        });
    });
}

const MENU_W: f32 = 210.0;

pub fn menu_item(ui: &mut Ui, label: &str, hint: &str, enabled: bool) -> bool {
    let clicked = ui.add_enabled(enabled, Button::new(label).shortcut_text(hint).min_size(vec2(MENU_W, 0.0))).clicked();
    if clicked {
        ui.close();
    }
    clicked
}
