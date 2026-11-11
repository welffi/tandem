use super::near;
use crate::export_audio::{Shaper, audio_mapping, audio_tags, edge_gain, speed_commands};
use crate::render_plan::{AudioPiece, Fades, SpeedWarp};
use crate::types::AudioMode;

fn samples(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn pieces_fade_at_their_edges_and_by_the_user_fades() {
    near(edge_gain(0, 1000, 64, 0.0, 0.0), 0.0);
    near(edge_gain(32, 1000, 64, 0.0, 0.0), 0.5);
    near(edge_gain(500, 1000, 64, 0.0, 0.0), 1.0);
    near(edge_gain(999, 1000, 64, 0.0, 0.0), 0.0);
    near(edge_gain(100, 1000, 64, 400.0, 0.0), 0.25);
    near(edge_gain(800, 1000, 64, 0.0, 400.0), 0.5);
}

#[test]
fn shaper_skips_frames_and_keeps_split_samples() {
    let mut shaper = Shaper::new(200, 2, 0.0, 0.0, 0.5);
    let bytes = samples(&[1000; 2 * 100]);
    let mut out = Vec::new();
    shaper.push(&bytes[..7], &mut out);
    shaper.push(&bytes[7..], &mut out);
    assert_eq!(shaper.frame, 100);
    assert_eq!(out.len(), 98 * 4);
    let value = |frame: usize| i16::from_le_bytes([out[(frame - 2) * 4], out[(frame - 2) * 4 + 1]]);
    assert_eq!(value(80), 500);
    assert_eq!(value(2), 16);
}

#[test]
fn louder_than_full_scale_is_clamped() {
    let mut shaper = Shaper::new(400, 0, 0.0, 0.0, 4.0);
    let mut out = Vec::new();
    shaper.push(&samples(&[20000; 2 * 200]), &mut out);
    assert_eq!(i16::from_le_bytes([out[150 * 4], out[150 * 4 + 1]]), 32767);
}

#[test]
fn speed_ramps_become_rubberband_commands() {
    let piece = AudioPiece {
        path: "a.mp4".into(),
        stream: 0,
        in_sec: 1.0,
        duration_sec: 2.0,
        timeline_start_sec: 0.0,
        speed: Some(SpeedWarp { points: vec![(0.0, 1.0), (1.0, 3.0), (1.0, 3.0), (2.0, 3.5)], keep_pitch: true }),
        volume: None,
        fades: Fades::default(),
    };
    assert_eq!(speed_commands(&piece).unwrap(), "0.000000 rubberband tempo 2.00000, rubberband pitch 1;\n2.000000 rubberband tempo 0.50000, rubberband pitch 1;");
    assert_eq!(speed_commands(&AudioPiece { speed: None, ..piece }), None);
}

#[test]
fn audio_maps_mixed_or_separate_with_the_limiter() {
    assert_eq!(audio_mapping(1, AudioMode::Mix, None), ["-map", "0:a", "-c:a", "aac", "-b:a", "320k"]);
    assert_eq!(audio_mapping(2, AudioMode::Mix, Some("lim"))[..2], ["-filter_complex", "[0:a][1:a]amix=inputs=2:normalize=0:duration=longest,lim[mix]"]);
    assert_eq!(audio_mapping(2, AudioMode::Separate, Some("lim"))[..4], ["-filter_complex", "[0:a]lim[a0];[1:a]lim[a1]", "-map", "[a0]"]);
    assert_eq!(audio_mapping(2, AudioMode::Separate, None)[..4], ["-map", "0:a", "-map", "1:a"]);
    let names = vec!["Game".to_string(), "Mic".to_string()];
    assert_eq!(audio_tags(&names, AudioMode::Mix)[1], "title=Mix");
    assert_eq!(audio_tags(&names, AudioMode::Separate)[11], "0");
}
