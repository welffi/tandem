use crate::edits::{duplicate_clips, find_clip, move_clips, set_fade, snap_points, trim_clips, update_track};
use crate::keyframes::{map_clip_keys, map_keys};
use crate::speed::{local_at, source_at};
use crate::timeline::{Snap, TrimSide, clip_duration, clip_end, find_snap};
use crate::timeline_math::SNAP_PX;
use crate::types::{Clip, Keyframe, Project, Track, TrackKind};
use eframe::egui::Pos2;

pub struct DragView {
    pub project: Project,
    pub snap_sec: Option<f64>,
}

pub enum Drag {
    Scrub,
    Move { base: Project, anchor: String, clip_ids: Vec<String>, track_id: String, grab_offset: f64, down: Pos2, active: bool, toggle: bool, copy: bool },
    Trim { base: Project, anchor: String, clip_ids: Vec<String>, side: TrimSide, range: (f64, f64) },
    Fade { base: Project, anchor: String, clip_ids: Vec<String>, side: TrimSide },
    Key { base: Project, anchor: String, key_ids: Vec<String>, down_sec: f64 },
}

pub struct Pointing {
    pub sec: f64,
    pub lane: Option<(String, TrackKind)>,
    pub snapping: bool,
    pub alt: bool,
    pub playhead: f64,
    pub px_per_sec: f64,
}

impl Drag {
    pub fn start_moving(&mut self) {
        let Drag::Move { base, anchor, clip_ids, active, copy, .. } = self else { return };
        if *active {
            return;
        }
        *active = true;
        if *copy {
            let (project, ids) = duplicate_clips(base, clip_ids);
            *base = project;
            *clip_ids = clip_ids.iter().filter_map(|id| ids.get(id).cloned()).collect();
            *anchor = ids.get(anchor).cloned().unwrap_or_default();
        }
    }

    pub fn is_copy_move(&self) -> bool {
        matches!(self, Drag::Move { copy: true, .. })
    }
}

pub fn move_keys(project: &Project, clip_id: &str, key_ids: &[String], delta_sec: f64) -> Project {
    let Some((track, clip)) = find_clip(project, clip_id) else { return project.clone() };
    let span = clip_duration(clip);
    let shift = |k: &Keyframe| {
        if !key_ids.contains(&k.id) {
            return k.clone();
        }
        Keyframe { source_sec: source_at(clip, (local_at(clip, k.source_sec) + delta_sec).clamp(0.0, span)), ..k.clone() }
    };
    let moved = map_clip_keys(clip, &mut |e| map_keys(e, &mut |k| shift(k)), &mut |t| map_keys(t, &mut |k| shift(k)));
    update_track(project, &track.id, |t| Track { clips: t.clips.iter().map(|c| if c.id == clip_id { moved.clone() } else { c.clone() }).collect(), ..t.clone() })
}

fn snapped(edges: &[f64], base: &Project, exclude: &[String], at: &Pointing) -> Option<Snap> {
    if !at.snapping || at.alt {
        return None;
    }
    find_snap(edges, &snap_points(base, exclude, &[at.playhead]), SNAP_PX / at.px_per_sec)
}

fn moved_view(drag: &mut Drag, at: &Pointing) -> Option<DragView> {
    let Drag::Move { base, anchor, clip_ids, track_id, grab_offset, .. } = drag else { return None };
    let (anchor_track, clip) = find_clip(base, anchor)?;
    if clip_ids.len() == 1
        && let Some((id, kind)) = &at.lane
        && *kind == anchor_track.kind
    {
        *track_id = id.clone();
    }
    let mut delta = (at.sec - *grab_offset).max(0.0) - clip.timeline_start_sec;
    let moving: Vec<&Clip> = clip_ids.iter().filter_map(|id| find_clip(base, id).map(|(_, c)| c)).collect();
    let edges: Vec<f64> = moving.iter().flat_map(|c| [c.timeline_start_sec + delta, clip_end(c) + delta]).collect();
    let snap = snapped(&edges, base, clip_ids, at);
    if let Some(s) = snap {
        delta += s.delta;
    }
    Some(DragView { project: move_clips(base, clip_ids, delta, Some(track_id)), snap_sec: snap.map(|s| s.point) })
}

fn trimmed_view(base: &Project, clip_ids: &[String], side: TrimSide, range: (f64, f64), at: &Pointing) -> DragView {
    let snap = snapped(&[at.sec], base, clip_ids, at);
    let wanted = snap.map_or(at.sec, |s| s.point);
    let clamped = wanted.max(range.0).min(range.1);
    let snap_sec = snap.map(|s| s.point).filter(|p| *p == clamped);
    DragView { project: trim_clips(base, clip_ids, side, clamped), snap_sec }
}

pub fn compute(drag: &mut Drag, at: &Pointing) -> Option<DragView> {
    match drag {
        Drag::Scrub => None,
        Drag::Move { .. } => moved_view(drag, at),
        Drag::Trim { base, clip_ids, side, range, .. } => Some(trimmed_view(base, clip_ids, *side, *range, at)),
        Drag::Fade { base, anchor, clip_ids, side } => {
            let (_, clip) = find_clip(base, anchor)?;
            let sec = if *side == TrimSide::Start { at.sec - clip.timeline_start_sec } else { clip_end(clip) - at.sec };
            Some(DragView { project: set_fade(base, clip_ids, *side, sec), snap_sec: None })
        }
        Drag::Key { base, anchor, key_ids, down_sec } => Some(DragView { project: move_keys(base, anchor, key_ids, at.sec - *down_sec), snap_sec: None }),
    }
}

pub fn dragged_ids(drag: &Drag) -> Option<&[String]> {
    match drag {
        Drag::Move { clip_ids, .. } => Some(clip_ids),
        _ => None,
    }
}
