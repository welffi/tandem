use super::{clip, media, track};
use crate::clip_menu::clip_menu;
use crate::edits::{blank_project, find_clip};
use crate::reverse::{Range, Swap, apply_swaps, plan_reverse, reversed_asset};
use crate::types::{Clip, MediaAsset, Project, ReverseSource, TrackKind};

fn project(kind: TrackKind, clips: Vec<Clip>) -> Project {
    Project { assets: vec![media("a")], tracks: vec![track(kind, clips)], ..blank_project("p") }
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn split_needs_the_playhead_inside_a_selected_clip() {
    let p = project(TrackKind::Video, vec![clip("c", 2.0, 0.0, 6.0)]);
    assert!(clip_menu(&p, &ids(&["c"]), "c", 4.0).unwrap().split);
    assert!(!clip_menu(&p, &ids(&["c"]), "c", 9.0).unwrap().split);
    assert!(!clip_menu(&p, &[], "c", 4.0).unwrap().split);
}

#[test]
fn freeze_is_offered_on_video_and_enabled_inside_the_clip() {
    let p = project(TrackKind::Video, vec![clip("c", 2.0, 0.0, 6.0)]);
    assert_eq!(clip_menu(&p, &ids(&["c"]), "c", 4.0).unwrap().freeze, Some(true));
    assert_eq!(clip_menu(&p, &ids(&["c"]), "c", 30.0).unwrap().freeze, Some(false));
    assert_eq!(clip_menu(&project(TrackKind::Audio, vec![clip("c", 2.0, 0.0, 6.0)]), &[], "c", 4.0).unwrap().freeze, None);
}

#[test]
fn a_held_clip_offers_end_freeze_not_freeze_or_reverse() {
    let held = Clip { freeze_at: Some(1.0), ..clip("c", 2.0, 0.0, 6.0) };
    let menu = clip_menu(&project(TrackKind::Video, vec![held]), &[], "c", 4.0).unwrap();
    assert_eq!((menu.freeze, menu.end_freeze, menu.reverse), (None, Some(true), false));
}

#[test]
fn trim_silence_is_for_audio_and_reverse_skips_text_and_adjustments() {
    let audio = clip_menu(&project(TrackKind::Audio, vec![clip("c", 0.0, 0.0, 6.0)]), &[], "c", 1.0).unwrap();
    assert!(audio.trim_silence && audio.reverse);
    let text = clip_menu(&project(TrackKind::Text, vec![clip("c", 0.0, 0.0, 6.0)]), &[], "c", 1.0).unwrap();
    assert!(!text.trim_silence && !text.reverse);
    let adjustment = Clip { adjustment: true, ..clip("c", 0.0, 0.0, 6.0) };
    assert!(!clip_menu(&project(TrackKind::Video, vec![adjustment]), &[], "c", 1.0).unwrap().reverse);
}

#[test]
fn clips_with_the_same_range_share_one_reverse() {
    let p = project(TrackKind::Video, vec![clip("c1", 0.0, 1.0, 5.0), clip("c2", 9.0, 1.0, 5.0), clip("c3", 20.0, 2.0, 5.0)]);
    let plan = plan_reverse(&p, &ids(&["c1", "c2", "c3"]));
    assert_eq!(plan.ranges.len(), 2);
    assert_eq!(plan.ranges[0], Range { asset_id: "a".into(), in_sec: 1.0, out_sec: 5.0, video: true, clip_ids: ids(&["c1", "c2"]) });
    assert!(plan.swaps.is_empty());
}

#[test]
fn text_held_and_still_clips_are_not_reversed() {
    let mut p = project(
        TrackKind::Video,
        vec![Clip { text: Some(crate::insert::default_text()), ..clip("t", 0.0, 0.0, 3.0) }, Clip { freeze_at: Some(1.0), ..clip("h", 5.0, 0.0, 3.0) }, clip("s", 10.0, 0.0, 3.0)],
    );
    p.assets[0].still = true;
    assert!(plan_reverse(&p, &ids(&["t", "h", "s"])).ranges.is_empty());
}

#[test]
fn a_reversed_clip_swaps_back_to_its_source() {
    let back = MediaAsset { reverse_of: Some(ReverseSource { asset_id: "a".into(), in_sec: 1.0, out_sec: 5.0, video: true }), ..media("r") };
    let mut p = project(TrackKind::Video, vec![Clip { asset_id: "r".into(), ..clip("c", 0.0, 0.0, 4.0) }]);
    p.assets.push(back);
    let plan = plan_reverse(&p, &ids(&["c"]));
    assert_eq!(plan.swaps, vec![Swap { clip_ids: ids(&["c"]), asset_id: "a".into(), pivot_sec: 5.0 }]);
    let restored = apply_swaps(&p, Vec::new(), &plan.swaps);
    let c = find_clip(&restored, "c").unwrap().1;
    assert_eq!((c.asset_id.as_str(), c.source_in_sec, c.source_out_sec), ("a", 1.0, 5.0));
}

#[test]
fn applying_a_reverse_adds_the_asset_once_and_mirrors_the_clip() {
    let p = project(TrackKind::Video, vec![clip("c", 0.0, 1.0, 5.0)]);
    let range = Range { asset_id: "a".into(), in_sec: 1.0, out_sec: 5.0, video: true, clip_ids: ids(&["c"]) };
    let made = reversed_asset(media("r"), &p.assets[0], &range);
    assert_eq!(made.name, "a (reversed)");
    let swap = Swap { clip_ids: range.clip_ids.clone(), asset_id: "r".into(), pivot_sec: 5.0 };
    let next = apply_swaps(&p, vec![made.clone(), made], &[swap]);
    assert_eq!(next.assets.len(), 2);
    let c = find_clip(&next, "c").unwrap().1;
    assert_eq!((c.asset_id.as_str(), c.source_in_sec, c.source_out_sec), ("r", 0.0, 4.0));
}
