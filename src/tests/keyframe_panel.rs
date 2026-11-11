use super::{clip, effect, near, track};
use crate::curve_math::{CurveChange, changed, dragged_handle, eased, from_screen, numbers, same_curve, selected_ease, to_screen, unit_box_in, visible_y};
use crate::edits::empty_project;
use crate::keyframe_rows::{animated_layers, field_label, format_value, key_fraction, playhead_fraction, rows_of};
use crate::keyframes::LINEAR;
use crate::timeline_ops::pick_keys;
use crate::types::{AnimField, Clip, CurvePreset, EffectKind, Keyframe, Project, TrackKind};
use eframe::egui::{Rect, pos2};
use std::collections::BTreeMap;

fn key(id: &str, source_sec: f64, value: f64) -> Keyframe {
    Keyframe { id: id.into(), source_sec, value, ease: None }
}

fn animated_clip() -> Clip {
    let mut fx = effect(EffectKind::Grayscale);
    fx.keys = Some(BTreeMap::from([(AnimField::Amount, vec![key("k1", 0.0, 0.0), key("k2", 2.0, 1.0)])]));
    Clip { effects: Some(vec![fx]), ..clip("c", 10.0, 0.0, 4.0) }
}

fn project() -> Project {
    Project { tracks: vec![track(TrackKind::Video, vec![animated_clip()])], ..empty_project() }
}

fn ease_of(project: &Project, key_id: &str) -> Option<[f64; 4]> {
    project.tracks[0].clips[0].effects()[0].keys.as_ref().unwrap()[&AnimField::Amount].iter().find(|k| k.id == key_id).and_then(|k| k.ease)
}

#[test]
fn picking_keys_toggles_only_when_additive() {
    let held = vec!["c/a".to_string(), "c/b".to_string()];
    assert_eq!(pick_keys(&held, "c/a", false), held);
    assert_eq!(pick_keys(&held, "c/z", false), vec!["c/z"]);
    assert_eq!(pick_keys(&held, "c/a", true), vec!["c/b"]);
    assert_eq!(pick_keys(&held, "c/z", true), vec!["c/a", "c/b", "c/z"]);
}

#[test]
fn easing_touches_only_the_selected_keys() {
    let next = eased(&project(), &["c/k1".to_string()], [0.4, 0.0, 0.6, 1.0]);
    assert_eq!(ease_of(&next, "k1"), Some([0.4, 0.0, 0.6, 1.0]));
    assert_eq!(ease_of(&next, "k2"), None);
}

