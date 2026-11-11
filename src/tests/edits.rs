use super::clip;
use crate::edits::{
    add_track, carry_mix, clips_sharing_edge, duplicate_clips, empty_project, find_clip, group_of, mirror_clip, move_clips, remove_asset, ripple_delete, set_fade, snap_points, split_clips_at,
    trim_clips, update_clips, update_track,
};
use crate::freeze::{freeze_frame_at, resume_from_freeze};
use crate::insert::{AudioTarget, InsertTarget, default_text, insert_asset, insert_text, project_from_media};
use crate::timeline::{TrimSide, clip_end};
use crate::types::{AudioStream, Clip, MediaAsset, MediaKind, Project, SpeedKey, Track, TrackKind};
use std::collections::HashSet;

fn stream(title: &str, codec: &str, channels: u32) -> AudioStream {
    AudioStream { title: title.into(), codec: codec.into(), channels }
}

fn asset(id: &str, name: &str, kind: MediaKind, duration_sec: f64, audio_streams: Vec<AudioStream>) -> MediaAsset {
    MediaAsset {
        id: id.into(),
        name: name.into(),
        source_path: format!("C:/media/{name}"),
        kind,
        duration_sec,
        audio_streams,
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

fn recording() -> MediaAsset {
    let streams = vec![stream("Game", "aac", 2), stream("Mic", "aac", 1), stream("", "opus", 2)];
    MediaAsset { width: Some(1279), height: Some(721), fps: Some(59.94005994), ..asset("v", "session.mkv", MediaKind::Video, 6.0, streams) }
}

fn song() -> MediaAsset {
    asset("s", "song.wav", MediaKind::Audio, 8.0, vec![stream("", "pcm_s16le", 2)])
}

fn named<'a>(p: &'a Project, name: &str) -> &'a Track {
    p.tracks.iter().find(|t| t.name == name).unwrap()
}

fn of_kind(p: &Project, kind: TrackKind) -> &Track {
    p.tracks.iter().find(|t| t.kind == kind).unwrap()
}

fn spans(p: &Project, name: &str) -> Vec<(f64, f64, f64)> {
    named(p, name).clips.iter().map(|c| (c.timeline_start_sec, c.source_in_sec, c.source_out_sec)).collect()
}

