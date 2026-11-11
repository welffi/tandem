use crate::theme::*;
use eframe::egui::{Align2, Color32, FontId, Painter, Rect, Stroke, StrokeKind, pos2, vec2};

const GAP: f32 = 8.0;
const CLEAR: f32 = 6.0;
const PAD_X: f32 = 4.0;
const HEIGHT: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Left,
    Right,
}

pub struct Tag {
    pub x: f32,
    pub text: String,
    pub selected: bool,
}

pub fn tag_sides(xs: &[f32], widths: &[f32], left: f32, right: f32) -> Vec<Option<Side>> {
    let mut taken = left;
    xs.iter()
        .zip(widths)
        .enumerate()
        .map(|(i, (&x, &width))| {
            let room_right = xs.get(i + 1).map_or(right, |next| next - CLEAR);
            let side = if x + GAP + width <= room_right {
                Some(Side::Right)
            } else if x - GAP - width >= taken {
                Some(Side::Left)
            } else {
                None
            };
            taken = match side {
                Some(Side::Right) => x + GAP + width,
                _ => x + CLEAR,
            };
            side
        })
        .collect()
}

fn font() -> FontId {
    FontId::proportional(10.0)
}

fn width_of(painter: &Painter, text: &str) -> f32 {
    painter.layout_no_wrap(text.to_string(), font(), TEXT).size().x + PAD_X * 2.0
}

pub fn draw(painter: &Painter, center_y: f32, tags: &[Tag], left: f32, right: f32) {
    let xs: Vec<f32> = tags.iter().map(|t| t.x).collect();
    let widths: Vec<f32> = tags.iter().map(|t| width_of(painter, &t.text)).collect();
    for ((tag, width), side) in tags.iter().zip(&widths).zip(tag_sides(&xs, &widths, left, right)) {
        let Some(side) = side else { continue };
        let start = if side == Side::Right { tag.x + GAP } else { tag.x - GAP - width };
        let rect = Rect::from_min_size(pos2(start, center_y - HEIGHT / 2.0), vec2(*width, HEIGHT));
        let (fill, text) = if tag.selected { (ACCENT, Color32::WHITE) } else { (PANEL_2, TEXT) };
        painter.rect(rect, 4.0, fill, Stroke::new(1.0, BORDER), StrokeKind::Inside);
        painter.text(rect.center(), Align2::CENTER_CENTER, &tag.text, font(), text);
    }
}
