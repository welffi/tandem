use crate::ids::new_id;
use crate::speed::{local_at, source_at};
use crate::types::{Clip, Project, Track};

pub const MIN_CLIP_SEC: f64 = 0.02;
const SLIVER_SEC: f64 = 0.001;

pub fn clip_duration(clip: &Clip) -> f64 {
    local_at(clip, clip.source_out_sec)
}

pub fn clip_end(clip: &Clip) -> f64 {
    clip.timeline_start_sec + clip_duration(clip)
}

pub fn clip_fades(clip: &Clip) -> (f64, f64) {
    let duration = clip_duration(clip);
    let fade_in = clip.fade_in_sec.unwrap_or(0.0).max(0.0).min(duration);
    let fade_out = clip.fade_out_sec.unwrap_or(0.0).max(0.0).min(duration);
    (fade_in, fade_out)
}

pub fn fade_gain(clip: &Clip, timeline_sec: f64) -> f64 {
    let (fade_in, fade_out) = clip_fades(clip);
    let local = timeline_sec - clip.timeline_start_sec;
    let mut gain: f64 = 1.0;
    if fade_in > 0.0 {
        gain = gain.min(local / fade_in);
    }
    if fade_out > 0.0 {
        gain = gain.min((clip_duration(clip) - local) / fade_out);
    }
    gain.clamp(0.0, 1.0)
}

pub fn sort_clips(clips: &mut [Clip]) {
    clips.sort_by(|a, b| a.timeline_start_sec.total_cmp(&b.timeline_start_sec));
}

pub fn clip_at(track: &Track, time_sec: f64) -> Option<&Clip> {
    track.clips.iter().find(|c| time_sec >= c.timeline_start_sec && time_sec < clip_end(c))
}

pub fn next_clip_after(track: &Track, time_sec: f64) -> Option<&Clip> {
    track.clips.iter().filter(|c| c.timeline_start_sec > time_sec).min_by(|a, b| a.timeline_start_sec.total_cmp(&b.timeline_start_sec))
}

pub fn project_duration(project: &Project) -> f64 {
    project.tracks.iter().flat_map(|t| &t.clips).map(clip_end).fold(0.0, f64::max)
}

fn nearest_index(sorted: &[f64], value: f64) -> usize {
    let mut lo = 0;
    let mut hi = sorted.len() - 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sorted[mid] < value {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo > 0 && (sorted[lo - 1] - value).abs() <= (sorted[lo] - value).abs() {
        return lo - 1;
    }
    lo
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snap {
    pub delta: f64,
    pub point: f64,
}

pub fn find_snap(edges: &[f64], sorted_points: &[f64], threshold_sec: f64) -> Option<Snap> {
    if sorted_points.is_empty() {
        return None;
    }
    let mut best: Option<Snap> = None;
    for &edge in edges {
        let point = sorted_points[nearest_index(sorted_points, edge)];
        let delta = point - edge;
        if delta.abs() <= threshold_sec && best.is_none_or(|b| delta.abs() < b.delta.abs()) {
            best = Some(Snap { delta, point });
        }
    }
    best
}

pub fn split_clip(clip: &Clip, at_sec: f64) -> Option<(Clip, Clip)> {
    if at_sec <= clip.timeline_start_sec + MIN_CLIP_SEC || at_sec >= clip_end(clip) - MIN_CLIP_SEC {
        return None;
    }
    let split_source = source_at(clip, at_sec - clip.timeline_start_sec);
    let left = Clip { source_out_sec: split_source, fade_out_sec: None, ..clip.clone() };
    let right = Clip { id: new_id(), source_in_sec: split_source, timeline_start_sec: at_sec, fade_in_sec: None, ..clip.clone() };
    Some((left, right))
}

pub fn split_clip_at_times(clip: &Clip, times: &[f64]) -> Vec<Clip> {
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mut pieces = Vec::new();
    let mut rest = clip.clone();
    for t in sorted {
        if let Some((left, right)) = split_clip(&rest, t) {
            pieces.push(left);
            rest = right;
        }
    }
    pieces.push(rest);
    pieces
}

pub fn clear_range(clips: &[Clip], start: f64, end: f64) -> Vec<Clip> {
    let mut out = Vec::new();
    for c in clips {
        let c_start = c.timeline_start_sec;
        let c_end = clip_end(c);
        if c_end <= start || c_start >= end {
            out.push(c.clone());
            continue;
        }
        if c_start < start {
            out.push(Clip { source_out_sec: source_at(c, start - c_start), fade_out_sec: None, ..c.clone() });
        }
        if c_end > end {
            let id = if c_start < start { new_id() } else { c.id.clone() };
            out.push(Clip { id, source_in_sec: source_at(c, end - c_start), timeline_start_sec: end, fade_in_sec: None, ..c.clone() });
        }
    }
    out.retain(|c| clip_duration(c) > SLIVER_SEC);
    out
}

pub fn place_clip(clips: &[Clip], clip: Clip) -> Vec<Clip> {
    let others: Vec<Clip> = clips.iter().filter(|c| c.id != clip.id).cloned().collect();
    let mut placed = clear_range(&others, clip.timeline_start_sec, clip_end(&clip));
    placed.push(clip);
    sort_clips(&mut placed);
    placed
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimSide {
    Start,
    End,
}

pub fn trim_range(track: &Track, clip: &Clip, asset_duration_sec: f64, side: TrimSide) -> (f64, f64) {
    let others = track.clips.iter().filter(|c| c.id != clip.id);
    match side {
        TrimSide::Start => {
            let free = clip.text.is_some() || clip.adjustment || clip.freeze_at.is_some();
            let mut min = if free { 0.0 } else { (clip.timeline_start_sec + local_at(clip, 0.0)).max(0.0) };
            for c in others {
                if clip_end(c) <= clip.timeline_start_sec + SLIVER_SEC {
                    min = min.max(clip_end(c));
                }
            }
            (min, clip_end(clip) - MIN_CLIP_SEC)
        }
        TrimSide::End => {
            let mut max = clip.timeline_start_sec + local_at(clip, asset_duration_sec);
            for c in others {
                if c.timeline_start_sec >= clip_end(clip) - SLIVER_SEC {
                    max = max.min(c.timeline_start_sec);
                }
            }
            (clip.timeline_start_sec + MIN_CLIP_SEC, max)
        }
    }
}

pub fn apply_trim(clip: &Clip, side: TrimSide, time_sec: f64) -> Clip {
    let source = source_at(clip, time_sec - clip.timeline_start_sec);
    match side {
        TrimSide::Start => Clip { source_in_sec: source, timeline_start_sec: time_sec, ..clip.clone() },
        TrimSide::End => Clip { source_out_sec: source, ..clip.clone() },
    }
}