fn with_song() -> Project {
    let base = Project { assets: vec![recording(), song()], ..project_from_media(&recording()) };
    insert_asset(&base, &song(), &InsertTarget { video_track_id: None, audio: AudioTarget::New }, 0.0).0
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

#[test]
fn a_video_with_several_streams_lands_as_one_group() {
    let project = project_from_media(&recording());
    assert_eq!(project.name, "session");
    assert_eq!((project.width, project.height, project.fps), (1280, 722, 59.94));
    let layout: Vec<_> = project.tracks.iter().map(|t| (t.kind, t.name.as_str(), t.clips.len())).collect();
    assert_eq!(layout, vec![(TrackKind::Video, "V1", 1), (TrackKind::Audio, "Game", 1), (TrackKind::Audio, "Mic", 1), (TrackKind::Audio, "Track 3", 1)]);
    let clips: Vec<&Clip> = project.tracks.iter().flat_map(|t| &t.clips).collect();
    assert_eq!(clips.iter().map(|c| c.group_id.clone()).collect::<HashSet<_>>().len(), 1);
    assert_eq!(clips.iter().map(|c| c.stream).collect::<Vec<_>>(), vec![None, Some(0), Some(1), Some(2)]);
    assert_eq!(group_of(&project, &clips[2].id).len(), 4);
}

#[test]
fn added_audio_goes_on_a_new_named_track_unlinked() {
    let base = Project { assets: vec![recording(), song()], ..project_from_media(&recording()) };
    let (project, clip_ids) = insert_asset(&base, &song(), &InsertTarget { video_track_id: None, audio: AudioTarget::New }, 1.0);
    assert_eq!(clip_ids.len(), 1);
    let added = named(&project, "song");
    assert_eq!(project.tracks.last().unwrap(), added);
    assert_eq!(added.clips[0].group_id, None);
    assert_eq!(added.clips[0].stream, Some(0));
    assert_eq!(spans(&project, "song"), vec![(1.0, 0.0, 8.0)]);
}

#[test]
fn splitting_everything_gives_each_side_its_own_group() {
    let split = split_clips_at(&project_from_media(&recording()), None, 2.0);
    assert_eq!(spans(&split, "Mic"), vec![(0.0, 0.0, 2.0), (2.0, 2.0, 6.0)]);
    let (left, right) = (&named(&split, "V1").clips[0], &named(&split, "V1").clips[1]);
    assert_ne!(left.group_id, right.group_id);
    let starts: Vec<f64> = group_of(&split, &right.id).iter().map(|id| find_clip(&split, id).unwrap().1.timeline_start_sec).collect();
    assert_eq!(starts, vec![2.0; 4]);
    assert_eq!(group_of(&split, &left.id).len(), 4);
}

#[test]
fn splitting_selected_clips_leaves_others_whole() {
    let project = project_from_media(&recording());
    let mic = named(&project, "Mic").clips[0].id.clone();
    let split = split_clips_at(&project, Some(&[mic]), 3.0);
    assert_eq!(named(&split, "Mic").clips.len(), 2);
    assert_eq!(named(&split, "Game").clips.len(), 1);
}

#[test]
fn moving_a_group_keeps_tracks_and_stays_above_zero() {
    let project = project_from_media(&recording());
    let group = group_of(&project, &named(&project, "V1").clips[0].id);
    let moved = move_clips(&project, &group, 3.0, None);
    assert_eq!(spans(&moved, "V1"), vec![(3.0, 0.0, 6.0)]);
    assert_eq!(spans(&moved, "Track 3"), vec![(3.0, 0.0, 6.0)]);
    assert_eq!(spans(&move_clips(&moved, &group, -10.0, None), "Game"), vec![(0.0, 0.0, 6.0)]);
}

#[test]
fn ripple_delete_closes_the_gap_on_every_track() {
    let rippled = ripple_delete(&with_song(), 1.0, 3.0);
    assert_eq!(spans(&rippled, "V1"), vec![(0.0, 0.0, 1.0), (1.0, 3.0, 6.0)]);
    assert_eq!(spans(&rippled, "song"), vec![(0.0, 0.0, 1.0), (1.0, 3.0, 8.0)]);
    let v = &named(&rippled, "V1").clips;
    assert_ne!(v[0].group_id, v[1].group_id);
}

#[test]
fn group_trims_only_move_lined_up_edges() {
    let base = project_from_media(&recording());
    let mic_track = named(&base, "Mic").id.clone();
    let project = update_track(&base, &mic_track, |t| Track { clips: t.clips.iter().map(|c| Clip { source_out_sec: 4.0, ..c.clone() }).collect(), ..t.clone() });
    let mic = named(&project, "Mic").clips[0].id.clone();
    let video = named(&project, "V1").clips[0].clone();
    let sharing = clips_sharing_edge(&project, &group_of(&project, &video.id), &video, TrimSide::End);
    assert_eq!(sharing.len(), 3);
    assert!(!sharing.contains(&mic));
    let trimmed = trim_clips(&project, &sharing, TrimSide::End, 5.0);
    assert_eq!(spans(&trimmed, "Game"), vec![(0.0, 0.0, 5.0)]);
    assert_eq!(spans(&trimmed, "Mic"), vec![(0.0, 0.0, 4.0)]);
}

#[test]
fn undo_keeps_the_live_mix() {
    let before = project_from_media(&recording());
    let game = named(&before, "Game").id.clone();
    let live = update_track(&before, &game, |t| Track { volume: 0.3, muted: true, ..t.clone() });
    let restored = carry_mix(&live, &before);
    assert_eq!(named(&restored, "Game").volume, 0.3);
    assert!(named(&restored, "Game").muted);
    assert_eq!(named(&restored, "Mic"), named(&before, "Mic"));
}

#[test]
fn snap_points_skip_dragged_clips() {
    let project = project_from_media(&recording());
    let group = group_of(&project, &named(&project, "V1").clips[0].id);
    assert_eq!(snap_points(&project, &group, &[2.5]), vec![0.0, 2.5]);
    assert_eq!(snap_points(&project, &[], &[]).len(), 9);
}

#[test]
fn removing_an_asset_drops_its_clips() {
    let removed = remove_asset(&with_song(), "v");
    assert_eq!(removed.assets.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["s"]);
    assert_eq!(removed.tracks.iter().flat_map(|t| &t.clips).count(), 1);
    assert_eq!(empty_project().tracks.len(), 2);
}

#[test]
fn new_tracks_keep_text_then_video_then_audio() {
    let mut project = empty_project();
    for kind in [TrackKind::Audio, TrackKind::Text, TrackKind::Video] {
        project = add_track(&project, kind, None);
    }
    assert_eq!(project.tracks.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["T1", "V2", "V1", "A1", "A2"]);
}

#[test]
fn text_goes_on_the_first_free_text_track() {
    let (first, _) = insert_text(&empty_project(), 1.0, 3.0, default_text());
    assert_eq!(first.tracks[0].kind, TrackKind::Text);
    let (second, _) = insert_text(&first, 5.0, 2.0, default_text());
    assert_eq!(second.tracks.iter().filter(|t| t.kind == TrackKind::Text).count(), 1);
    let (third, id) = insert_text(&second, 2.0, 2.0, default_text());
    let text_tracks: Vec<&Track> = third.tracks.iter().filter(|t| t.kind == TrackKind::Text).collect();
    assert_eq!(text_tracks.len(), 2);
    assert_eq!(find_clip(&third, &id).unwrap().0.id, text_tracks[0].id);
    assert_eq!(text_tracks[1].clips.len(), 2);
}

#[test]
fn fades_clamp_to_the_clip_length() {
    let (project, id) = insert_text(&empty_project(), 0.0, 3.0, default_text());
    let ids = [id.clone()];
    let faded = set_fade(&set_fade(&project, &ids, TrimSide::Start, 9.0), &ids, TrimSide::End, -1.0);
    let c = find_clip(&faded, &id).unwrap().1;
    assert_eq!((c.fade_in_sec, c.fade_out_sec), (Some(3.0), Some(0.0)));
}

#[test]
fn splitting_keeps_the_outer_fades_only() {
    let (project, id) = insert_text(&empty_project(), 0.0, 4.0, default_text());
    let ids = [id];
    let faded = set_fade(&set_fade(&project, &ids, TrimSide::Start, 1.0), &ids, TrimSide::End, 1.0);
    let split = split_clips_at(&faded, Some(&ids), 2.0);
    let (left, right) = (&split.tracks[0].clips[0], &split.tracks[0].clips[1]);
    assert_eq!((left.fade_in_sec, left.fade_out_sec), (Some(1.0), None));
    assert_eq!((right.fade_in_sec, right.fade_out_sec), (None, Some(1.0)));
    assert_eq!(right.text.as_ref().unwrap().content, "Text");
}

#[test]
fn a_duplicated_group_moves_without_the_original() {
    let project = project_from_media(&recording());
    let group = group_of(&project, &named(&project, "V1").clips[0].id);
    let (dup, map) = duplicate_clips(&project, &group);
    let copies: Vec<String> = group.iter().map(|id| map[id].clone()).collect();
    let moved = move_clips(&dup, &copies, 6.0, None);
    assert_eq!(spans(&moved, "V1"), vec![(0.0, 0.0, 6.0), (6.0, 0.0, 6.0)]);
    assert_eq!(sorted(group_of(&moved, &copies[0])), sorted(copies.clone()));
    assert_eq!(sorted(group_of(&moved, &group[0])), sorted(group.clone()));
}

#[test]
fn mirroring_there_and_back_restores_range_and_speed() {
    let keys = |k: &[(f64, f64)]| Some(k.iter().map(|&(source_sec, speed)| SpeedKey { source_sec, speed }).collect::<Vec<_>>());
    let original = Clip { speed_keys: keys(&[(12.0, 1.0), (18.0, 2.0)]), ..clip("c", 3.0, 12.0, 20.0) };
    let reversed = mirror_clip(&original, "r", 20.0);
    assert_eq!((reversed.asset_id.as_str(), reversed.source_in_sec, reversed.source_out_sec), ("r", 0.0, 8.0));
    assert_eq!(reversed.speed_keys, keys(&[(2.0, 2.0), (8.0, 1.0)]));
    assert_eq!(mirror_clip(&Clip { source_in_sec: 1.0, ..reversed }, "a", 20.0), Clip { source_out_sec: 19.0, ..original });
}

#[test]
fn a_freeze_holds_the_frame_and_resuming_plays_on() {
    let base = project_from_media(&recording());
    let frozen = freeze_frame_at(&base, &of_kind(&base, TrackKind::Video).clips[0].id, 2.0);
    let clips = &of_kind(&frozen, TrackKind::Video).clips;
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].source_out_sec, 2.0);
    assert_eq!(clips[1].freeze_at, Some(2.0));
    assert_eq!(clips[1].timeline_start_sec, 2.0);
    assert_eq!(clip_end(&clips[1]), 6.0);
    assert_eq!(of_kind(&frozen, TrackKind::Audio).clips.len(), 2);

    let resumed = resume_from_freeze(&frozen, &clips[1].id, 3.0);
    let after = &of_kind(&resumed, TrackKind::Video).clips;
    assert_eq!(after.len(), 3);
    assert_eq!(after[1].freeze_at, Some(2.0));
    assert_eq!(clip_end(&after[1]), 3.0);
    assert_eq!(after[2].freeze_at, None);
    assert_eq!((after[2].timeline_start_sec, after[2].source_in_sec, after[2].source_out_sec), (3.0, 2.0, 6.0));
    assert_eq!(clip_end(&after[2]), 7.0);
    let sound = &of_kind(&resumed, TrackKind::Audio).clips;
    assert_eq!((sound[1].timeline_start_sec, sound[1].source_in_sec), (3.0, 2.0));
}

