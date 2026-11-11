use crate::ids::new_id;
use crate::types::{AnimField, Bezier, Clip, CurvePreset, Effect, Keyframe, Keys, TextStyle};

pub const LINEAR: Bezier = [0.0, 0.0, 1.0, 1.0];
const SAME_KEY_SEC: f64 = 1e-3;

pub fn built_in_curves() -> Vec<CurvePreset> {
    let curve = |id: &str, name: &str, bezier: Bezier| CurvePreset { id: id.into(), name: name.into(), bezier };
    vec![
        curve("linear", "Linear", LINEAR),
        curve("ease", "Ease", [0.25, 0.1, 0.25, 1.0]),
        curve("ease-in", "Ease in", [0.42, 0.0, 1.0, 1.0]),
        curve("ease-out", "Ease out", [0.0, 0.0, 0.58, 1.0]),
        curve("ease-in-out", "Ease in-out", [0.42, 0.0, 0.58, 1.0]),
        curve("expo-in", "Expo in", [0.7, 0.0, 0.84, 0.0]),
        curve("expo-out", "Expo out", [0.16, 1.0, 0.3, 1.0]),
        curve("expo-in-out", "Expo in-out", [0.87, 0.0, 0.13, 1.0]),
        curve("back-in", "Back in", [0.36, 0.0, 0.66, -0.56]),
        curve("back-out", "Back out", [0.34, 1.56, 0.64, 1.0]),
        curve("back-in-out", "Back in-out", [0.68, -0.6, 0.32, 1.6]),
    ]
}

pub trait Keyed: Clone {
    fn keys(&self) -> Option<&Keys>;
    fn keys_mut(&mut self) -> &mut Option<Keys>;
    fn field(&self, field: AnimField) -> Option<f64>;
    fn set_field(&mut self, field: AnimField, value: f64);
}

impl Keyed for Effect {
    fn keys(&self) -> Option<&Keys> {
        self.keys.as_ref()
    }

    fn keys_mut(&mut self) -> &mut Option<Keys> {
        &mut self.keys
    }

    fn field(&self, field: AnimField) -> Option<f64> {
        match field {
            AnimField::Amount => Some(self.amount),
            AnimField::Size => self.size,
            AnimField::Height => self.height,
            AnimField::X => self.x,
            AnimField::Y => self.y,
            AnimField::Rotation => self.rotation,
            AnimField::Softness => self.softness,
            AnimField::Spill => self.spill,
            AnimField::Distance => self.distance,
            AnimField::Blur => self.blur,
            AnimField::Angle => self.angle,
        }
    }

    fn set_field(&mut self, field: AnimField, value: f64) {
        match field {
            AnimField::Amount => self.amount = value,
            AnimField::Size => self.size = Some(value),
            AnimField::Height => self.height = Some(value),
            AnimField::X => self.x = Some(value),
            AnimField::Y => self.y = Some(value),
            AnimField::Rotation => self.rotation = Some(value),
            AnimField::Softness => self.softness = Some(value),
            AnimField::Spill => self.spill = Some(value),
            AnimField::Distance => self.distance = Some(value),
            AnimField::Blur => self.blur = Some(value),
            AnimField::Angle => self.angle = Some(value),
        }
    }
}

impl Keyed for TextStyle {
    fn keys(&self) -> Option<&Keys> {
        self.keys.as_ref()
    }

    fn keys_mut(&mut self) -> &mut Option<Keys> {
        &mut self.keys
    }

    fn field(&self, field: AnimField) -> Option<f64> {
        match field {
            AnimField::Size => Some(self.size),
            AnimField::X => Some(self.x),
            AnimField::Y => Some(self.y),
            _ => None,
        }
    }

    fn set_field(&mut self, field: AnimField, value: f64) {
        match field {
            AnimField::Size => self.size = value,
            AnimField::X => self.x = value,
            AnimField::Y => self.y = value,
            _ => {}
        }
    }
}

fn cubic(a: f64, b: f64, t: f64) -> f64 {
    3.0 * a * (1.0 - t).powi(2) * t + 3.0 * b * (1.0 - t) * t * t + t.powi(3)
}

