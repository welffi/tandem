use crate::edits::update_clips;
use crate::keyframes::{LINEAR, clip_keys, map_clip_keys, map_keys};
use crate::store::key_ref;
use crate::timeline_ops::keys_by_clip;
use crate::types::{Bezier, CurvePreset, Keyframe, Project};
use eframe::egui::{Pos2, Rect, Vec2, pos2};

const FINE_STEP: f64 = 0.001;
const SNAP_STEP: f64 = 0.05;
const SAME_CURVE: f64 = 1e-3;

pub enum CurveChange {
    Add(CurvePreset),
    Rename(String, String),
    Delete(String),
}

pub fn changed(list: &[CurvePreset], change: &CurveChange) -> Vec<CurvePreset> {
    match change {
        CurveChange::Add(preset) => list.iter().cloned().chain([preset.clone()]).collect(),
        CurveChange::Rename(id, name) => list.iter().map(|c| if &c.id == id { CurvePreset { name: name.clone(), ..c.clone() } } else { c.clone() }).collect(),
        CurveChange::Delete(id) => list.iter().filter(|c| &c.id != id).cloned().collect(),
    }
}

pub fn same_curve(a: Bezier, b: Bezier) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < SAME_CURVE)
}

pub fn selected_ease(project: &Project, refs: &[String]) -> Bezier {
    project
        .tracks
        .iter()
        .flat_map(|t| &t.clips)
        .filter(|c| refs.iter().any(|r| r.split('/').next() == Some(c.id.as_str())))
        .flat_map(|c| clip_keys(c).into_iter().filter(|k| refs.contains(&key_ref(&c.id, &k.id))))
        .find_map(|k| k.ease)
        .unwrap_or(LINEAR)
}

pub fn eased(project: &Project, refs: &[String], ease: Bezier) -> Project {
    let by_clip = keys_by_clip(refs);
    let ids: Vec<String> = by_clip.keys().cloned().collect();
    update_clips(project, &ids, |clip| {
        let picked = by_clip.get(&clip.id).cloned().unwrap_or_default();
        let set = |k: &Keyframe| if picked.contains(&k.id) { Keyframe { ease: Some(ease), ..k.clone() } } else { k.clone() };
        map_clip_keys(clip, &mut |e| map_keys(e, &mut |k| set(k)), &mut |t| map_keys(t, &mut |k| set(k)))
    })
}

fn snapped(value: f64, snap: bool) -> f64 {
    let step = if snap { SNAP_STEP } else { FINE_STEP };
    (value / step).round() * step
}

pub fn dragged_handle(bezier: Bezier, handle: usize, (x, y): (f64, f64), snap: bool, (low, high): (f64, f64)) -> Bezier {
    let (x, y) = (snapped(x, snap).clamp(0.0, 1.0), snapped(y, snap).clamp(low, high));
    let [x1, y1, x2, y2] = bezier;
    if handle == 0 { [x, y, x2, y2] } else { [x1, y1, x, y] }
}

pub fn unit_box_in(view: Rect, margin: Vec2) -> Rect {
    Rect::from_min_max(view.min + margin, view.max - margin)
}

pub fn visible_y(view: Rect, unit_box: Rect) -> (f64, f64) {
    (from_screen(unit_box, view.left_bottom()).1, from_screen(unit_box, view.left_top()).1)
}

pub fn to_screen(unit_box: Rect, (x, y): (f64, f64)) -> Pos2 {
    pos2(unit_box.left() + x as f32 * unit_box.width(), unit_box.bottom() - y as f32 * unit_box.height())
}

pub fn from_screen(unit_box: Rect, at: Pos2) -> (f64, f64) {
    (((at.x - unit_box.left()) / unit_box.width()) as f64, ((unit_box.bottom() - at.y) / unit_box.height()) as f64)
}

pub fn numbers(bezier: Bezier) -> String {
    bezier.iter().map(|v| ((v * 100.0).round() / 100.0).to_string()).collect::<Vec<_>>().join(", ")
}
