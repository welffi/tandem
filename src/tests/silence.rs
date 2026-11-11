use super::{clip, track};
use crate::edits::{blank_project, find_clip};
use crate::formats::db_to_gain;
use crate::silence::{SILENCE_DB, sound_range, trim_silence};
use crate::types::{Clip, Peaks, Project, TrackKind};

const RATE: usize = 10;

fn peaks_with(seconds: usize, loud: &[(usize, usize)], level: f32) -> Peaks {
    let mut peaks = vec![0.0; seconds * RATE * 2];
    for &(from, to) in loud {
        for b in from * RATE..to * RATE {
            peaks[2 * b] = -level;
            peaks[2 * b + 1] = level;
        }
    }
    Peaks { peaks, peaks_rate: (RATE * 2) as f64 }
}

fn audio_clip() -> Clip {
    clip("c", 5.0, 0.0, 10.0)
}

fn project() -> Project {
    Project { tracks: vec![track(TrackKind::Audio, vec![audio_clip()])], ..blank_project("p") }
}

#[test]
fn sound_range_finds_first_and_last_audible_bucket() {
    assert_eq!(sound_range(&audio_clip(), &peaks_with(10, &[(2, 3), (6, 7)], 0.5), db_to_gain(SILENCE_DB)), Some((2.0, 7.0)));
}

#[test]
fn sound_range_ignores_sound_outside_and_silence() {
    let floor = db_to_gain(SILENCE_DB);
    let inner = Clip { source_in_sec: 3.0, source_out_sec: 6.0, ..audio_clip() };
    assert_eq!(sound_range(&inner, &peaks_with(10, &[(2, 3), (7, 8)], 0.5), floor), None);
    assert_eq!(sound_range(&audio_clip(), &peaks_with(10, &[(2, 3)], 0.0005), floor), None);
}

#[test]
fn trim_silence_cuts_both_ends_and_moves_the_start() {
    let (out, trimmed) = trim_silence(&project(), &["c".into()], |_| Some(peaks_with(10, &[(2, 3), (6, 7)], 0.5)));
    assert_eq!(trimmed, 1);
    let next = find_clip(&out, "c").unwrap().1;
    assert_eq!((next.source_in_sec, next.source_out_sec, next.timeline_start_sec), (2.0, 7.0, 7.0));
}

#[test]
fn trim_silence_leaves_tight_silent_and_unknown_clips() {
    let p = project();
    let ids = ["c".to_string()];
    assert_eq!(trim_silence(&p, &ids, |_| Some(peaks_with(10, &[(0, 10)], 0.5))).1, 0);
    assert_eq!(trim_silence(&p, &ids, |_| Some(peaks_with(10, &[], 0.5))).1, 0);
    assert_eq!(trim_silence(&p, &ids, |_| None).1, 0);
}
