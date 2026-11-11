use super::{clip, near, track};
use crate::edits::blank_project;
use crate::timeline::TrimSide;
use crate::timeline_draw::Geo;
use crate::timeline_hit::{Hit, hit_test};
use crate::timeline_math::{edge_hit, follow_scroll, lane_at, layout_lanes, ruler_label, ruler_step, scrub_time, zoom_about};
use crate::timeline_state::{loudest, track_loudest};
use crate::track_headers::peak_note;
use crate::types::{Peaks, Project, TrackKind};
use eframe::egui::{Rect, pos2};

fn project_of(clips: Vec<crate::types::Clip>) -> Project {
    Project { tracks: vec![track(TrackKind::Video, clips)], ..blank_project("p") }
}

fn geo() -> Geo {
    Geo { stage: Rect::from_min_max(pos2(170.0, 26.0), pos2(1000.0, 500.0)), v_scroll: 0.0, scroll_sec: 0.0, px: 100.0 }
}

#[test]
fn ruler_step_widens_as_you_zoom_out() {
    near(ruler_step(80.0), 2.0);
    near(ruler_step(2000.0), 0.05);
    near(ruler_step(2.0), 60.0);
}

#[test]
fn ruler_labels_show_fractions_only_for_small_steps() {
    assert_eq!(ruler_label(65.0, 1.0), "1:05");
    assert_eq!(ruler_label(65.5, 0.5), "1:05.50");
}

#[test]
fn zooming_keeps_the_anchor_time_under_the_pointer() {
    let (px, scroll) = zoom_about(10.0, 50.0, 100.0, 100.0);
    near(px, 100.0);
    near(scroll, 11.0);
}

#[test]
fn zoom_is_clamped_and_never_scrolls_before_zero() {
    let (px, scroll) = zoom_about(0.0, 50.0, 0.0, 0.5);
    near(px, 2.0);
    near(scroll, 0.0);
}

#[test]
fn lanes_stack_by_kind_height() {
    let tracks = vec![track(TrackKind::Video, vec![]), track(TrackKind::Audio, vec![])];
    let (lanes, total) = layout_lanes(&tracks);
    near(total as f64, 116.0);
    assert_eq!(lane_at(&lanes, 59.9).unwrap().index, 0);
    assert_eq!(lane_at(&lanes, 60.0).unwrap().index, 1);
    assert!(lane_at(&lanes, 116.0).is_none());
}

#[test]
fn edges_are_a_third_of_a_narrow_clip() {
    assert_eq!(edge_hit(100.0, 3.0), Some(TrimSide::Start));
    assert_eq!(edge_hit(100.0, 97.0), Some(TrimSide::End));
    assert_eq!(edge_hit(100.0, 50.0), None);
    assert_eq!(edge_hit(12.0, 5.0), None);
}

#[test]
fn following_scrolls_only_when_the_playhead_leaves_the_view() {
    assert_eq!(follow_scroll(0.0, 10.0, 1000.0, 5.0, true, true), None);
    assert_eq!(follow_scroll(0.0, 10.0, 1000.0, 95.0, true, true), Some(85.0));
    assert_eq!(follow_scroll(0.0, 10.0, 1000.0, 99.0, false, false), None);
    assert_eq!(follow_scroll(0.0, 10.0, 1000.0, 99.0, false, true), Some(2.0));
}

#[test]
fn scrubbing_snaps_only_when_asked() {
    near(scrub_time(-1.0, false, &[], 0.2), 0.0);
    near(scrub_time(5.1, true, &[5.0], 0.2), 5.0);
    near(scrub_time(5.1, false, &[5.0], 0.2), 5.1);
}

#[test]
fn hit_testing_finds_body_edges_fades_and_gaps() {
    let plain = project_of(vec![clip("c", 1.0, 0.0, 3.0)]);
    let (lanes, _) = layout_lanes(&plain.tracks);
    let at = |x: f32, y: f32| hit_test(&plain, &lanes, &geo(), &[], pos2(x, y));
    assert_eq!(at(400.0, 50.0), Hit::Body("c".into()));
    assert_eq!(at(272.0, 50.0), Hit::Edge("c".into(), TrimSide::Start));
    assert_eq!(at(568.0, 50.0), Hit::Edge("c".into(), TrimSide::End));
    assert_eq!(at(200.0, 50.0), Hit::Empty);
    assert_eq!(at(400.0, 100.0), Hit::Empty);
    let mut faded = clip("c", 1.0, 0.0, 3.0);
    faded.fade_in_sec = Some(1.0);
    let faded = project_of(vec![faded]);
    assert_eq!(hit_test(&faded, &lanes, &geo(), &[], pos2(370.0, 30.0)), Hit::Fade("c".into(), TrimSide::Start));
}

#[test]
fn peak_notes_flag_hot_and_silent_tracks() {
    assert_eq!(peak_note(None), None);
    assert_eq!(peak_note(Some(0.0)), Some(("silent".into(), false)));
    assert_eq!(peak_note(Some(1.0)), Some(("0.0 dB".into(), true)));
    assert_eq!(peak_note(Some(0.5)), Some(("-6.0 dB".into(), false)));
}

#[test]
fn loudest_never_returns_zero() {
    near(loudest(&[0.0, 0.0]) as f64, 1.0);
    near(loudest(&[-0.5, 0.25]) as f64, 0.5);
}

#[test]
fn track_loudness_scales_with_the_track_volume() {
    let mut peaks = Peaks { peaks: vec![0.0; 500], peaks_rate: 500.0 };
    peaks.peaks[20] = -0.5;
    peaks.peaks[21] = 0.8;
    let mut audio = track(TrackKind::Audio, vec![clip("c", 0.0, 0.0, 1.0)]);
    audio.volume = 0.5;
    let found = track_loudest(&audio, |_, _| Some(&peaks)).unwrap();
    near(found as f64, 0.4);
    assert!(track_loudest(&audio, |_, _| None).is_none());
}
