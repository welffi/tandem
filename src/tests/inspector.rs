use super::{clip, effect};
use crate::insert::default_text;
use crate::inspector::clip_title;
use crate::inspector_effects::{Change, changed};
use crate::inspector_keys::{KeyAction, Outcome, apply, keying, spot, value_at};
use crate::inspector_project::{aspect_label, size_for};
use crate::inspector_speed::{single_speed, with_key};
use crate::inspector_text::placed;
use crate::keyframes::{keys_of, set_key};
use crate::types::{AnimField, Clip, Effect, EffectKind, SpeedKey};

fn keyed(times: &[f64]) -> Effect {
    times.iter().fold(effect(EffectKind::Transform), |e, t| set_key(&e, AnimField::X, *t, *t * 10.0))
}

#[test]
fn sizes_keep_the_ratio_and_stay_even() {
    assert_eq!(size_for(16.0 / 9.0, 1080), (1920, 1080));
    assert_eq!(size_for(9.0 / 16.0, 1080), (1080, 1920));
    assert_eq!(size_for(1.0, 720), (720, 720));
    assert_eq!(size_for(4.0 / 5.0, 1080), (1080, 1350));
    assert_eq!(size_for(21.0 / 9.0, 1080), (2520, 1080));
}

#[test]
fn odd_results_are_rounded_to_even() {
    let (w, h) = size_for(4.0 / 3.0, 720);
    assert_eq!((w % 2, h % 2), (0, 0));
}

#[test]
fn aspect_labels_match_presets_or_say_custom() {
    assert_eq!(aspect_label(1920, 1080), "16:9");
    assert_eq!(aspect_label(1080, 1920), "9:16");
    assert_eq!(aspect_label(1080, 1350), "4:5");
    assert_eq!(aspect_label(1000, 700), "Custom");
}

#[test]
fn the_spot_follows_the_playhead_and_flags_outside() {
    let c = clip("c", 10.0, 2.0, 6.0);
    let inside = spot(&c, 11.0);
    assert!((inside.source_sec - 3.0).abs() < 1e-9 && !inside.outside);
    assert!(spot(&c, 9.0).outside);
    assert!(spot(&c, 15.0).outside);
}

#[test]
fn keying_reports_neighbours_around_the_spot() {
    let e = keyed(&[1.0, 3.0]);
    let at = |s: f64| keying(&e, AnimField::X, &crate::inspector_keys::Spot { source_sec: s, outside: false });
    let middle = at(2.0);
    assert!(middle.animated && !middle.here && middle.has_prev && middle.has_next);
    let on_first = at(1.0);
    assert!(on_first.here && !on_first.has_prev && on_first.has_next);
    let on_last = at(3.0);
    assert!(on_last.here && on_last.has_prev && !on_last.has_next);
}

#[test]
fn a_plain_value_is_not_animated() {
    let e = effect(EffectKind::Transform);
    let k = keying(&e, AnimField::X, &crate::inspector_keys::Spot { source_sec: 0.0, outside: false });
    assert!(!k.animated && !k.here && !k.has_prev && !k.has_next);
}

#[test]
fn animate_adds_one_key_and_a_second_press_removes_them_all() {
    let e = effect(EffectKind::Transform);
    let at = crate::inspector_keys::Spot { source_sec: 2.0, outside: false };
    let Outcome::Item(animated) = apply(&e, AnimField::X, &at, 0.5, KeyAction::Animate) else { panic!() };
    assert_eq!(keys_of(&animated, AnimField::X).len(), 1);
    let Outcome::Item(plain) = apply(&animated, AnimField::X, &at, 0.5, KeyAction::Animate) else { panic!() };
    assert!(keys_of(&plain, AnimField::X).is_empty());
}

#[test]
fn the_diamond_toggles_a_key_at_the_spot() {
    let e = keyed(&[1.0]);
    let at = crate::inspector_keys::Spot { source_sec: 2.0, outside: false };
    let Outcome::Item(added) = apply(&e, AnimField::X, &at, 0.0, KeyAction::Key) else { panic!() };
    assert_eq!(keys_of(&added, AnimField::X).len(), 2);
    let Outcome::Item(removed) = apply(&added, AnimField::X, &at, 0.0, KeyAction::Key) else { panic!() };
    assert_eq!(keys_of(&removed, AnimField::X).len(), 1);
}

#[test]
fn previous_and_next_seek_to_the_neighbouring_keys() {
    let e = keyed(&[1.0, 3.0, 5.0]);
    let at = crate::inspector_keys::Spot { source_sec: 3.0, outside: false };
    assert!(matches!(apply(&e, AnimField::X, &at, 0.0, KeyAction::Prev), Outcome::Seek(s) if (s - 1.0).abs() < 1e-9));
    assert!(matches!(apply(&e, AnimField::X, &at, 0.0, KeyAction::Next), Outcome::Seek(s) if (s - 5.0).abs() < 1e-9));
}

