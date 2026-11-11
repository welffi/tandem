use super::{clip, track};
use crate::edits::blank_project;
use crate::export_options::{audio_track_count, is_running, resolution_label, status_text, summary};
use crate::types::{AudioMode, ExportPhase, ExportProgress, Project, Track, TrackKind};

fn audio(id: &str, volume: f64, muted: bool, filled: bool) -> Track {
    let clips = if filled { vec![clip(id, 0.0, 0.0, 4.0)] } else { vec![] };
    Track { id: id.into(), volume, muted, ..track(TrackKind::Audio, clips) }
}

fn with_tracks(tracks: Vec<Track>) -> Project {
    Project { tracks, ..blank_project("p") }
}

fn progress(phase: ExportPhase, fraction: f64) -> ExportProgress {
    ExportProgress { phase, fraction }
}

#[test]
fn only_audible_filled_audio_tracks_count() {
    let project = with_tracks(vec![audio("a", 1.0, false, true), audio("b", 1.0, true, true), audio("c", 0.0, false, true), audio("d", 1.0, false, false), audio("e", 0.5, false, true)]);
    assert_eq!(audio_track_count(&project), 2);
}

#[test]
fn the_summary_names_the_audio_that_will_be_written() {
    let two = with_tracks(vec![audio("a", 1.0, false, true), audio("b", 1.0, false, true)]);
    assert!(summary(&two, AudioMode::Mix).ends_with("H.264 · 1 audio track"));
    assert!(summary(&two, AudioMode::Separate).ends_with("H.264 · 2 audio tracks"));
    assert!(summary(&with_tracks(vec![]), AudioMode::Mix).ends_with("H.264 · no audio"));
}

#[test]
fn the_resolution_label_falls_back_to_custom() {
    let hd = Project { width: 1920, height: 1080, ..blank_project("p") };
    assert_eq!(resolution_label(&hd), "1080p (1920×1080)");
    let odd = Project { width: 1000, height: 500, ..blank_project("p") };
    assert_eq!(resolution_label(&odd), "Custom (1000×500)");
}

#[test]
fn status_text_follows_the_phase() {
    assert_eq!(status_text(&progress(ExportPhase::Video, 0.456)), "Rendering video… 46%");
    assert_eq!(status_text(&progress(ExportPhase::Error("no disk".into()), 0.0)), "Export failed: no disk");
    assert_eq!(status_text(&progress(ExportPhase::Cancelled, 0.0)), "Export cancelled.");
    assert!(is_running(&ExportPhase::Mux));
    assert!(!is_running(&ExportPhase::Done));
}
