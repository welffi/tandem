use super::{clip, near, track};
use crate::edits::{blank_project, retime_clips};
use crate::speed::{local_at, source_at, speed_at};
use crate::timeline::{TrimSide, apply_trim, clip_duration, clip_end, split_clip};
use crate::types::{Clip, Project, SpeedKey, TrackKind};

fn keyed(keys: &[(f64, f64)]) -> Clip {
    Clip { speed_keys: Some(keys.iter().map(|&(source_sec, speed)| SpeedKey { source_sec, speed }).collect()), ..clip("c", 0.0, 0.0, 8.0) }
}

#[test]
fn no_keys_is_one_to_one() {
    let c = clip("c", 0.0, 2.0, 8.0);
    assert_eq!(source_at(&c, 1.5), 3.5);
    assert_eq!(clip_duration(&c), 6.0);
}

#[test]
fn constant_speed_scales_duration() {
    let c = keyed(&[(0.0, 2.0)]);
    near(clip_duration(&c), 4.0);
    near(source_at(&c, 1.0), 2.0);
    near(speed_at(&c, 3.0), 2.0);
}

#[test]
fn ramp_is_smooth_and_invertible() {
    let c = keyed(&[(2.0, 1.0), (4.0, 3.0)]);
    near(source_at(&c, 2.0), 2.0);
    let d = clip_duration(&c);
    assert!(d > 2.0 + 2.0 / 3.0 + 4.0 / 3.0 && d < 2.0 + 2.0 + 4.0 / 3.0);
    for t in [0.5, 2.3, 2.9, 3.4, 4.0] {
        near(local_at(&c, source_at(&c, t)), t);
    }
    assert!(speed_at(&c, 2.5) > 1.0 && speed_at(&c, 2.5) < 3.0);
}

#[test]
fn split_keeps_the_speed_curve() {
    let c = keyed(&[(2.0, 1.0), (4.0, 3.0)]);
    let (a, b) = split_clip(&c, 2.5).unwrap();
    near(clip_end(&a), 2.5);
    near(b.timeline_start_sec, 2.5);
    near(clip_end(&b), clip_end(&c));
    near(source_at(&b, 0.5), source_at(&c, 3.0));
}

#[test]
fn trim_maps_through_the_speed() {
    let c = keyed(&[(0.0, 2.0)]);
    near(apply_trim(&c, TrimSide::End, 1.0).source_out_sec, 2.0);
    near(apply_trim(&c, TrimSide::Start, 1.0).source_in_sec, 2.0);
}

#[test]
fn slowing_down_stops_at_the_next_clip() {
    let t = track(TrackKind::Video, vec![clip("c", 0.0, 0.0, 4.0), clip("d", 5.0, 0.0, 8.0)]);
    let project = Project { tracks: vec![t], ..blank_project("p") };
    let out = retime_clips(&project, &["c".into()], |c| Clip { speed_keys: Some(vec![SpeedKey { source_sec: 0.0, speed: 0.5 }]), ..c.clone() });
    let (slowed, next) = (&out.tracks[0].clips[0], &out.tracks[0].clips[1]);
    near(clip_end(slowed), 5.0);
    near(slowed.source_out_sec, 2.5);
    assert_eq!(next.timeline_start_sec, 5.0);
}
