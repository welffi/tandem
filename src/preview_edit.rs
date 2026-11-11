use crate::edits::{find_clip, update_clips};
use crate::effects::{Transform, active_effects, new_effect, transform_of};
use crate::framing::{Rect, frame_look, video_rect};
use crate::ids::new_id;
use crate::inspector_text::placed;
use crate::keyframes::{edit_field, effect_at};
use crate::speed::source_at;
use crate::text_render::TextBounds;
use crate::timeline::clip_at;
use crate::types::{AnimField, Clip, Effect, EffectKind, Project, TrackKind};

const CENTER_SNAP: f64 = 0.015;
const MIN_SCALE: f64 = 0.1;
const MAX_SCALE: f64 = 4.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub clip_id: String,
    pub rect: Rect,
    pub rot: f64,
}

#[derive(Clone, Debug)]
pub struct TextGrab {
    pub clip_id: String,
    pub source_sec: f64,
    pub start: (f64, f64),
}

#[derive(Clone, Debug)]
pub struct TransformGrab {
    pub clip_id: String,
    pub source_sec: f64,
    pub original: Effect,
    pub start: Effect,
    pub total: Transform,
}

pub fn snap_center(value: f64, snap: bool) -> f64 {
    if snap && (value - 0.5).abs() < CENTER_SNAP { 0.5 } else { value }
}

pub fn text_under<'a>(texts: &'a [(String, TextBounds)], size: (usize, usize), at: (f64, f64)) -> Option<&'a str> {
    let (w, h) = (size.0 as f64, size.1 as f64);
    texts.iter().rev().find(|(_, b)| at.0 >= b.x / w && at.0 <= (b.x + b.w) / w && at.1 >= b.y / h && at.1 <= (b.y + b.h) / h).map(|(id, _)| id.as_str())
}

pub fn text_grab(project: &Project, clip_id: &str, playhead: f64) -> Option<TextGrab> {
    let (_, clip) = find_clip(project, clip_id)?;
    let text = clip.text.as_ref()?;
    let source_sec = source_at(clip, playhead - clip.timeline_start_sec);
    let now = effect_at(text, source_sec);
    Some(TextGrab { clip_id: clip_id.to_string(), source_sec, start: (now.x, now.y) })
}

pub fn dragged_text(grab: &TextGrab, delta: (f64, f64), snap: bool) -> (f64, f64) {
    let moved = |start: f64, by: f64| snap_center((start + by).clamp(0.0, 1.0), snap);
    (moved(grab.start.0, delta.0), moved(grab.start.1, delta.1))
}

pub fn with_text_position(project: &Project, grab: &TextGrab, at: (f64, f64)) -> Project {
    update_clips(project, std::slice::from_ref(&grab.clip_id), |c| match &c.text {
        Some(text) => Clip { text: Some(placed(text, grab.source_sec, at)), ..c.clone() },
        None => c.clone(),
    })
}

pub fn transform_target(project: &Project, selection: &[String], playhead: f64) -> Option<Target> {
    let (frame_w, frame_h) = (project.width as f64, project.height as f64);
    project.tracks.iter().filter(|t| t.kind == TrackKind::Video && !t.hidden).find_map(|track| {
        let clip = clip_at(track, playhead).filter(|c| selection.contains(&c.id))?;
        let asset = project.assets.iter().find(|a| a.id == clip.asset_id)?;
        let (w, h) = (asset.width.filter(|w| *w > 0)? as f64, asset.height.filter(|h| *h > 0)? as f64);
        let transform = transform_of(&active_effects(clip, Some(playhead - clip.timeline_start_sec)));
        let r = video_rect(frame_w, frame_h, w, h, &frame_look(project), &transform);
        Some(Target { clip_id: clip.id.clone(), rect: Rect { x: r.x / frame_w, y: r.y / frame_h, w: r.w / frame_w, h: r.h / frame_h }, rot: transform.rot })
    })
}

pub fn transform_grab(project: &Project, clip_id: &str, playhead: f64) -> Option<TransformGrab> {
    let (_, clip) = find_clip(project, clip_id)?;
    let local = playhead - clip.timeline_start_sec;
    let source_sec = source_at(clip, local);
    let original = clip.effects().iter().find(|e| e.kind == EffectKind::Transform && e.enabled).cloned().unwrap_or_else(|| new_effect(EffectKind::Transform, new_id()));
    let start = effect_at(&original, source_sec);
    Some(TransformGrab { clip_id: clip_id.to_string(), source_sec, original, start, total: transform_of(&active_effects(clip, Some(local))) })
}

fn edited(grab: &TransformGrab, values: &[(AnimField, f64)]) -> Effect {
    values.iter().fold(grab.original.clone(), |effect, (field, value)| edit_field(&effect, *field, grab.source_sec, *value))
}

pub fn moved_effect(grab: &TransformGrab, delta: (f64, f64), snap: bool) -> Effect {
    let x = snap_center(grab.total.x + delta.0 + 0.5, snap) - 0.5;
    let y = snap_center(grab.total.y + delta.1 + 0.5, snap) - 0.5;
    edited(grab, &[(AnimField::X, grab.start.x.unwrap_or(0.0) + x - grab.total.x), (AnimField::Y, grab.start.y.unwrap_or(0.0) + y - grab.total.y)])
}

pub fn scaled_effect(grab: &TransformGrab, ratio: f64) -> Effect {
    let scaled = |value: f64| (value * ratio).clamp(MIN_SCALE, MAX_SCALE);
    let mut values = vec![(AnimField::Size, scaled(grab.start.size.unwrap_or(1.0)))];
    values.extend(grab.start.height.map(|h| (AnimField::Height, scaled(h))));
    edited(grab, &values)
}

pub fn with_effect(project: &Project, clip_id: &str, next: &Effect) -> Project {
    update_clips(project, &[clip_id.to_string()], |c| {
        let mut list = c.effects().to_vec();
        match list.iter_mut().find(|e| e.id == next.id) {
            Some(slot) => *slot = next.clone(),
            None => list.push(next.clone()),
        }
        Clip { effects: Some(list), ..c.clone() }
    })
}