#[test]
fn a_freeze_at_the_end_extends_the_last_frame_without_audio() {
    let base = project_from_media(&recording());
    let video = of_kind(&base, TrackKind::Video).clips[0].clone();
    let frozen = freeze_frame_at(&base, &video.id, clip_end(&video));
    let clips = &of_kind(&frozen, TrackKind::Video).clips;
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[1].timeline_start_sec, clip_end(&video));
    let at = clips[1].freeze_at.unwrap();
    assert!(at < video.source_out_sec && at > video.source_out_sec - 0.1);
    assert_eq!(of_kind(&frozen, TrackKind::Audio).clips.len(), 1);
}

#[test]
fn only_plain_video_clips_freeze() {
    let base = project_from_media(&recording());
    let audio = of_kind(&base, TrackKind::Audio).clips[0].id.clone();
    assert_eq!(freeze_frame_at(&base, &audio, 2.0), base);
}

#[test]
fn ending_an_end_freeze_plays_the_trimmed_off_video_and_audio_on() {
    let base = project_from_media(&recording());
    let ids: Vec<String> = base.tracks.iter().flat_map(|t| &t.clips).map(|c| c.id.clone()).collect();
    let trimmed = update_clips(&base, &ids, |c| Clip { source_out_sec: 4.0, ..c.clone() });
    let video = of_kind(&trimmed, TrackKind::Video).clips[0].clone();
    let frozen = freeze_frame_at(&trimmed, &video.id, clip_end(&video));
    let held = of_kind(&frozen, TrackKind::Video).clips[1].clone();
    assert_eq!(held.group_id, video.group_id);
    let resumed = resume_from_freeze(&frozen, &held.id, 5.0);
    let live = of_kind(&resumed, TrackKind::Video).clips[2].clone();
    assert_eq!((live.timeline_start_sec, live.source_in_sec), (5.0, held.freeze_at.unwrap()));
    let continued: Vec<&Clip> = resumed.tracks.iter().filter(|t| t.kind == TrackKind::Audio).flat_map(|t| t.clips.iter().filter(|c| c.timeline_start_sec == 5.0)).collect();
    assert_eq!(continued.len(), base.tracks.iter().filter(|t| t.kind == TrackKind::Audio).count());
    assert!(continued.iter().all(|c| c.source_in_sec == live.source_in_sec && c.source_out_sec == live.source_out_sec && c.group_id == live.group_id && c.group_id != video.group_id));
}
