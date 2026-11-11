use super::{clip, near, track};
use crate::asset_drop::{DropPointer, Piece, plan_drop};
use crate::edits::blank_project;
use crate::insert::AudioTarget;
use crate::types::{AudioStream, MediaAsset, MediaKind, Project, Track, TrackKind};

fn lane(id: &str, kind: TrackKind) -> Track {
    Track { id: id.into(), ..track(kind, vec![]) }
}

fn project() -> Project {
    Project { tracks: vec![lane("v1", TrackKind::Video), lane("a1", TrackKind::Audio), lane("a2", TrackKind::Audio)], ..blank_project("p") }
}

fn video(streams: usize) -> MediaAsset {
    MediaAsset {
        id: "m".into(),
        name: "m.mp4".into(),
        source_path: "C:/m.mp4".into(),
        kind: MediaKind::Video,
        duration_sec: 2.0,
        audio_streams: (0..streams).map(|_| AudioStream { title: String::new(), codec: "aac".into(), channels: 2 }).collect(),
        width: None,
        height: None,
        fps: None,
        video_codec: None,
        still: false,
        alpha: false,
        reverse_of: None,
        original_path: None,
    }
}

fn pointer(sec: f64, lane: Option<(&'static str, TrackKind)>) -> DropPointer<'static> {
    DropPointer { sec, lane, snapping: false, alt: false, playhead: 10.0, px_per_sec: 100.0 }
}

#[test]
fn dropping_on_a_video_lane_targets_that_video_track() {
    let plan = plan_drop(&project(), &video(1), &pointer(1.0, Some(("v1", TrackKind::Video))));
    assert_eq!(plan.target.video_track_id.as_deref(), Some("v1"));
    assert_eq!(plan.pieces, vec![Piece { track_id: Some("v1".into()), kind: TrackKind::Video }, Piece { track_id: Some("a1".into()), kind: TrackKind::Audio }]);
}

#[test]
fn dropping_on_an_audio_lane_starts_the_audio_there() {
    let plan = plan_drop(&project(), &video(2), &pointer(1.0, Some(("a2", TrackKind::Audio))));
    assert_eq!(plan.target.audio, AudioTarget::Track("a2".into()));
    assert_eq!(plan.pieces[1].track_id.as_deref(), Some("a2"));
    assert_eq!(plan.pieces[2].track_id, None);
}

#[test]
fn an_audio_only_file_ignores_video_lanes() {
    let song = MediaAsset { kind: MediaKind::Audio, ..video(1) };
    let plan = plan_drop(&project(), &song, &pointer(1.0, Some(("v1", TrackKind::Video))));
    assert_eq!(plan.target.video_track_id, None);
    assert_eq!(plan.pieces.len(), 1);
}

#[test]
fn the_drop_never_starts_before_zero() {
    near(plan_drop(&project(), &video(0), &pointer(-3.0, None)).start_sec, 0.0);
}

#[test]
fn snapping_pulls_the_start_or_end_to_a_nearby_edge() {
    let busy = Project { tracks: vec![Track { clips: vec![clip("c", 0.0, 0.0, 5.0)], ..lane("v1", TrackKind::Video) }], ..blank_project("p") };
    let mut at = pointer(5.02, None);
    at.snapping = true;
    let plan = plan_drop(&busy, &video(0), &at);
    near(plan.start_sec, 5.0);
    assert_eq!(plan.snap_sec, Some(5.0));
    at.alt = true;
    let free = plan_drop(&busy, &video(0), &at);
    near(free.start_sec, 5.02);
    assert_eq!(free.snap_sec, None);
}

#[test]
fn asset_meta_lists_size_and_audio_notes() {
    let plain = MediaAsset { width: Some(1920), height: Some(1080), ..video(2) };
    assert_eq!(crate::media_bin::asset_meta(&plain), "2.0s · 1920×1080 · 2 audio tracks");
    assert_eq!(crate::media_bin::asset_meta(&video(0)), "2.0s · no audio");
    let photo = MediaAsset { still: true, ..video(0) };
    assert_eq!(crate::media_bin::asset_meta(&photo), "Image");
}
