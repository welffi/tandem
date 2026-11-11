use crate::format::format_time;
use crate::timeline::project_duration;
use crate::types::{AudioMode, ExportPhase, ExportProgress, Project, Quality};

pub const RESOLUTIONS: [(&str, u32, u32); 6] = [
    ("4K UHD (3840×2160)", 3840, 2160),
    ("1440p (2560×1440)", 2560, 1440),
    ("1080p (1920×1080)", 1920, 1080),
    ("720p (1280×720)", 1280, 720),
    ("Vertical (1080×1920)", 1080, 1920),
    ("Square (1080×1080)", 1080, 1080),
];

pub const FRAME_RATES: [f64; 8] = [23.976, 24.0, 25.0, 29.97, 30.0, 50.0, 59.94, 60.0];

pub const QUALITIES: [(Quality, &str); 3] = [(Quality::High, "High (slower, best quality)"), (Quality::Medium, "Medium"), (Quality::Draft, "Draft (fast preview)")];

pub fn is_running(phase: &ExportPhase) -> bool {
    matches!(phase, ExportPhase::Audio | ExportPhase::Video | ExportPhase::Mux)
}

pub fn resolution_label(project: &Project) -> String {
    RESOLUTIONS.iter().find(|(_, w, h)| *w == project.width && *h == project.height).map_or_else(|| format!("Custom ({}×{})", project.width, project.height), |(label, ..)| label.to_string())
}

pub fn frame_rate_label(fps: f64) -> String {
    format!("{fps} fps")
}

pub fn audio_track_count(project: &Project) -> usize {
    project.tracks.iter().filter(|t| t.kind == crate::types::TrackKind::Audio && !t.muted && t.volume > 0.0 && !t.clips.is_empty()).count()
}

pub fn summary(project: &Project, mode: AudioMode) -> String {
    let audio = match audio_track_count(project) {
        0 => "no audio".to_string(),
        1 => "1 audio track".to_string(),
        _ if mode == AudioMode::Mix => "1 audio track".to_string(),
        n => format!("{n} audio tracks"),
    };
    format!("{} · H.264 · {audio}", format_time(project_duration(project), Some(project.fps)))
}

pub fn status_text(progress: &ExportProgress) -> String {
    match &progress.phase {
        ExportPhase::Audio => "Mixing audio…".into(),
        ExportPhase::Video => format!("Rendering video… {}%", (progress.fraction * 100.0).round()),
        ExportPhase::Mux => "Finishing…".into(),
        ExportPhase::Done => "Export complete.".into(),
        ExportPhase::Cancelled => "Export cancelled.".into(),
        ExportPhase::Error(message) => format!("Export failed: {message}"),
    }
}