#[test]
fn selected_ease_is_linear_without_one() {
    assert_eq!(selected_ease(&project(), &["c/k1".to_string()]), LINEAR);
    let next = eased(&project(), &["c/k2".to_string()], [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(selected_ease(&next, &["c/k1".to_string(), "c/k2".to_string()]), [0.1, 0.2, 0.3, 0.4]);
}

#[test]
fn handles_clamp_and_snap() {
    let start = [0.25, 0.1, 0.25, 1.0];
    assert_eq!(dragged_handle(start, 0, (1.4, 5.0), false, (-0.7, 1.7)), [1.0, 1.7, 0.25, 1.0]);
    assert_eq!(dragged_handle(start, 1, (-0.3, -3.0), false, (-0.7, 1.7)), [0.25, 0.1, 0.0, -0.7]);
    let snapped = dragged_handle(start, 0, (0.52, 0.48), true, (-0.7, 1.7));
    near(snapped[0], 0.5);
    near(snapped[1], 0.5);
}

#[test]
fn box_positions_round_trip() {
    let unit_box = Rect::from_min_size(pos2(20.0, 40.0), eframe::egui::vec2(80.0, 80.0));
    assert_eq!(to_screen(unit_box, (0.0, 0.0)), pos2(20.0, 120.0));
    assert_eq!(to_screen(unit_box, (1.0, 1.0)), pos2(100.0, 40.0));
    let (x, y) = from_screen(unit_box, to_screen(unit_box, (0.25, 1.5)));
    near(x, 0.25);
    near(y, 1.5);
}

#[test]
fn the_box_fills_the_view_inside_its_margin() {
    let view = Rect::from_min_size(pos2(10.0, 20.0), eframe::egui::vec2(170.0, 240.0));
    let unit_box = unit_box_in(view, eframe::egui::vec2(14.0, 16.0));
    assert_eq!(to_screen(unit_box, (0.0, 0.0)), pos2(24.0, 244.0));
    assert_eq!(to_screen(unit_box, (1.0, 1.0)), pos2(166.0, 36.0));
    let (low, high) = visible_y(view, unit_box);
    near(low, -16.0 / 208.0);
    near(high, 1.0 + 16.0 / 208.0);
}

#[test]
fn curves_compare_within_a_small_gap() {
    assert!(same_curve([0.42, 0.0, 0.58, 1.0], [0.4204, 0.0, 0.5796, 1.0]));
    assert!(!same_curve([0.42, 0.0, 0.58, 1.0], [0.43, 0.0, 0.58, 1.0]));
    assert_eq!(numbers([0.254, 0.0, 0.5, 1.0]), "0.25, 0, 0.5, 1");
}

#[test]
fn saved_curves_change_by_id() {
    let list = vec![CurvePreset { id: "a".into(), name: "One".into(), bezier: LINEAR }];
    let added = changed(&list, &CurveChange::Add(CurvePreset { id: "b".into(), name: "Two".into(), bezier: LINEAR }));
    assert_eq!(added.len(), 2);
    let renamed = changed(&added, &CurveChange::Rename("b".into(), "Zwei".into()));
    assert_eq!(renamed[1].name, "Zwei");
    assert_eq!(changed(&renamed, &CurveChange::Delete("a".into())).len(), 1);
}

#[test]
fn rows_list_only_fields_with_keys() {
    let rows = rows_of(&animated_clip());
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].group.as_str(), rows[0].label), ("Grayscale", "Amount"));
    assert!(rows_of(&clip("plain", 0.0, 0.0, 1.0)).is_empty());
}

#[test]
fn labels_follow_the_effect() {
    assert_eq!(field_label(&effect(EffectKind::ChromaKey), AnimField::Amount), "Tolerance");
    assert_eq!(field_label(&effect(EffectKind::DropShadow), AnimField::Amount), "Opacity");
    let mut wide = effect(EffectKind::Transform);
    assert_eq!(field_label(&wide, AnimField::Size), "Size");
    wide.height = Some(1.0);
    assert_eq!(field_label(&wide, AnimField::Size), "Width");
}

#[test]
fn values_read_as_percent_or_degrees() {
    let row = |field, text| crate::keyframe_rows::KeyRow { id: String::new(), group: String::new(), label: "", field, text, keys: vec![] };
    assert_eq!(format_value(&row(AnimField::Amount, false), 0.456), "45.6%");
    assert_eq!(format_value(&row(AnimField::Rotation, false), 44.6), "45°");
    assert_eq!(format_value(&row(AnimField::Size, true), 8.04), "8%");
}

#[test]
fn lane_positions_follow_the_clip() {
    let clip = animated_clip();
    near(key_fraction(&clip, &clip.effects()[0].keys.as_ref().unwrap()[&AnimField::Amount][1]) as f64, 0.5);
    assert_eq!(playhead_fraction(&clip, 12.0), Some(0.5));
    assert_eq!(playhead_fraction(&clip, 9.0), None);
    assert_eq!(playhead_fraction(&clip, 14.5), None);
}

#[test]
fn only_selected_animated_clips_show() {
    let project = project();
    assert_eq!(animated_layers(&project, &["c".to_string()]).len(), 1);
    assert!(animated_layers(&project, &[]).is_empty());
}
