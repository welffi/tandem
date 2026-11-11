use crate::edits::{EDGE_EPS, add_track, update_track};
use crate::ids::new_id;
use crate::insert::project_from_media;
use crate::timeline::{clip_end, place_clip};
use crate::types::{Clip, MediaAsset, Preset, PresetTrack, Project, Track, TrackKind};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64)
}

pub fn make_preset(project: &Project, clip_ids: &[String], name: &str) -> Option<Preset> {
    let picked: Vec<(&Track, Vec<&Clip>)> =
        project.tracks.iter().map(|t| (t, t.clips.iter().filter(|c| clip_ids.contains(&c.id)).collect::<Vec<_>>())).filter(|(_, clips)| !clips.is_empty()).collect();
    let all: Vec<&Clip> = picked.iter().flat_map(|(_, clips)| clips.iter().copied()).collect();
    let origin = all.iter().map(|c| c.timeline_start_sec).reduce(f64::min)?;
    let end = all.iter().map(|c| clip_end(c)).fold(f64::MIN, f64::max);
    let asset_ids: HashSet<&str> = all.iter().map(|c| c.asset_id.as_str()).filter(|id| !id.is_empty()).collect();
    let tracks = picked
        .iter()
        .map(|(track, clips)| PresetTrack {
            kind: track.kind,
            name: track.name.clone(),
            muted: track.muted,
            hidden: track.hidden,
            volume: track.volume,
            clips: clips.iter().map(|c| Clip { timeline_start_sec: c.timeline_start_sec - origin, ..(*c).clone() }).collect(),
        })
        .collect();
    let trimmed = name.trim();
    Some(Preset {
        id: new_id(),
        name: if trimmed.is_empty() { "Preset".into() } else { trimmed.into() },
        created_at: now_ms(),
        duration_sec: end - origin,
        assets: project.assets.iter().filter(|a| asset_ids.contains(a.id.as_str())).cloned().collect(),
        tracks,
        folder_id: None,
        color: None,
    })
}

fn span(clips: &[Clip], offset: f64) -> (f64, f64) {
    let start = clips.iter().map(|c| c.timeline_start_sec).fold(f64::MAX, f64::min);
    let end = clips.iter().map(clip_end).fold(f64::MIN, f64::max);
    (offset + start, offset + end)
}

fn is_free(track: &Track, (start, end): (f64, f64)) -> bool {
    track.clips.iter().all(|c| clip_end(c) <= start + EDGE_EPS || c.timeline_start_sec >= end - EDGE_EPS)
}

fn same_mix(track: &Track, wanted: &PresetTrack) -> bool {
    track.volume == wanted.volume && track.muted == wanted.muted && track.hidden == wanted.hidden
}

fn add_preset_track(project: &Project, wanted: &PresetTrack, after_id: Option<&str>) -> (Project, String) {
    let before: HashSet<&str> = project.tracks.iter().map(|t| t.id.as_str()).collect();
    let mut next = add_track(project, wanted.kind, Some(&wanted.name));
    let created_at = next.tracks.iter().position(|t| !before.contains(t.id.as_str())).unwrap_or(0);
    let mut track = next.tracks.remove(created_at);
    track.muted = wanted.muted;
    track.hidden = wanted.hidden;
    track.volume = wanted.volume;
    let id = track.id.clone();
    let at = after_id.and_then(|a| next.tracks.iter().position(|t| t.id == a)).map_or(created_at, |i| i + 1);
    next.tracks.insert(at, track);
    (next, id)
}

pub struct Applied {
    pub project: Project,
    pub clip_ids: Vec<String>,
    pub new_assets: Vec<MediaAsset>,
}

pub fn apply_preset(project: &Project, preset: &Preset, start_sec: f64) -> Applied {
    let start = start_sec.max(0.0);
    let mut asset_ids: HashMap<&str, String> = HashMap::new();
    let mut new_assets = Vec::new();
    for asset in &preset.assets {
        let existing = project.assets.iter().find(|a| a.source_path == asset.source_path || Some(&a.source_path) == asset.original_path.as_ref());
        let id = match existing {
            Some(a) => a.id.clone(),
            None => {
                let added = MediaAsset { id: new_id(), ..asset.clone() };
                let id = added.id.clone();
                new_assets.push(added);
                id
            }
        };
        asset_ids.insert(&asset.id, id);
    }

    let mut groups: HashMap<String, String> = HashMap::new();
    let mut next = project.clone();
    next.assets.extend(new_assets.iter().cloned());
    let mut last_of_kind: HashMap<TrackKind, String> = HashMap::new();
    let mut clip_ids = Vec::new();
    for wanted in &preset.tracks {
        if wanted.clips.is_empty() {
            continue;
        }
        let range = span(&wanted.clips, start);
        let last = last_of_kind.get(&wanted.kind).cloned();
        let same_kind: Vec<&Track> = next.tracks.iter().filter(|t| t.kind == wanted.kind).collect();
        let after = last.as_ref().and_then(|l| same_kind.iter().position(|t| &t.id == l)).map_or(0, |i| i + 1);
        let free = same_kind[after..].iter().find(|t| same_mix(t, wanted) && is_free(t, range)).map(|t| t.id.clone());
        let track_id = match free {
            Some(id) => id,
            None => {
                let (added, id) = add_preset_track(&next, wanted, last.as_deref());
                next = added;
                id
            }
        };
        last_of_kind.insert(wanted.kind, track_id.clone());
        for clip in &wanted.clips {
            let placed = Clip {
                id: new_id(),
                asset_id: if clip.asset_id.is_empty() { String::new() } else { asset_ids.get(clip.asset_id.as_str()).cloned().unwrap_or_else(|| clip.asset_id.clone()) },
                timeline_start_sec: clip.timeline_start_sec + start,
                group_id: clip.group_id.as_ref().map(|g| groups.entry(g.clone()).or_insert_with(new_id).clone()),
                ..clip.clone()
            };
            clip_ids.push(placed.id.clone());
            next = update_track(&next, &track_id, |t| Track { clips: place_clip(&t.clips, placed), ..t.clone() });
        }
    }
    Applied { project: next, clip_ids, new_assets }
}

pub fn preset_from_asset(asset: &MediaAsset) -> Option<Preset> {
    let project = project_from_media(asset);
    let ids: Vec<String> = project.tracks.iter().flat_map(|t| t.clips.iter().map(|c| c.id.clone())).collect();
    make_preset(&project, &ids, &asset.name)
}
