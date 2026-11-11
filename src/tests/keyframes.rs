use super::{clip, effect};
use crate::edits::mirror_clip;
use crate::insert::default_text;
use crate::keyframes::{LINEAR, bezier_at, clip_animated, clip_keys, edit_field, effect_at, map_clip_keys, remove_keys, set_key, value_at};
use crate::types::{AnimField, Bezier, Clip, EffectKind, Keyframe, Keys};

fn key(id: &str, source_sec: f64, value: f64, ease: Option<Bezier>) -> Keyframe {
    Keyframe { id: id.into(), source_sec, value, ease }
}

const EASE_IN_OUT: Bezier = [0.42, 0.0, 0.58, 1.0];

#[test]
fn bezier_keeps_its_ends_and_climbs_steadily() {
    assert_eq!(bezier_at(EASE_IN_OUT, 0.0), 0.0);
    assert_eq!(bezier_at(EASE_IN_OUT, 1.0), 1.0);
    for u in [0.1, 0.25, 0.5, 0.9] {
        assert!((bezier_at(LINEAR, u) - u).abs() < 1e-6);
    }
    assert!((bezier_at(EASE_IN_OUT, 0.5) - 0.5).abs() < 1e-6);
    assert!(bezier_at(EASE_IN_OUT, 0.2) < 0.2);
    let samples: Vec<f64> = (0..=20).map(|i| bezier_at(EASE_IN_OUT, i as f64 / 20.0)).collect();
    assert!(samples.windows(2).all(|w| w[1] >= w[0]));
}

#[test]
fn values_hold_outside_keys_and_follow_the_earlier_ease() {
    let keys = [key("a", 1.0, 10.0, Some([0.42, 0.0, 1.0, 1.0])), key("b", 3.0, 20.0, None)];
    assert_eq!(value_at(&keys, 0.0), 10.0);
    assert_eq!(value_at(&keys, 5.0), 20.0);
    assert!(value_at(&keys, 2.0) < 15.0);
    assert!((value_at(&[key("a", 1.0, 10.0, None), key("b", 3.0, 20.0, None)], 2.0) - 15.0).abs() < 1e-6);
}

#[test]
fn editing_static_changes_it_and_animated_adds_a_key() {
    let still = effect(EffectKind::Grayscale);
    let edited = edit_field(&still, AnimField::Amount, 2.0, 0.5);
    assert_eq!(edited.amount, 0.5);
    assert!(edited.keys.is_none());
    let mut moving = still.clone();
    moving.keys = Some(Keys::from([(AnimField::Amount, vec![key("a", 0.0, 0.0, None), key("b", 4.0, 1.0, None)])]));
    let edited = edit_field(&moving, AnimField::Amount, 2.0, 0.25);
    let got: Vec<_> = edited.keys.as_ref().unwrap()[&AnimField::Amount].iter().map(|k| (k.source_sec, k.value)).collect();
    assert_eq!(got, vec![(0.0, 0.0), (2.0, 0.25), (4.0, 1.0)]);
    assert_eq!(edit_field(&edited, AnimField::Amount, 2.0, 0.75).keys.unwrap()[&AnimField::Amount].len(), 3);
}

#[test]
fn reversing_mirrors_keys_and_turns_each_ease_around() {
    let mut fx = effect(EffectKind::Grayscale);
    fx.keys = Some(Keys::from([(AnimField::Amount, vec![key("a", 1.0, 0.0, Some([0.42, 0.0, 1.0, 1.0])), key("b", 3.0, 1.0, None)])]));
    let c = Clip { effects: Some(vec![fx]), ..clip("c", 0.0, 0.0, 4.0) };
    let mirrored = mirror_clip(&c, "r", 4.0);
    let keys = &mirrored.effects.as_ref().unwrap()[0].keys.as_ref().unwrap()[&AnimField::Amount];
    assert_eq!(keys.iter().map(|k| (k.id.as_str(), k.source_sec, k.value)).collect::<Vec<_>>(), vec![("b", 1.0, 1.0), ("a", 3.0, 0.0)]);
    let ease = keys[0].ease.unwrap();
    for (got, want) in ease.iter().zip([0.0, 0.0, 0.58, 1.0]) {
        assert!((got - want).abs() < 1e-6);
    }
    assert!(keys[1].ease.is_none());
}

#[test]
fn text_takes_keys_on_position_and_size() {
    let style = default_text();
    let moving = set_key(&set_key(&style, AnimField::X, 0.0, 0.2), AnimField::X, 2.0, 0.8);
    assert_eq!((effect_at(&moving, 1.0).x * 100.0).round(), 50.0);
    assert_eq!(effect_at(&moving, 1.0).size, style.size);
    let c = Clip { asset_id: String::new(), text: Some(moving), ..clip("t", 0.0, 0.0, 2.0) };
    assert!(clip_animated(&c));
    let ids: Vec<String> = clip_keys(&c).iter().map(|k| k.id.clone()).collect();
    assert_eq!(ids.len(), 2);
    let cleared = map_clip_keys(&c, &mut |e| remove_keys(e, &ids), &mut |t| remove_keys(t, &ids));
    assert!(cleared.text.as_ref().unwrap().keys.is_none());
    assert!(!clip_animated(&cleared));
}
