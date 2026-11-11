use crate::keyframes::{Keyed, effect_at, key_at, keys_of, remove_keys, set_key, stop_animating};
use crate::speed::source_at;
use crate::timeline::clip_duration;
use crate::types::{AnimField, Clip};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyAction {
    Animate,
    Key,
    Prev,
    Next,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keying {
    pub animated: bool,
    pub here: bool,
    pub outside: bool,
    pub has_prev: bool,
    pub has_next: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub source_sec: f64,
    pub outside: bool,
}

pub fn spot(clip: &Clip, playhead_sec: f64) -> Spot {
    let local = playhead_sec - clip.timeline_start_sec;
    let length = clip_duration(clip);
    Spot { source_sec: source_at(clip, local.clamp(0.0, length)), outside: local < 0.0 || local > length }
}

pub fn value_at<T: Keyed>(item: &T, field: AnimField, spot: &Spot, fallback: f64) -> f64 {
    effect_at(item, spot.source_sec).field(field).unwrap_or(fallback)
}

fn neighbor<T: Keyed>(item: &T, field: AnimField, spot: &Spot, forward: bool) -> Option<f64> {
    let here = key_at(item, field, spot.source_sec).map(|k| k.id.as_str());
    let mut others = keys_of(item, field).iter().filter(|k| Some(k.id.as_str()) != here);
    if forward { others.find(|k| k.source_sec > spot.source_sec) } else { others.rev().find(|k| k.source_sec < spot.source_sec) }.map(|k| k.source_sec)
}

pub fn keying<T: Keyed>(item: &T, field: AnimField, spot: &Spot) -> Keying {
    Keying {
        animated: !keys_of(item, field).is_empty(),
        here: key_at(item, field, spot.source_sec).is_some(),
        outside: spot.outside,
        has_prev: neighbor(item, field, spot, false).is_some(),
        has_next: neighbor(item, field, spot, true).is_some(),
    }
}

pub enum Outcome<T> {
    Item(T),
    Seek(f64),
}

pub fn apply<T: Keyed>(item: &T, field: AnimField, spot: &Spot, fallback: f64, action: KeyAction) -> Outcome<T> {
    let value = value_at(item, field, spot, fallback);
    match action {
        KeyAction::Animate if keys_of(item, field).is_empty() => Outcome::Item(set_key(item, field, spot.source_sec, value)),
        KeyAction::Animate => Outcome::Item(stop_animating(item, field, value)),
        KeyAction::Key => Outcome::Item(match key_at(item, field, spot.source_sec) {
            Some(key) => remove_keys(item, &[key.id.clone()]),
            None => set_key(item, field, spot.source_sec, value),
        }),
        KeyAction::Prev => neighbor(item, field, spot, false).map_or(Outcome::Item(item.clone()), Outcome::Seek),
        KeyAction::Next => neighbor(item, field, spot, true).map_or(Outcome::Item(item.clone()), Outcome::Seek),
    }
}
