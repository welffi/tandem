use crate::edits::{EDGE_EPS, add_track, base_name, blank_project, place_on_track, stream_track_name};
use crate::ids::new_id;
use crate::timeline::clip_end;
use crate::types::{Clip, MediaAsset, MediaKind, Project, TextAlign, TextStyle, Track, TrackKind};

const STANDARD_FPS: [f64; 10] = [23.976, 24.0, 25.0, 29.97, 30.0, 48.0, 50.0, 59.94, 60.0, 120.0];

pub fn default_text() -> TextStyle {
    TextStyle {
        content: "Text".into(),
        font: "Segoe UI".into(),
        size: 8.0,
        color: "#ffffff".into(),
        bold: true,
        italic: false,
        align: TextAlign::Center,
        x: 0.5,
        y: 0.5,
        outline: true,
        boxed: false,
        keys: None,
    }
}

fn is_free(track: &Track, start: f64, end: f64) -> bool {
    track.clips.iter().all(|c| clip_end(c) <= start + EDGE_EPS || c.timeline_start_sec >= end - EDGE_EPS)
}

pub fn insert_text(project: &Project, start_sec: f64, duration_sec: f64, style: TextStyle) -> (Project, String) {
    let start = start_sec.max(0.0);
    let end = start + duration_sec;
    let mut next = project.clone();
    let track_id = match project.tracks.iter().find(|t| t.kind == TrackKind::Text && is_free(t, start, end)) {
        Some(track) => track.id.clone(),
        None => {
            next = add_track(&next, TrackKind::Text, None);
            next.tracks[0].id.clone()
        }
    };
    let clip = Clip { id: new_id(), source_out_sec: duration_sec, timeline_start_sec: start, text: Some(style), ..Default::default() };
    let id = clip.id.clone();
    (place_on_track(&next, &track_id, clip), id)
}

pub fn insert_adjustment(project: &Project, start_sec: f64, duration_sec: f64) -> (Project, String) {
    let start = start_sec.max(0.0);
    let end = start + duration_sec;
    let free = project.tracks.iter().find(|t| t.kind == TrackKind::Video).is_some_and(|t| is_free(t, start, end));
    let next = if free { project.clone() } else { add_track(project, TrackKind::Video, None) };
    let track_id = next.tracks.iter().find(|t| t.kind == TrackKind::Video).map(|t| t.id.clone()).unwrap_or_default();
    let clip = Clip { id: new_id(), source_out_sec: duration_sec, timeline_start_sec: start, adjustment: true, effects: Some(Vec::new()), ..Default::default() };
    let id = clip.id.clone();
    (place_on_track(&next, &track_id, clip), id)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AudioTarget {
    #[default]
    First,
    Track(String),
    New,
}

#[derive(Clone, Debug, Default)]
pub struct InsertTarget {
    pub video_track_id: Option<String>,
    pub audio: AudioTarget,
}

pub fn even_size(n: u32) -> u32 {
    ((n as f64 / 2.0).round() as u32 * 2).max(2)
}

pub fn round_fps(fps: f64) -> f64 {
    STANDARD_FPS.iter().copied().fold(STANDARD_FPS[0], |best, f| if (f - fps).abs() < (best - fps).abs() { f } else { best })
}

pub fn insert_asset(project: &Project, asset: &MediaAsset, target: &InsertTarget, start_sec: f64) -> (Project, Vec<String>) {
    let start = start_sec.max(0.0);
    let mut pieces: Vec<(String, Clip)> = Vec::new();
    let mut next = project.clone();
    if asset.kind == MediaKind::Video
        && !asset.still
        && project.tracks.iter().all(|t| t.clips.is_empty())
        && let (Some(w), Some(h)) = (asset.width, asset.height)
        && w > 0
        && h > 0
    {
        next.width = even_size(w);
        next.height = even_size(h);
        next.fps = asset.fps.map_or(project.fps, round_fps);
    }
    let clip_for = |stream: Option<usize>| Clip { id: new_id(), asset_id: asset.id.clone(), stream, source_out_sec: asset.duration_sec, timeline_start_sec: start, ..Default::default() };

    if asset.kind == MediaKind::Video {
        let mut track_id = target.video_track_id.clone().or_else(|| next.tracks.iter().find(|t| t.kind == TrackKind::Video).map(|t| t.id.clone()));
        if track_id.is_none() {
            next = add_track(&next, TrackKind::Video, None);
            track_id = next.tracks.iter().find(|t| t.kind == TrackKind::Video).map(|t| t.id.clone());
        }
        pieces.push((track_id.unwrap_or_default(), clip_for(None)));
    }

    let existing: Vec<String> = next.tracks.iter().filter(|t| t.kind == TrackKind::Audio).map(|t| t.id.clone()).collect();
    let first_index = match &target.audio {
        AudioTarget::New => existing.len(),
        AudioTarget::First => 0,
        AudioTarget::Track(id) => existing.iter().position(|t| t == id).unwrap_or(0),
    };
    for stream in 0..asset.audio_streams.len() {
        let audio: Vec<&Track> = next.tracks.iter().filter(|t| t.kind == TrackKind::Audio).collect();
        let track_id = match audio.get(first_index + stream) {
            Some(track) => track.id.clone(),
            None => {
                next = add_track(&next, TrackKind::Audio, Some(&stream_track_name(asset, stream)));
                next.tracks[next.tracks.len() - 1].id.clone()
            }
        };
        pieces.push((track_id, clip_for(Some(stream))));
    }

    let group = if pieces.len() > 1 { Some(new_id()) } else { None };
    let ids = pieces.iter().map(|(_, c)| c.id.clone()).collect();
    for (track_id, clip) in pieces {
        next = place_on_track(&next, &track_id, Clip { group_id: group.clone(), ..clip });
    }
    (next, ids)
}

pub fn project_from_media(asset: &MediaAsset) -> Project {
    let base = Project { assets: vec![asset.clone()], ..blank_project(&base_name(&asset.name)) };
    insert_asset(&base, asset, &InsertTarget::default(), 0.0).0
}
