use crate::assets::{asset_from_probe, file_name, probe_asset};
use crate::types::{AudioStream, MediaKind, Probe};

fn stream() -> AudioStream {
    AudioStream { title: "Game".into(), codec: "aac".into(), channels: 2 }
}

#[test]
fn probe_picks_the_kind() {
    let video = asset_from_probe(r"C:\clips\trip.mp4", Probe { has_video: true, duration_sec: 3.0, ..Default::default() }).unwrap();
    assert_eq!(video.kind, MediaKind::Video);
    assert_eq!(video.name, "trip.mp4");
    let audio = asset_from_probe("song.mp3", Probe { audio_streams: vec![stream()], ..Default::default() }).unwrap();
    assert_eq!(audio.kind, MediaKind::Audio);
}

#[test]
fn nothing_playable_is_an_error() {
    assert!(asset_from_probe("notes.mp4", Probe::default()).is_err());
}

#[test]
fn known_paths_reuse_the_asset() {
    let known = asset_from_probe("a.mp3", Probe { audio_streams: vec![stream()], ..Default::default() }).unwrap();
    assert_eq!(probe_asset("a.mp3", std::slice::from_ref(&known)).unwrap().id, known.id);
    assert_eq!(file_name("C:/x/y.wav"), "y.wav");
}
