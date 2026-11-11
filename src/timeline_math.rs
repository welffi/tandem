use crate::edits::find_clip;
use crate::speed::local_at;
use crate::timeline::{TrimSide, clip_duration, clip_end, clip_fades, trim_range};
use crate::types::{Clip, Keyframe, Project, Track, TrackKind};

pub const RULER_H: f32 = 26.0;
pub const HEADER_W: f32 = 170.0;
pub const EDGE_PX: f32 = 7.0;
pub const FADE_ZONE_PX: f32 = 12.0;
pub const FADE_INSET_PX: f32 = 5.0;
pub const KEY_R: f32 = 5.0;
pub const SNAP_PX: f64 = 9.0;
pub const MIN_PX_PER_SEC: f64 = 2.0;
pub const MAX_PX_PER_SEC: f64 = 2000.0;
pub const LANE_PAD: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lane {
    pub index: usize,
    pub y: f32,
    pub h: f32,
}

pub fn lane_height(kind: TrackKind) -> f32 {
    match kind {
        TrackKind::Text => 40.0,
        TrackKind::Video => 60.0,
        TrackKind::Audio => 56.0,
    }
}

pub fn layout_lanes(tracks: &[Track]) -> (Vec<Lane>, f32) {
    let mut y = 0.0;
    let lanes = tracks
        .iter()
        .enumerate()
        .map(|(index, t)| {
            let lane = Lane { index, y, h: lane_height(t.kind) };
            y += lane.h;
            lane
        })
        .collect();
    (lanes, y)
}

pub fn lane_at(lanes: &[Lane], y: f32) -> Option<Lane> {
    lanes.iter().find(|l| y >= l.y && y < l.y + l.h).copied()
}

pub fn ruler_step(px_per_sec: f64) -> f64 {
    const STEPS: [f64; 14] = [0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
    STEPS.iter().copied().find(|s| s * px_per_sec >= 90.0).unwrap_or(600.0)
}

pub fn ruler_label(t: f64, step: f64) -> String {
    let minutes = (t / 60.0).floor();
    let seconds = t - minutes * 60.0;
    if step >= 1.0 { format!("{}:{:02}", minutes as u64, seconds.round() as u64) } else { format!("{}:{:05.2}", minutes as u64, seconds) }
}

pub fn clamp_zoom(px_per_sec: f64) -> f64 {
    px_per_sec.clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC)
}

pub fn zoom_about(scroll_sec: f64, px_per_sec: f64, anchor_x: f64, next_px: f64) -> (f64, f64) {
    let next = clamp_zoom(next_px);
    let anchor_sec = scroll_sec + anchor_x / px_per_sec;
    (next, (anchor_sec - anchor_x / next).max(0.0))
}

pub fn fit_zoom(stage_width: f64, duration_sec: f64) -> f64 {
    clamp_zoom((stage_width - 40.0) / duration_sec.max(5.0))
}

pub fn edge_hit(clip_width: f32, local_x: f32) -> Option<TrimSide> {
    let edge = EDGE_PX.min(clip_width / 3.0);
    if local_x <= edge {
        Some(TrimSide::Start)
    } else if clip_width - local_x <= edge {
        Some(TrimSide::End)
    } else {
        None
    }
}

pub fn fade_handle_x(clip: &Clip, side: TrimSide, width: f32, px_per_sec: f64) -> f32 {
    let (fade_in, fade_out) = clip_fades(clip);
    let x = match side {
        TrimSide::Start => fade_in as f32 * px_per_sec as f32,
        TrimSide::End => width - fade_out as f32 * px_per_sec as f32,
    };
    x.max(FADE_INSET_PX).min(width - FADE_INSET_PX)
}

pub fn fade_hit(clip: &Clip, width: f32, px_per_sec: f64, local_x: f32, local_y: f32) -> Option<TrimSide> {
    if width < 30.0 || local_y > FADE_ZONE_PX {
        return None;
    }
    let d_in = (local_x - fade_handle_x(clip, TrimSide::Start, width, px_per_sec)).abs();
    let d_out = (local_x - fade_handle_x(clip, TrimSide::End, width, px_per_sec)).abs();
    if d_in.min(d_out) > EDGE_PX {
        return None;
    }
    Some(if d_in <= d_out { TrimSide::Start } else { TrimSide::End })
}

pub fn key_x(clip: &Clip, key: &Keyframe, px_per_sec: f64) -> f32 {
    (local_at(clip, key.source_sec) * px_per_sec) as f32
}

pub fn key_hit(clip: &Clip, keys: &[Keyframe], height: f32, px_per_sec: f64, local_x: f32, local_y: f32) -> Option<Keyframe> {
    if (local_y - height / 2.0).abs() > KEY_R + 2.0 {
        return None;
    }
    keys.iter().find(|k| (key_x(clip, k, px_per_sec) - local_x).abs() <= KEY_R + 2.0).cloned()
}

pub fn follow_scroll(scroll_sec: f64, px_per_sec: f64, stage_width: f64, playhead_sec: f64, playing: bool, moved: bool) -> Option<f64> {
    if stage_width <= 0.0 {
        return None;
    }
    let visible = stage_width / px_per_sec;
    if playing {
        let outside = playhead_sec > scroll_sec + visible * 0.92 || playhead_sec < scroll_sec;
        return outside.then(|| (playhead_sec - visible * 0.1).max(0.0));
    }
    if !moved {
        return None;
    }
    if playhead_sec > scroll_sec + visible * 0.97 {
        Some((playhead_sec - visible * 0.97).max(0.0))
    } else if playhead_sec < scroll_sec {
        Some((playhead_sec - visible * 0.03).max(0.0))
    } else {
        None
    }
}

pub fn trim_limits(project: &Project, clip_ids: &[String], side: TrimSide) -> (f64, f64) {
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for id in clip_ids {
        let Some((track, clip)) = find_clip(project, id) else { continue };
        let asset = project.assets.iter().find(|a| a.id == clip.asset_id);
        let unbounded = clip.text.is_some() || clip.adjustment || clip.freeze_at.is_some() || asset.is_some_and(|a| a.still);
        let source_sec = if unbounded { f64::INFINITY } else { asset.map_or(clip_end(clip), |a| a.duration_sec) };
        let (a, b) = trim_range(track, clip, source_sec, side);
        lo = lo.max(a);
        hi = hi.min(b);
    }
    (lo, hi.max(lo))
}

pub fn scrub_time(sec: f64, snapping: bool, points: &[f64], threshold_sec: f64) -> f64 {
    let t = sec.max(0.0);
    if !snapping {
        return t;
    }
    crate::timeline::find_snap(&[t], points, threshold_sec).map_or(t, |s| s.point)
}

pub fn clip_width(clip: &Clip, px_per_sec: f64) -> f32 {
    ((clip_duration(clip) * px_per_sec) as f32).max(2.0)
}
