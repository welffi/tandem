use super::{clip, track};
use crate::timeline::{Snap, TrimSide, apply_trim, clear_range, clip_end, fade_gain, find_snap, place_clip, split_clip, split_clip_at_times, trim_range};
use crate::types::{Clip, TrackKind};

fn layout(clips: &[Clip]) -> Vec<(String, f64, f64, f64)> {
    clips.iter().map(|c| (c.id.clone(), c.timeline_start_sec, c.source_in_sec, c.source_out_sec)).collect()
}

#[test]
fn split_divides_source_range_at_the_timeline_time() {
    let (a, b) = split_clip(&clip("c", 10.0, 2.0, 8.0), 13.0).unwrap();
    assert_eq!((a.timeline_start_sec, a.source_in_sec, a.source_out_sec), (10.0, 2.0, 5.0));
    assert_eq!((b.timeline_start_sec, b.source_in_sec, b.source_out_sec), (13.0, 5.0, 8.0));
    assert_ne!(b.id, "c");
    assert_eq!(clip_end(&b), 16.0);
}

#[test]
fn split_refuses_cuts_at_the_edges() {
    assert!(split_clip(&clip("c", 10.0, 0.0, 4.0), 10.0).is_none());
    assert!(split_clip(&clip("c", 10.0, 0.0, 4.0), 14.0).is_none());
}

#[test]
fn split_at_times_cuts_at_every_interior_time() {
    let pieces = split_clip_at_times(&clip("c", 0.0, 0.0, 4.0), &[3.0, 1.0, 2.0, 9.0]);
    let got: Vec<_> = pieces.iter().map(|p| (p.timeline_start_sec, p.source_in_sec, p.source_out_sec)).collect();
    assert_eq!(got, vec![(0.0, 0.0, 1.0), (1.0, 1.0, 2.0), (2.0, 2.0, 3.0), (3.0, 3.0, 4.0)]);
}

#[test]
fn clear_range_trims_splits_and_drops() {
    let clips = [clip("a", 0.0, 0.0, 4.0), clip("b", 5.0, 0.0, 2.0), clip("c", 8.0, 0.0, 6.0)];
    assert_eq!(layout(&clear_range(&clips, 2.0, 10.0)), vec![("a".into(), 0.0, 0.0, 2.0), ("c".into(), 10.0, 2.0, 6.0)]);
    let middle = clear_range(&[clip("x", 0.0, 0.0, 10.0)], 3.0, 4.0);
    assert_eq!(middle.len(), 2);
    assert_eq!((middle[1].timeline_start_sec, middle[1].source_in_sec), (4.0, 4.0));
    assert_ne!(middle[1].id, "x");
}

#[test]
fn place_clip_overwrites_and_keeps_order() {
    let moving = clip("m", 3.0, 0.0, 2.0);
    let out = place_clip(&[clip("a", 0.0, 0.0, 4.0), moving.clone(), clip("b", 6.0, 0.0, 2.0)], moving);
    let got: Vec<_> = out.iter().map(|c| (c.id.as_str(), c.timeline_start_sec, clip_end(c))).collect();
    assert_eq!(got, vec![("a", 0.0, 3.0), ("m", 3.0, 5.0), ("b", 6.0, 8.0)]);
}

#[test]
fn trim_range_stops_at_neighbours_and_media() {
    let t = track(TrackKind::Video, vec![clip("a", 0.0, 0.0, 2.0), clip("b", 3.0, 1.0, 3.0), clip("c", 7.0, 0.0, 1.0)]);
    let b = &t.clips[1];
    assert_eq!(trim_range(&t, b, 10.0, TrimSide::Start), (2.0, 5.0 - 0.02));
    assert_eq!(trim_range(&t, b, 10.0, TrimSide::End), (3.0 + 0.02, 7.0));
    assert_eq!(trim_range(&t, b, 3.5, TrimSide::End).1, 5.5);
}

#[test]
fn apply_trim_keeps_source_and_timeline_together() {
    let trimmed = apply_trim(&clip("c", 4.0, 2.0, 6.0), TrimSide::Start, 5.0);
    assert_eq!((trimmed.timeline_start_sec, trimmed.source_in_sec, clip_end(&trimmed)), (5.0, 3.0, 8.0));
    assert_eq!(apply_trim(&clip("c", 4.0, 2.0, 6.0), TrimSide::End, 6.0).source_out_sec, 4.0);
}

#[test]
fn find_snap_picks_the_closest_edge() {
    assert_eq!(find_snap(&[1.04, 3.2], &[0.0, 1.0, 2.0, 3.0], 0.1), Some(Snap { delta: 1.0 - 1.04, point: 1.0 }));
    assert_eq!(find_snap(&[1.5], &[0.0, 1.0, 2.0], 0.1), None);
    assert_eq!(find_snap(&[1.0], &[], 0.1), None);
}

#[test]
fn fade_gain_ramps_in_and_out() {
    let c = Clip { fade_in_sec: Some(1.0), fade_out_sec: Some(2.0), ..clip("c", 10.0, 0.0, 4.0) };
    assert_eq!(fade_gain(&c, 10.0), 0.0);
    assert_eq!(fade_gain(&c, 10.5), 0.5);
    assert_eq!(fade_gain(&c, 12.0), 1.0);
    assert_eq!(fade_gain(&c, 13.0), 0.5);
    assert_eq!(fade_gain(&clip("d", 0.0, 0.0, 4.0), 0.0), 1.0);
}
