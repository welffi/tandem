use crate::edits::snap_points;
use crate::insert::{AudioTarget, InsertTarget};
use crate::timeline::find_snap;
use crate::timeline_math::SNAP_PX;
use crate::types::{MediaAsset, MediaKind, Project, TrackKind};

pub struct DropPointer<'a> {
    pub sec: f64,
    pub lane: Option<(&'a str, TrackKind)>,
    pub snapping: bool,
    pub alt: bool,
    pub playhead: f64,
    pub px_per_sec: f64,
}

#[derive(Debug, PartialEq)]
pub struct Piece {
    pub track_id: Option<String>,
    pub kind: TrackKind,
}

pub struct DropPlan {
    pub target: InsertTarget,
    pub start_sec: f64,
    pub duration_sec: f64,
    pub snap_sec: Option<f64>,
    pub pieces: Vec<Piece>,
}

fn pieces_of(project: &Project, asset: &MediaAsset, target: &InsertTarget) -> Vec<Piece> {
    let mut pieces = Vec::new();
    if asset.kind == MediaKind::Video {
        let track_id = target.video_track_id.clone().or_else(|| project.tracks.iter().find(|t| t.kind == TrackKind::Video).map(|t| t.id.clone()));
        pieces.push(Piece { track_id, kind: TrackKind::Video });
    }
    let audio: Vec<&str> = project.tracks.iter().filter(|t| t.kind == TrackKind::Audio).map(|t| t.id.as_str()).collect();
    let first = match &target.audio {
        AudioTarget::Track(id) => audio.iter().position(|t| t == id).unwrap_or(0),
        _ => 0,
    };
    for stream in 0..asset.audio_streams.len() {
        pieces.push(Piece { track_id: audio.get(first + stream).map(|t| t.to_string()), kind: TrackKind::Audio });
    }
    pieces
}

fn target_of(asset: &MediaAsset, lane: Option<(&str, TrackKind)>) -> InsertTarget {
    let mut target = InsertTarget::default();
    match lane {
        Some((id, TrackKind::Video)) if asset.kind == MediaKind::Video => target.video_track_id = Some(id.to_string()),
        Some((id, TrackKind::Audio)) => target.audio = AudioTarget::Track(id.to_string()),
        _ => {}
    }
    target
}

fn snapped_start(project: &Project, asset: &MediaAsset, at: &DropPointer) -> (f64, Option<f64>) {
    let start = at.sec.max(0.0);
    if !at.snapping || at.alt {
        return (start, None);
    }
    let points = snap_points(project, &[], &[at.playhead]);
    match find_snap(&[start, start + asset.duration_sec], &points, SNAP_PX / at.px_per_sec) {
        Some(snap) if start + snap.delta >= 0.0 => (start + snap.delta, Some(snap.point)),
        _ => (start, None),
    }
}

pub fn plan_drop(project: &Project, asset: &MediaAsset, at: &DropPointer) -> DropPlan {
    let target = target_of(asset, at.lane);
    let (start_sec, snap_sec) = snapped_start(project, asset, at);
    let pieces = pieces_of(project, asset, &target);
    DropPlan { target, start_sec, duration_sec: asset.duration_sec, snap_sec, pieces }
}