pub fn bezier_at(bezier: Bezier, u: f64) -> f64 {
    let [x1, y1, x2, y2] = bezier;
    if u <= 0.0 {
        return 0.0;
    }
    if u >= 1.0 {
        return 1.0;
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    for _ in 0..32 {
        let mid = (lo + hi) / 2.0;
        if cubic(x1, x2, mid) < u {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    cubic(y1, y2, (lo + hi) / 2.0)
}

pub fn value_at(keys: &[Keyframe], source_sec: f64) -> f64 {
    if source_sec <= keys[0].source_sec {
        return keys[0].value;
    }
    for pair in keys.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if source_sec < b.source_sec {
            let u = (source_sec - a.source_sec) / (b.source_sec - a.source_sec);
            return a.value + (b.value - a.value) * bezier_at(a.ease.unwrap_or(LINEAR), u);
        }
    }
    keys[keys.len() - 1].value
}

pub fn keys_of<T: Keyed>(item: &T, field: AnimField) -> &[Keyframe] {
    item.keys().and_then(|k| k.get(&field)).map_or(&[], |v| v.as_slice())
}

pub fn is_animated<T: Keyed>(items: &[T]) -> bool {
    items.iter().any(|e| e.keys().is_some_and(|k| k.values().any(|v| !v.is_empty())))
}

pub fn effect_at<T: Keyed>(item: &T, source_sec: f64) -> T {
    let mut out = item.clone();
    if let Some(keys) = item.keys() {
        for (field, list) in keys {
            if !list.is_empty() {
                out.set_field(*field, value_at(list, source_sec));
            }
        }
    }
    out
}

fn with_keys<T: Keyed>(mut item: T, field: AnimField, mut keys: Vec<Keyframe>) -> T {
    let mut rest = item.keys().cloned().unwrap_or_default();
    if keys.is_empty() {
        rest.remove(&field);
    } else {
        keys.sort_by(|a, b| a.source_sec.total_cmp(&b.source_sec));
        rest.insert(field, keys);
    }
    *item.keys_mut() = if rest.is_empty() { None } else { Some(rest) };
    item
}

pub fn set_key<T: Keyed>(item: &T, field: AnimField, source_sec: f64, value: f64) -> T {
    let mut keys = keys_of(item, field).to_vec();
    match keys.iter_mut().find(|k| (k.source_sec - source_sec).abs() < SAME_KEY_SEC) {
        Some(key) => key.value = value,
        None => keys.push(Keyframe { id: new_id(), source_sec, value, ease: None }),
    }
    let mut next = item.clone();
    next.set_field(field, value);
    with_keys(next, field, keys)
}

pub fn edit_field<T: Keyed>(item: &T, field: AnimField, source_sec: f64, value: f64) -> T {
    if !keys_of(item, field).is_empty() {
        return set_key(item, field, source_sec, value);
    }
    let mut next = item.clone();
    next.set_field(field, value);
    next
}

pub fn key_at<T: Keyed>(item: &T, field: AnimField, source_sec: f64) -> Option<&Keyframe> {
    keys_of(item, field).iter().find(|k| (k.source_sec - source_sec).abs() < SAME_KEY_SEC)
}

fn fields_of<T: Keyed>(item: &T) -> Vec<AnimField> {
    item.keys().map(|k| k.keys().copied().collect()).unwrap_or_default()
}

pub fn remove_keys<T: Keyed>(item: &T, key_ids: &[String]) -> T {
    fields_of(item).into_iter().fold(item.clone(), |e, field| {
        let kept = keys_of(&e, field).iter().filter(|k| !key_ids.contains(&k.id)).cloned().collect();
        with_keys(e, field, kept)
    })
}

pub fn stop_animating<T: Keyed>(item: &T, field: AnimField, value: f64) -> T {
    let mut next = item.clone();
    next.set_field(field, value);
    with_keys(next, field, Vec::new())
}

pub fn map_keys<T: Keyed>(item: &T, map: &mut dyn FnMut(&Keyframe) -> Keyframe) -> T {
    fields_of(item).into_iter().fold(item.clone(), |e, field| {
        let mapped = keys_of(&e, field).iter().map(&mut *map).collect();
        with_keys(e, field, mapped)
    })
}

pub fn mirror_keys<T: Keyed>(item: &T, pivot_sec: f64) -> T {
    fields_of(item).into_iter().fold(item.clone(), |e, field| {
        let mut keys = keys_of(&e, field).to_vec();
        keys.sort_by(|a, b| b.source_sec.total_cmp(&a.source_sec));
        let flipped = (0..keys.len())
            .map(|i| Keyframe {
                id: keys[i].id.clone(),
                source_sec: pivot_sec - keys[i].source_sec,
                value: keys[i].value,
                ease: keys.get(i + 1).and_then(|k| k.ease).map(|e| [1.0 - e[2], 1.0 - e[3], 1.0 - e[0], 1.0 - e[1]]),
            })
            .collect();
        with_keys(e, field, flipped)
    })
}

pub fn clip_keys(clip: &Clip) -> Vec<Keyframe> {
    let mut all: Vec<Keyframe> = Vec::new();
    for effect in clip.effects() {
        all.extend(effect.keys.iter().flat_map(|k| k.values().flatten().cloned()));
    }
    if let Some(text) = &clip.text {
        all.extend(text.keys.iter().flat_map(|k| k.values().flatten().cloned()));
    }
    all
}

pub fn clip_animated(clip: &Clip) -> bool {
    is_animated(clip.effects()) || clip.text.as_ref().is_some_and(|t| is_animated(std::slice::from_ref(t)))
}

pub fn map_clip_keys(clip: &Clip, effect: &mut dyn FnMut(&Effect) -> Effect, text: &mut dyn FnMut(&TextStyle) -> TextStyle) -> Clip {
    let mut next = clip.clone();
    if let Some(effects) = &clip.effects {
        next.effects = Some(effects.iter().map(&mut *effect).collect());
    }
    if let Some(style) = &clip.text {
        next.text = Some(text(style));
    }
    next
}
