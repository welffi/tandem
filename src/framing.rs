use crate::effects::{IDENTITY, Transform};
use crate::types::{EdgeFill, Framing, Project};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameLook {
    pub framing: Framing,
    pub scale: f64,
    pub edge_fill: EdgeFill,
    pub edge_blur: f64,
    pub shadow: f64,
    pub shadow_size: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub const MIN_SCALE: f64 = 0.25;
pub const MAX_SCALE: f64 = 4.0;

fn percent(value: Option<f64>, fallback: f64) -> f64 {
    value.unwrap_or(fallback).clamp(0.0, 100.0)
}

pub fn frame_look(project: &Project) -> FrameLook {
    FrameLook {
        framing: project.framing.unwrap_or_default(),
        scale: project.video_scale.unwrap_or(1.0).clamp(MIN_SCALE, MAX_SCALE),
        edge_fill: project.edge_fill.unwrap_or_default(),
        edge_blur: percent(project.edge_blur, 40.0),
        shadow: percent(project.shadow, 0.0),
        shadow_size: percent(project.shadow_size, 40.0),
    }
}

pub fn video_rect(frame_w: f64, frame_h: f64, src_w: f64, src_h: f64, look: &FrameLook, transform: &Transform) -> Rect {
    let (a, b) = (frame_w / src_w, frame_h / src_h);
    let pick = if look.framing == Framing::Fill { a.max(b) } else { a.min(b) };
    let s = pick * look.scale;
    let w = src_w * s * transform.sx;
    let h = src_h * s * transform.sy;
    Rect { x: (frame_w - w) / 2.0 + transform.x * frame_w, y: (frame_h - h) / 2.0 + transform.y * frame_h, w, h }
}

pub fn plain_rect(frame_w: f64, frame_h: f64, src_w: f64, src_h: f64, look: &FrameLook) -> Rect {
    video_rect(frame_w, frame_h, src_w, src_h, look, &IDENTITY)
}

pub fn rotated_bounds(rect: Rect, degrees: f64) -> Rect {
    let a = degrees.to_radians();
    let (cos, sin) = (a.cos().abs(), a.sin().abs());
    let w = rect.w * cos + rect.h * sin;
    let h = rect.w * sin + rect.h * cos;
    Rect { x: rect.x + (rect.w - w) / 2.0, y: rect.y + (rect.h - h) / 2.0, w, h }
}

pub fn blur_sigma(frame_h: f64, edge_blur: f64) -> f64 {
    edge_blur * frame_h / 1000.0
}

pub fn has_shadow(look: &FrameLook) -> bool {
    look.edge_fill != EdgeFill::Black && look.shadow > 0.0
}

pub fn shadow_sigma(frame_h: f64, shadow_size: f64) -> f64 {
    (shadow_size * frame_h / 1000.0).max(1.0)
}

pub fn shadow_drop(sigma: f64) -> f64 {
    sigma / 3.0
}

pub fn covers_frame(rect: &Rect, frame_w: f64, frame_h: f64) -> bool {
    rect.x <= 0.5 && rect.y <= 0.5 && rect.x + rect.w >= frame_w - 0.5 && rect.y + rect.h >= frame_h - 0.5
}
