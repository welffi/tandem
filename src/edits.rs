use crate::ids::new_id;
use crate::keyframes::mirror_keys;
use crate::speed::source_at;
use crate::timeline::{TrimSide, apply_trim, clear_range, clip_at, clip_duration, clip_end, place_clip, split_clip};
use crate::types::{Clip, MediaAsset, MediaKind, Project, SpeedKey, Track, TrackKind};
use std::collections::{HashMap, HashSet};

pub const EDGE_EPS: f64 = 0.001;

pub fn blank_project(name: &str) -> Project {
    Project {
        version: 1,
        name: name.into(),
        width: 1920,
        height: 1080,
        fps: 30.0,
        framing: None,
        video_scale: None,
        edge_fill: None,
        edge_blur: None,
        shadow: None,
        shadow_size: None,
        limit_db: None,
        assets: Vec::new(),
        tracks: Vec::new(),
    }
}

pub fn empty_project() -> Project {
    Project { tracks: vec![make_track(TrackKind::Video, "V1"), make_track(TrackKind::Audio, "A1")], ..blank_project("Untitled") }
}

pub fn make_track(kind: TrackKind, name: &str) -> Track {
    Track { id: new_id(), kind, name: name.into(), clips: Vec::new(), muted: false, hidden: false, volume: 1.0 }
}

fn prefix(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Text => "T",
        TrackKind::Video => "V",
        TrackKind::Audio => "A",
    }
}

pub fn add_track(project: &Project, kind: TrackKind, name: Option<&str>) -> Project {
    let count = project.tracks.iter().filter(|t| t.kind == kind).count();
    let track = make_track(kind, &name.map_or_else(|| format!("{}{}", prefix(kind), count + 1), String::from));
    let mut next = project.clone();
    match kind {
        TrackKind::Text => next.tracks.insert(0, track),
        TrackKind::Audio => next.tracks.push(track),
        TrackKind::Video => {
            let at = next.tracks.iter().position(|t| t.kind != TrackKind::Text).unwrap_or(next.tracks.len());
            next.tracks.insert(at, track);
        }
    }
    next
}

pub fn update_clips(project: &Project, clip_ids: &[String], mut change: impl FnMut(&Clip) -> Clip) -> Project {
    let mut next = project.clone();
    for track in &mut next.tracks {
        for clip in &mut track.clips {
            if clip_ids.contains(&clip.id) {
                *clip = change(clip);
            }
        }
    }
    next
}

pub fn update_track(project: &Project, track_id: &str, change: impl FnOnce(&Track) -> Track) -> Project {
    let mut next = project.clone();
    if let Some(track) = next.tracks.iter_mut().find(|t| t.id == track_id) {
        *track = change(track);
    }
    next
}

pub fn place_on_track(project: &Project, track_id: &str, clip: Clip) -> Project {
    update_track(project, track_id, |t| Track { clips: place_clip(&t.clips, clip), ..t.clone() })
}

pub fn mirror_clip(clip: &Clip, asset_id: &str, pivot_sec: f64) -> Clip {
    let mut next = Clip { asset_id: asset_id.into(), source_in_sec: (pivot_sec - clip.source_out_sec).max(0.0), source_out_sec: pivot_sec - clip.source_in_sec, ..clip.clone() };
    if let Some(keys) = &clip.speed_keys {
        let mut flipped: Vec<SpeedKey> = keys.iter().map(|k| SpeedKey { source_sec: pivot_sec - k.source_sec, ..*k }).collect();
        flipped.sort_by(|a, b| a.source_sec.total_cmp(&b.source_sec));
        next.speed_keys = Some(flipped);
    }
    if let Some(effects) = &clip.effects {
        next.effects = Some(effects.iter().map(|e| mirror_keys(e, pivot_sec)).collect());
    }
    next
}

pub fn set_fade(project: &Project, clip_ids: &[String], side: TrimSide, sec: f64) -> Project {
    update_clips(project, clip_ids, |c| {
        let value = (sec.max(0.0).min(clip_duration(c)) * 1000.0).round() / 1000.0;
        match side {
            TrimSide::Start => Clip { fade_in_sec: Some(value), ..c.clone() },
            TrimSide::End => Clip { fade_out_sec: Some(value), ..c.clone() },
        }
    })
}

