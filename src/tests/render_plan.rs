use super::effect;
use crate::edits::blank_project;
use crate::render_plan::{Fades, Overlay, RenderPlan, VideoSegment, build_render_plan};
use crate::types::{AnimField, AudioStream, Clip, EffectKind, Keyframe, Keys, MediaAsset, MediaKind, Project, SpeedKey, Track, TrackKind};

fn asset(id: &str, kind: MediaKind, duration_sec: f64) -> MediaAsset {
    let stream = |title: &str, channels| AudioStream { title: title.into(), codec: "aac".into(), channels };
    MediaAsset {
        id: id.into(),
        name: id.into(),
        source_path: format!("C:/media/{id}"),
        kind,
        duration_sec,
        audio_streams: vec![stream("", 2), stream("Mic", 1)],
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

fn image(id: &str, alpha: bool) -> MediaAsset {
    MediaAsset { audio_streams: Vec::new(), still: true, alpha, ..asset(id, MediaKind::Video, 5.0) }
}

fn clip(id: &str, asset_id: &str, start: f64, source_in: f64, source_out: f64) -> Clip {
    Clip { id: id.into(), asset_id: asset_id.into(), source_in_sec: source_in, source_out_sec: source_out, timeline_start_sec: start, ..Default::default() }
}

fn on_stream(c: Clip, stream: usize) -> Clip {
    Clip { stream: Some(stream), ..c }
}

fn track(id: &str, kind: TrackKind, clips: Vec<Clip>) -> Track {
    Track { id: id.into(), kind, name: id.into(), clips, muted: false, hidden: false, volume: 1.0 }
}

fn project(tracks: Vec<Track>) -> Project {
    Project { assets: vec![asset("top", MediaKind::Video, 10.0), asset("base", MediaKind::Video, 20.0), asset("song", MediaKind::Audio, 60.0)], tracks, ..blank_project("p") }
}

fn with_assets(mut p: Project, extra: Vec<MediaAsset>) -> Project {
    p.assets.extend(extra);
    p
}

fn path(s: &VideoSegment) -> Option<&str> {
    s.source.as_ref().map(|v| v.path.as_str())
}

fn segment_at(plan: &RenderPlan, sec: f64) -> &VideoSegment {
    let f = (sec * plan.fps) as i64;
    plan.video.iter().find(|s| s.start_frame <= f && f < s.start_frame + s.frames).unwrap()
}

#[test]
fn top_video_wins_and_gaps_render_black() {
    let plan = build_render_plan(&project(vec![
        track("V2", TrackKind::Video, vec![clip("t", "top", 2.0, 0.0, 1.0)]),
        track("V1", TrackKind::Video, vec![clip("b", "base", 0.0, 5.0, 9.0), clip("b2", "base", 5.0, 0.0, 1.0)]),
    ]));
    assert_eq!(plan.total_frames, 180);
    let got: Vec<_> = plan.video.iter().map(|s| (s.start_frame, s.frames, path(s), s.source.as_ref().map(|v| v.in_sec))).collect();
    assert_eq!(
        got,
        vec![
            (0, 60, Some("C:/media/base"), Some(5.0)),
            (60, 30, Some("C:/media/top"), Some(0.0)),
            (90, 30, Some("C:/media/base"), Some(8.0)),
            (120, 30, None, None),
            (150, 30, Some("C:/media/base"), Some(0.0)),
        ]
    );
}

#[test]
fn hidden_drops_video_and_muted_or_silent_drops_audio() {
    let plan = build_render_plan(&project(vec![
        Track { hidden: true, ..track("V1", TrackKind::Video, vec![clip("b", "base", 0.0, 0.0, 2.0)]) },
        Track { volume: 0.5, ..track("Game", TrackKind::Audio, vec![clip("s", "song", 1.0, 10.0, 13.0)]) },
        Track { muted: true, ..track("Muted", TrackKind::Audio, vec![clip("s2", "song", 0.0, 0.0, 1.0)]) },
        Track { volume: 0.0, ..track("Silent", TrackKind::Audio, vec![clip("s3", "song", 0.0, 0.0, 1.0)]) },
    ]));
    assert_eq!(plan.video, vec![VideoSegment { start_frame: 0, frames: 120, source: None, overlays: Vec::new() }]);
    assert_eq!(plan.audio.len(), 1);
    assert_eq!((plan.audio[0].name.as_str(), plan.audio[0].volume), ("Game", 0.5));
    let piece = &plan.audio[0].pieces[0];
    assert_eq!((piece.path.as_str(), piece.stream, piece.in_sec, piece.duration_sec, piece.timeline_start_sec), ("C:/media/song", 0, 10.0, 3.0, 1.0));
    assert_eq!((piece.fades, piece.speed.is_none(), piece.volume), (Fades::default(), true, None));
}

#[test]
fn each_audio_track_keeps_its_stream_and_skips_missing_ones() {
    let plan = build_render_plan(&project(vec![
        track("V1", TrackKind::Video, vec![clip("v", "base", 0.0, 0.0, 4.0)]),
        track("Mic", TrackKind::Audio, vec![on_stream(clip("late", "base", 3.0, 3.0, 4.0), 1), on_stream(clip("early", "base", 0.0, 0.0, 2.0), 1)]),
        track("Bad", TrackKind::Audio, vec![on_stream(clip("x", "base", 0.0, 0.0, 4.0), 5)]),
    ]));
    assert_eq!(plan.audio.len(), 1);
    let got: Vec<_> = plan.audio[0].pieces.iter().map(|p| (p.stream, p.timeline_start_sec, p.in_sec)).collect();
    assert_eq!(got, vec![(1, 0.0, 0.0), (1, 3.0, 3.0)]);
}

#[test]
fn images_render_as_stills_past_their_length_and_ignore_speed() {
    let sped = Clip { speed_keys: Some(vec![SpeedKey { source_sec: 0.0, speed: 2.0 }]), ..clip("i", "img", 0.0, 0.0, 20.0) };
    let p = Project { assets: vec![image("img", false)], ..project(vec![track("v", TrackKind::Video, vec![sped])]) };
    let plan = build_render_plan(&p);
    assert_eq!(plan.total_frames, 300);
    let source = plan.video[0].source.as_ref().unwrap();
    assert!(source.still);
    assert!(source.speed_clip.is_none());
}

#[test]
fn transparent_images_layer_over_the_video_below() {
    let p = with_assets(
        project(vec![track("V2", TrackKind::Video, vec![clip("logo", "png", 1.0, 0.0, 2.0)]), track("V1", TrackKind::Video, vec![clip("b", "base", 0.0, 0.0, 4.0)])]),
        vec![image("png", true)],
    );
    let plan = build_render_plan(&p);
    let offsets = |s: &VideoSegment| s.overlays.iter().map(|o| if let Overlay::Still(o) = o { o.offset_frames } else { -1 }).collect::<Vec<_>>();
    let got: Vec<_> = plan.video.iter().map(|s| (s.start_frame, s.frames, path(s), offsets(s))).collect();
    let base = Some("C:/media/base");
    assert_eq!(got, vec![(0, 30, base, vec![]), (30, 60, base, vec![0]), (90, 30, base, vec![])]);
}

#[test]
fn keyed_video_layers_over_and_keeps_its_source_time() {
    let keyed = Clip { effects: Some(vec![crate::types::Effect { amount: 0.3, ..effect(EffectKind::ChromaKey) }]), ..clip("k", "green", 1.0, 5.0, 7.0) };
    let p = with_assets(project(vec![track("V2", TrackKind::Video, vec![keyed]), track("V1", TrackKind::Video, vec![clip("b", "base", 0.0, 0.0, 4.0)])]), vec![asset("green", MediaKind::Video, 10.0)]);
    let plan = build_render_plan(&p);
    let mid = &plan.video[1];
    assert_eq!(path(mid), Some("C:/media/base"));
    let Overlay::Keyed(overlay) = &mid.overlays[0] else { panic!("not keyed") };
    assert_eq!((overlay.path.as_str(), overlay.in_sec), ("C:/media/green", 5.0));
    assert_eq!(plan.video.len(), 3);
}

#[test]
fn fade_color_only_passes_a_plain_hex_color() {
    let colored = |color: &str| {
        let c = Clip { fade_color: Some(color.into()), ..clip("b", "base", 0.0, 0.0, 2.0) };
        build_render_plan(&project(vec![track("V1", TrackKind::Video, vec![c])])).video[0].source.as_ref().unwrap().fade_color.clone()
    };
    assert_eq!(colored("#ffffff"), Some("#ffffff".into()));
    assert_eq!(colored("white,drawbox"), None);
}

#[test]
fn cross_dissolve_lays_the_tail_over_the_early_incoming_clip() {
    let a = Clip { fade_out_sec: Some(1.0), dissolve: true, ..clip("a", "base", 0.0, 2.0, 6.0) };
    let b = Clip { fade_in_sec: Some(0.5), dissolve: true, ..clip("b", "top", 4.0, 3.0, 8.0) };
    let plan = build_render_plan(&project(vec![track("V1", TrackKind::Video, vec![a, b])]));
    assert_eq!(path(segment_at(&plan, 2.0)), Some("C:/media/base"));
    assert!(segment_at(&plan, 2.0).overlays.is_empty());
    let mid = segment_at(&plan, 3.5);
    let source = mid.source.as_ref().unwrap();
    assert_eq!(source.path, "C:/media/top");
    assert!((source.in_sec - 2.0).abs() < 1e-6);
    assert_eq!(source.fades.in_sec, 0.0);
    assert_eq!(mid.overlays.len(), 1);
    let Overlay::Blend(tail) = &mid.overlays[0] else { panic!("not a blend") };
    assert_eq!(tail.fades.out_sec, 1.5);
    assert_eq!(segment_at(&plan, 4.25).overlays.len(), 1);
    assert!(segment_at(&plan, 5.0).overlays.is_empty());
}

#[test]
fn without_dissolve_fades_go_to_color() {
    let plan = build_render_plan(&project(vec![track(
        "V1",
        TrackKind::Video,
        vec![Clip { fade_out_sec: Some(1.0), ..clip("a", "base", 0.0, 2.0, 6.0) }, Clip { fade_in_sec: Some(0.5), ..clip("b", "top", 4.0, 3.0, 8.0) }],
    )]));
    assert!(plan.video.iter().all(|s| s.overlays.is_empty()));
    assert_eq!(plan.video.len(), 2);
}

#[test]
fn adjustment_layers_sit_over_and_animated_clips_carry_keys() {
    let adjust = Clip { adjustment: true, effects: Some(vec![effect(EffectKind::Grayscale)]), ..clip("adj", "", 1.0, 0.0, 2.0) };
    let key = |id: &str, source_sec, value| Keyframe { id: id.into(), source_sec, value, ease: None };
    let moving = crate::types::Effect { keys: Some(Keys::from([(AnimField::X, vec![key("k", 0.0, 0.0), key("k2", 2.0, 0.5)])])), ..effect(EffectKind::Transform) };
    let png = Clip { effects: Some(vec![moving]), ..clip("logo", "png", 0.0, 0.0, 4.0) };
    let p = with_assets(
        project(vec![track("V3", TrackKind::Video, vec![adjust]), track("V2", TrackKind::Video, vec![png]), track("V1", TrackKind::Video, vec![clip("b", "base", 0.0, 0.0, 4.0)])]),
        vec![image("png", true)],
    );
    let plan = build_render_plan(&p);
    let mid = plan.video.iter().find(|s| s.start_frame == 30).unwrap();
    assert_eq!(path(mid), Some("C:/media/base"));
    let kinds: Vec<_> = mid.overlays.iter().map(|o| if matches!(o, Overlay::Adjust(_)) { "adjust" } else { "still" }).collect();
    assert_eq!(kinds, vec!["still", "adjust"]);
    let Overlay::Still(still) = &mid.overlays[0] else { panic!("not a still") };
    assert_eq!(still.anim_clip.as_ref().map(|c| c.id.as_str()), Some("logo"));
}

#[test]
fn a_plain_image_lets_video_show_and_backs_the_frame_alone() {
    let p = with_assets(
        project(vec![track("V2", TrackKind::Video, vec![clip("pic", "jpg", 0.0, 0.0, 4.0)]), track("V1", TrackKind::Video, vec![clip("b", "base", 2.0, 0.0, 2.0)])]),
        vec![image("jpg", false)],
    );
    let got: Vec<_> = build_render_plan(&p).video.iter().map(|s| (s.start_frame, path(s).map(String::from), s.overlays.len())).collect();
    assert_eq!(got, vec![(0, Some("C:/media/jpg".into()), 0), (60, Some("C:/media/base".into()), 1)]);
}
