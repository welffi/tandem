use crate::edits::empty_project;
use crate::insert::project_from_media;
use crate::presets::{apply_preset, make_preset, preset_from_asset};
use crate::types::{AudioStream, MediaAsset, MediaKind, Preset, Project};
use std::collections::HashSet;

fn recording() -> MediaAsset {
    let stream = |title: &str, channels| AudioStream { title: title.into(), codec: "aac".into(), channels };
    MediaAsset {
        id: "v".into(),
        name: "session.mkv".into(),
        source_path: "C:/media/session.mkv".into(),
        kind: MediaKind::Video,
        duration_sec: 6.0,
        audio_streams: vec![stream("Game", 2), stream("Mic", 1)],
        width: Some(1280),
        height: Some(720),
        fps: Some(30.0),
        video_codec: None,
        still: false,
        alpha: false,
        reverse_of: None,
        original_path: None,
    }
}

fn all_clip_ids(project: &Project) -> Vec<String> {
    project.tracks.iter().flat_map(|t| t.clips.iter().map(|c| c.id.clone())).collect()
}

#[test]
fn a_preset_round_trips_clips_tracks_and_mix() {
    let mut source = project_from_media(&recording());
    source.tracks.iter_mut().filter(|t| t.name.contains("Mic")).for_each(|t| t.volume = 0.4);
    let clip_ids = all_clip_ids(&source);

    let preset = make_preset(&source, &clip_ids, "Intro").unwrap();
    assert_eq!(preset.name, "Intro");
    assert_eq!(preset.duration_sec, 6.0);
    assert_eq!(preset.assets.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["v"]);

    let applied = apply_preset(&empty_project(), &preset, 2.0);
    assert_eq!(applied.new_assets.len(), 1);
    assert_eq!(applied.clip_ids.len(), clip_ids.len());
    let placed: Vec<_> = applied.project.tracks.iter().flat_map(|t| &t.clips).filter(|c| applied.clip_ids.contains(&c.id)).collect();
    assert!(placed.iter().all(|c| c.timeline_start_sec == 2.0 && c.asset_id == applied.new_assets[0].id));
    assert_eq!(placed.iter().map(|c| c.group_id.clone()).collect::<HashSet<_>>().len(), 1);
    assert!(!placed.iter().any(|c| clip_ids.contains(&c.id)));
    assert!(applied.project.tracks.iter().any(|t| t.volume == 0.4 && !t.clips.is_empty()));
}

#[test]
fn applying_reuses_assets_already_in_the_project() {
    let source = project_from_media(&recording());
    let preset = make_preset(&source, &all_clip_ids(&source), "").unwrap();
    assert_eq!(preset.name, "Preset");
    let applied = apply_preset(&source, &preset, 10.0);
    assert!(applied.new_assets.is_empty());
    assert_eq!(applied.project.assets.len(), 1);
}

#[test]
fn nothing_selected_makes_no_preset() {
    assert!(make_preset(&empty_project(), &[], "x").is_none());
}

#[test]
fn a_saved_media_file_adds_all_streams_back() {
    let item = preset_from_asset(&recording()).unwrap();
    assert_eq!(item.name, "session.mkv");
    assert_eq!(item.duration_sec, 6.0);
    let applied = apply_preset(&empty_project(), &item, 3.0);
    assert_eq!(applied.clip_ids.len(), 3);
    assert_eq!(applied.new_assets.len(), 1);
}

#[test]
fn a_library_copy_matches_the_original_file() {
    let source = project_from_media(&recording());
    let item = preset_from_asset(&recording()).unwrap();
    let kept = Preset { assets: item.assets.iter().map(|a| MediaAsset { source_path: "C:/lib/abc.mkv".into(), original_path: Some(a.source_path.clone()), ..a.clone() }).collect(), ..item };
    let applied = apply_preset(&source, &kept, 10.0);
    assert!(applied.new_assets.is_empty());
    assert_eq!(applied.project.assets.len(), 1);
}