pub fn retime_clips(project: &Project, clip_ids: &[String], change: impl Fn(&Clip) -> Clip) -> Project {
    let ids: HashSet<String> = clip_ids.iter().flat_map(|id| group_of(project, id)).collect();
    let mut next = project.clone();
    for track in &mut next.tracks {
        let original = track.clips.clone();
        for clip in &mut track.clips {
            if !ids.contains(&clip.id) {
                continue;
            }
            let mut changed = change(clip);
            let limit = original.iter().filter(|o| o.timeline_start_sec >= clip.timeline_start_sec + EDGE_EPS).map(|o| o.timeline_start_sec).fold(f64::INFINITY, f64::min);
            if clip_end(&changed) > limit {
                changed.source_out_sec = source_at(&changed, limit - changed.timeline_start_sec);
            }
            *clip = changed;
        }
    }
    next
}

pub fn remove_track(project: &Project, track_id: &str) -> Project {
    Project { tracks: project.tracks.iter().filter(|t| t.id != track_id).cloned().collect(), ..project.clone() }
}

pub fn base_name(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    match name.rfind('.') {
        Some(dot) if dot + 1 < name.len() => name[..dot].to_string(),
        _ => name.to_string(),
    }
}

pub fn stream_track_name(asset: &MediaAsset, stream: usize) -> String {
    let title = asset.audio_streams.get(stream).map(|s| s.title.trim()).unwrap_or("");
    if !title.is_empty() {
        return title.to_string();
    }
    if asset.kind == MediaKind::Audio && asset.audio_streams.len() == 1 {
        return base_name(&asset.name);
    }
    format!("Track {}", stream + 1)
}

pub fn find_clip<'a>(project: &'a Project, clip_id: &str) -> Option<(&'a Track, &'a Clip)> {
    project.tracks.iter().find_map(|t| t.clips.iter().find(|c| c.id == clip_id).map(|c| (t, c)))
}

pub fn group_of(project: &Project, clip_id: &str) -> Vec<String> {
    let Some((_, clip)) = find_clip(project, clip_id) else {
        return Vec::new();
    };
    let Some(group) = &clip.group_id else {
        return vec![clip_id.to_string()];
    };
    project.tracks.iter().flat_map(|t| &t.clips).filter(|c| c.group_id.as_ref() == Some(group)).map(|c| c.id.clone()).collect()
}

pub fn remove_clips(project: &Project, clip_ids: &[String]) -> Project {
    let mut next = project.clone();
    for track in &mut next.tracks {
        track.clips.retain(|c| !clip_ids.contains(&c.id));
    }
    next
}

pub fn duplicate_clips(project: &Project, clip_ids: &[String]) -> (Project, HashMap<String, String>) {
    let ids: HashMap<String, String> = clip_ids.iter().map(|id| (id.clone(), new_id())).collect();
    let mut groups: HashMap<String, String> = HashMap::new();
    let mut next = project.clone();
    for track in &mut next.tracks {
        let copies: Vec<Clip> = track
            .clips
            .iter()
            .filter(|c| ids.contains_key(&c.id))
            .map(|c| Clip {
                id: ids[&c.id].clone(),
                group_id: c.group_id.as_ref().map(|g| groups.entry(g.clone()).or_insert_with(new_id).clone()),
                effects: c.effects.as_ref().map(|list| list.iter().map(|e| crate::types::Effect { id: new_id(), ..e.clone() }).collect()),
                ..c.clone()
            })
            .collect();
        track.clips.extend(copies);
    }
    (next, ids)
}

pub fn move_clips(project: &Project, clip_ids: &[String], delta_sec: f64, to_track_id: Option<&str>) -> Project {
    let moving: Vec<(Clip, String)> = project.tracks.iter().flat_map(|t| t.clips.iter().filter(|c| clip_ids.contains(&c.id)).map(|c| (c.clone(), t.id.clone()))).collect();
    if moving.is_empty() {
        return project.clone();
    }
    let earliest = moving.iter().map(|(c, _)| c.timeline_start_sec).fold(f64::INFINITY, f64::min);
    let delta = delta_sec.max(-earliest);
    let retarget = if moving.len() == 1 { to_track_id } else { None };
    let mut next = remove_clips(project, clip_ids);
    for (clip, track_id) in moving {
        let moved = Clip { timeline_start_sec: clip.timeline_start_sec + delta, ..clip };
        next = place_on_track(&next, retarget.unwrap_or(&track_id), moved);
    }
    next
}

pub fn trim_clips(project: &Project, clip_ids: &[String], side: TrimSide, time_sec: f64) -> Project {
    update_clips(project, clip_ids, |c| apply_trim(c, side, time_sec))
}

