mod asset_drop;
mod assets;
mod clip_menu;
mod edits;
mod export;
mod export_options;
mod format;
mod framing;
mod inspector;
mod key_tags;
mod keyframe_panel;
mod keyframes;
mod level_meter;
mod library;
mod media;
mod mix;
mod player;
mod presets;
mod preview_edit;
mod raster;
mod render_plan;
mod silence;
mod speed;
mod store;
mod text;
mod timeline;
mod timeline_drag;
mod timeline_view;

use crate::types::{Clip, Effect, EffectKind, MediaAsset, MediaKind, Track, TrackKind};

pub fn effect(kind: EffectKind) -> Effect {
    Effect {
        id: "e".into(),
        kind,
        enabled: true,
        amount: 1.0,
        size: None,
        height: None,
        x: None,
        y: None,
        rotation: None,
        color: None,
        softness: None,
        spill: None,
        distance: None,
        blur: None,
        angle: None,
        keys: None,
    }
}

pub fn clip(id: &str, start: f64, source_in: f64, source_out: f64) -> Clip {
    Clip { id: id.into(), asset_id: "a".into(), source_in_sec: source_in, source_out_sec: source_out, timeline_start_sec: start, ..Default::default() }
}

pub fn track(kind: TrackKind, clips: Vec<Clip>) -> Track {
    Track { id: "t".into(), kind, name: "V1".into(), clips, muted: false, hidden: false, volume: 1.0 }
}

pub fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-3, "{a} != {b}");
}

pub fn media(id: &str) -> MediaAsset {
    MediaAsset {
        id: id.into(),
        name: id.into(),
        source_path: format!("C:/media/{id}"),
        kind: MediaKind::Video,
        duration_sec: 20.0,
        audio_streams: Vec::new(),
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
