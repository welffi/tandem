use super::{clip, near, track};
use crate::edits::{blank_project, find_clip};
use crate::store::Store;
use crate::timeline::{TrimSide, clip_end};
use crate::timeline_drag::{Drag, Pointing, compute};
use crate::timeline_ops::keys_by_clip;
use crate::types::{Project, TrackKind};
use eframe::egui::pos2;

fn base() -> Project {
    Project { tracks: vec![track(TrackKind::Video, vec![clip("c", 2.0, 0.0, 2.0)])], ..blank_project("p") }
}

fn mover(base: Project, copy: bool) -> Drag {
    Drag::Move { base, anchor: "c".into(), clip_ids: vec!["c".into()], track_id: "t".into(), grab_offset: 0.5, down: pos2(0.0, 0.0), active: true, toggle: false, copy }
}

fn at(sec: f64, snapping: bool, alt: bool) -> Pointing {
    Pointing { sec, lane: Some(("t".into(), TrackKind::Video)), snapping, alt, playhead: 5.0, px_per_sec: 100.0 }
}

fn start_of(project: &Project) -> f64 {
    find_clip(project, "c").unwrap().1.timeline_start_sec
}

#[test]
fn moving_follows_the_pointer_and_stops_at_zero() {
    let mut drag = mover(base(), false);
    near(start_of(&compute(&mut drag, &at(5.5, false, false)).unwrap().project), 5.0);
    near(start_of(&compute(&mut drag, &at(0.2, false, false)).unwrap().project), 0.0);
}

#[test]
fn moving_snaps_to_the_playhead_unless_alt_is_held() {
    let mut drag = mover(base(), false);
    let view = compute(&mut drag, &at(5.45, true, false)).unwrap();
    near(start_of(&view.project), 5.0);
    assert_eq!(view.snap_sec, Some(5.0));
    let free = compute(&mut drag, &at(5.45, true, true)).unwrap();
    near(start_of(&free.project), 4.95);
    assert_eq!(free.snap_sec, None);
}

#[test]
fn a_copy_drag_leaves_the_original_in_place() {
    let mut drag = Drag::Move { base: base(), anchor: "c".into(), clip_ids: vec!["c".into()], track_id: "t".into(), grab_offset: 0.5, down: pos2(0.0, 0.0), active: false, toggle: false, copy: true };
    drag.start_moving();
    let Drag::Move { clip_ids, base, .. } = &drag else { panic!("not a move") };
    assert_ne!(clip_ids[0], "c");
    assert_eq!(base.tracks[0].clips.len(), 2);
    assert!(drag.is_copy_move());
}

#[test]
fn trimming_stays_inside_the_allowed_range() {
    let mut drag = Drag::Trim { base: base(), anchor: "c".into(), clip_ids: vec!["c".into()], side: TrimSide::End, range: (2.5, 10.0) };
    near(clip_end(find_clip(&compute(&mut drag, &at(3.0, false, false)).unwrap().project, "c").unwrap().1), 3.0);
    near(clip_end(find_clip(&compute(&mut drag, &at(1.0, false, false)).unwrap().project, "c").unwrap().1), 2.5);
}

#[test]
fn dragging_a_fade_handle_sets_the_fade_length() {
    let mut drag = Drag::Fade { base: base(), anchor: "c".into(), clip_ids: vec!["c".into()], side: TrimSide::Start };
    let view = compute(&mut drag, &at(3.0, false, false)).unwrap();
    assert_eq!(find_clip(&view.project, "c").unwrap().1.fade_in_sec, Some(1.0));
}

#[test]
fn scrubbing_has_no_preview_project() {
    assert!(compute(&mut Drag::Scrub, &at(1.0, false, false)).is_none());
}

#[test]
fn key_references_group_by_clip() {
    let refs: Vec<String> = ["a/k1", "a/k2", "b/k3", "bad"].iter().map(|s| s.to_string()).collect();
    let grouped = keys_by_clip(&refs);
    assert_eq!(grouped["a"], vec!["k1", "k2"]);
    assert_eq!(grouped["b"], vec!["k3"]);
    assert_eq!(grouped.len(), 2);
}

fn store_with(clips: Vec<crate::types::Clip>) -> Store {
    let mut store = Store::new();
    store.load_project(Project { tracks: vec![track(TrackKind::Video, clips)], ..blank_project("p") }, None);
    store
}

#[test]
fn split_cuts_every_clip_under_the_playhead() {
    let mut store = store_with(vec![clip("c", 0.0, 0.0, 4.0)]);
    store.playhead_sec = 1.0;
    store.split_at_playhead();
    assert_eq!(store.project().tracks[0].clips.len(), 2);
}

#[test]
fn delete_removes_the_selection_and_can_be_undone() {
    let mut store = store_with(vec![clip("c", 0.0, 0.0, 4.0)]);
    store.set_selection(vec!["c".into()]);
    store.delete_selected_clips();
    assert!(store.project().tracks[0].clips.is_empty());
    store.undo();
    assert_eq!(store.project().tracks[0].clips.len(), 1);
}

#[test]
fn ripple_delete_closes_the_gap_and_moves_the_playhead() {
    let mut store = store_with(vec![clip("a", 0.0, 0.0, 2.0), clip("b", 2.0, 0.0, 2.0)]);
    store.set_selection(vec!["a".into()]);
    store.playhead_sec = 3.0;
    store.ripple_delete_selection();
    let remaining = &store.project().tracks[0].clips;
    assert_eq!(remaining.len(), 1);
    near(remaining[0].timeline_start_sec, 0.0);
    near(store.playhead_sec, 0.0);
}

#[test]
fn added_text_and_adjustments_land_at_the_playhead_and_get_selected() {
    let mut store = store_with(vec![]);
    store.playhead_sec = 1.0;
    store.add_text_at_playhead();
    let text_id = store.selection[0].clone();
    let (_, text) = find_clip(store.project(), &text_id).unwrap();
    near(text.timeline_start_sec, 1.0);
    near(clip_end(text), 4.0);
    store.add_adjustment_at_playhead();
    let (_, adjust) = find_clip(store.project(), &store.selection[0]).unwrap();
    assert!(adjust.adjustment);
    near(clip_end(adjust), 6.0);
}

#[test]
fn zooming_in_around_the_start_keeps_the_scroll() {
    let mut store = Store::new();
    store.zoom_by(1.5, 1000.0);
    near(store.px_per_sec, 120.0);
    near(store.scroll_sec, 0.0);
}

#[test]
fn fit_zoom_covers_the_project() {
    let mut store = store_with(vec![clip("c", 0.0, 0.0, 4.0)]);
    store.zoom_to_fit(1000.0);
    near(store.px_per_sec, 192.0);
}