fn edge_of(clip: &Clip, side: TrimSide) -> f64 {
    match side {
        TrimSide::Start => clip.timeline_start_sec,
        TrimSide::End => clip_end(clip),
    }
}

pub fn clips_sharing_edge(project: &Project, clip_ids: &[String], anchor: &Clip, side: TrimSide) -> Vec<String> {
    let edge = edge_of(anchor, side);
    project.tracks.iter().flat_map(|t| &t.clips).filter(|c| c.id == anchor.id || (clip_ids.contains(&c.id) && (edge_of(c, side) - edge).abs() < EDGE_EPS)).map(|c| c.id.clone()).collect()
}

pub fn split_groups_at(project: &Project, time_sec: f64) -> Project {
    let left: HashSet<String> = project.tracks.iter().flat_map(|t| &t.clips).filter(|c| clip_end(c) <= time_sec + EDGE_EPS).filter_map(|c| c.group_id.clone()).collect();
    if left.is_empty() {
        return project.clone();
    }
    let mut renamed: HashMap<String, String> = HashMap::new();
    let mut next = project.clone();
    for clip in next.tracks.iter_mut().flat_map(|t| &mut t.clips) {
        if let Some(group) = clip.group_id.clone()
            && left.contains(&group)
            && clip.timeline_start_sec >= time_sec - EDGE_EPS
        {
            clip.group_id = Some(renamed.entry(group).or_insert_with(new_id).clone());
        }
    }
    next
}

pub fn split_clips_at(project: &Project, clip_ids: Option<&[String]>, time_sec: f64) -> Project {
    let mut next = project.clone();
    for track in &mut next.tracks {
        let target = match clip_ids {
            Some(ids) => track.clips.iter().find(|c| ids.contains(&c.id) && time_sec > c.timeline_start_sec && time_sec < clip_end(c)),
            None => clip_at(track, time_sec),
        };
        let Some(target) = target.cloned() else {
            continue;
        };
        let Some((left, right)) = split_clip(&target, time_sec) else {
            continue;
        };
        let at = track.clips.iter().position(|c| c.id == target.id).unwrap_or(0);
        track.clips.splice(at..=at, [left, right]);
    }
    split_groups_at(&next, time_sec)
}

pub fn ripple_delete(project: &Project, start_sec: f64, end_sec: f64) -> Project {
    let length = end_sec - start_sec;
    if length <= 0.0 {
        return project.clone();
    }
    let mut cleared = project.clone();
    for track in &mut cleared.tracks {
        track.clips = clear_range(&track.clips, start_sec, end_sec);
    }
    let mut next = split_groups_at(&cleared, end_sec);
    for clip in next.tracks.iter_mut().flat_map(|t| &mut t.clips) {
        if clip.timeline_start_sec >= end_sec - EDGE_EPS {
            clip.timeline_start_sec = (clip.timeline_start_sec - length).max(0.0);
        }
    }
    next
}

pub fn clips_span(project: &Project, clip_ids: &[String]) -> Option<(f64, f64)> {
    let clips: Vec<&Clip> = project.tracks.iter().flat_map(|t| &t.clips).filter(|c| clip_ids.contains(&c.id)).collect();
    if clips.is_empty() {
        return None;
    }
    let start = clips.iter().map(|c| c.timeline_start_sec).fold(f64::INFINITY, f64::min);
    let end = clips.iter().map(|c| clip_end(c)).fold(f64::NEG_INFINITY, f64::max);
    Some((start, end))
}

pub fn remove_asset(project: &Project, asset_id: &str) -> Project {
    let mut next = project.clone();
    next.assets.retain(|a| a.id != asset_id);
    for track in &mut next.tracks {
        track.clips.retain(|c| c.asset_id != asset_id);
    }
    next
}

pub fn snap_points(project: &Project, exclude: &[String], extra: &[f64]) -> Vec<f64> {
    let mut points = vec![0.0];
    points.extend_from_slice(extra);
    for clip in project.tracks.iter().flat_map(|t| &t.clips) {
        if !exclude.contains(&clip.id) {
            points.push(clip.timeline_start_sec);
            points.push(clip_end(clip));
        }
    }
    points.sort_by(f64::total_cmp);
    points
}

pub fn carry_mix(from: &Project, to: &Project) -> Project {
    let mut next = to.clone();
    for track in &mut next.tracks {
        if let Some(live) = from.tracks.iter().find(|t| t.id == track.id) {
            track.muted = live.muted;
            track.volume = live.volume;
        }
    }
    next
}
