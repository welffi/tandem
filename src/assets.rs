use crate::ids::new_id;
use crate::probe::probe;
use crate::types::{MediaAsset, MediaKind, Probe};

pub fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

pub fn asset_from_probe(path: &str, info: Probe) -> Result<MediaAsset, String> {
    if !info.has_video && info.audio_streams.is_empty() {
        return Err("No video or audio found".into());
    }
    Ok(MediaAsset {
        id: new_id(),
        name: file_name(path),
        source_path: path.to_string(),
        kind: if info.has_video { MediaKind::Video } else { MediaKind::Audio },
        duration_sec: info.duration_sec,
        audio_streams: info.audio_streams,
        width: info.width,
        height: info.height,
        fps: info.fps,
        video_codec: info.video_codec,
        still: info.still,
        alpha: info.alpha,
        reverse_of: None,
        original_path: None,
    })
}

pub fn probe_asset(path: &str, known: &[MediaAsset]) -> Result<MediaAsset, String> {
    if let Some(existing) = known.iter().find(|a| a.source_path == path) {
        return Ok(existing.clone());
    }
    asset_from_probe(path, probe(path)?)
}