#[test]
fn value_at_falls_back_when_nothing_is_set() {
    let e = effect(EffectKind::Transform);
    let at = crate::inspector_keys::Spot { source_sec: 0.0, outside: false };
    assert_eq!(value_at(&e, AnimField::X, &at, 0.25), 0.25);
}

#[test]
fn the_clip_tab_is_named_after_the_selection() {
    let plain = clip("a", 0.0, 0.0, 1.0);
    let text = Clip { text: Some(default_text()), ..clip("t", 0.0, 0.0, 1.0) };
    let adjust = Clip { adjustment: true, ..clip("j", 0.0, 0.0, 1.0) };
    assert_eq!(clip_title(&[plain.clone()]), "Clip");
    assert_eq!(clip_title(&[text.clone()]), "Text");
    assert_eq!(clip_title(&[adjust]), "Adjustment");
    assert_eq!(clip_title(&[plain.clone(), plain.clone()]), "2 clips");
    assert_eq!(clip_title(&[plain, text]), "Text");
}

fn named(id: &str, kind: EffectKind) -> Effect {
    Effect { id: id.into(), ..effect(kind) }
}

fn ids(list: &[Effect]) -> Vec<&str> {
    list.iter().map(|e| e.id.as_str()).collect()
}

#[test]
fn effects_move_within_the_list_and_stop_at_the_ends() {
    let list = vec![named("a", EffectKind::Grayscale), named("b", EffectKind::Transform), named("c", EffectKind::ChromaKey)];
    assert_eq!(ids(&changed(&list, &Change::Move("b".into(), -1))), ["b", "a", "c"]);
    assert_eq!(ids(&changed(&list, &Change::Move("b".into(), 1))), ["a", "c", "b"]);
    assert_eq!(ids(&changed(&list, &Change::Move("a".into(), -1))), ["a", "b", "c"]);
    assert_eq!(ids(&changed(&list, &Change::Move("c".into(), 1))), ["a", "b", "c"]);
}

#[test]
fn effects_are_replaced_removed_and_added_by_id() {
    let list = vec![named("a", EffectKind::Grayscale), named("b", EffectKind::Transform)];
    let off = Effect { enabled: false, ..named("b", EffectKind::Transform) };
    let replaced = changed(&list, &Change::Replace(off, None));
    assert!(replaced[0].enabled && !replaced[1].enabled);
    assert_eq!(ids(&changed(&list, &Change::Remove("a".into()))), ["b"]);
    assert_eq!(ids(&changed(&list, &Change::Add(named("c", EffectKind::DropShadow)))), ["a", "b", "c"]);
    assert_eq!(ids(&changed(&list, &Change::Remove("zzz".into()))), ["a", "b"]);
}

#[test]
fn a_single_speed_of_one_clears_the_keys() {
    assert!(single_speed(&[], 2.0, 1.0).is_empty());
    let keys = single_speed(&[], 2.0, 2.0);
    assert_eq!(keys, vec![SpeedKey { source_sec: 2.0, speed: 2.0 }]);
}

#[test]
fn a_single_speed_keeps_the_existing_key_position_and_clamps() {
    let keys = single_speed(&[SpeedKey { source_sec: 1.5, speed: 3.0 }], 0.0, 50.0);
    assert_eq!(keys, vec![SpeedKey { source_sec: 1.5, speed: 10.0 }]);
}

#[test]
fn a_speed_key_replaces_one_at_the_same_spot_and_stays_sorted() {
    let keys = [SpeedKey { source_sec: 1.0, speed: 1.0 }, SpeedKey { source_sec: 3.0, speed: 2.0 }];
    let added = with_key(&keys, SpeedKey { source_sec: 2.0, speed: 0.5 });
    assert_eq!(added.iter().map(|k| k.source_sec).collect::<Vec<_>>(), [1.0, 2.0, 3.0]);
    let replaced = with_key(&added, SpeedKey { source_sec: 2.005, speed: 4.0 });
    assert_eq!(replaced.len(), 3);
    assert_eq!(replaced[1].speed, 4.0);
}

#[test]
fn placing_text_sets_both_positions_at_the_spot() {
    let text = default_text();
    let moved = placed(&text, 1.0, (0.2, 0.9));
    assert_eq!((moved.x, moved.y), (0.2, 0.9));
    let animated = placed(&set_key(&text, AnimField::X, 0.0, 0.5), 2.0, (0.8, 0.1));
    assert_eq!(keys_of(&animated, AnimField::X).len(), 2);
}
