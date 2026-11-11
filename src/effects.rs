use crate::keyframes::effect_at;
use crate::speed::source_at;
use crate::types::{AnimField, Clip, Effect, EffectKind};

pub const DEFAULT_KEY_COLOR: &str = "#00ff00";
const MAX_SIMILARITY: f64 = 0.4;
const MAX_BLEND: f64 = 0.3;
pub const MIN_SIZE: f64 = 0.05;
pub const MAX_SIZE: f64 = 5.0;

pub const EFFECT_KINDS: [EffectKind; 4] = [EffectKind::Grayscale, EffectKind::Transform, EffectKind::ChromaKey, EffectKind::DropShadow];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Green,
    Blue,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromaKey {
    pub rgb: [f64; 3],
    pub similarity: f64,
    pub blend: f64,
    pub spill: f64,
    pub screen: Option<Screen>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub sx: f64,
    pub sy: f64,
    pub x: f64,
    pub y: f64,
    pub rot: f64,
}

pub const IDENTITY: Transform = Transform { sx: 1.0, sy: 1.0, x: 0.0, y: 0.0, rot: 0.0 };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropShadow {
    pub dx: f64,
    pub dy: f64,
    pub blur: f64,
    pub rgb: [u8; 3],
    pub opacity: f64,
}

pub fn hex_rgb(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let part = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
    Some([part(0), part(2), part(4)])
}

pub fn rgb_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

pub fn is_hex_color(text: &str) -> bool {
    text.starts_with('#') && hex_rgb(text).is_some()
}

pub fn chroma_key_of(effect: &Effect) -> ChromaKey {
    let rgb = effect.color.as_deref().and_then(hex_rgb).unwrap_or([0, 255, 0]).map(|c| c as f64 / 255.0);
    let [r, g, b] = rgb;
    ChromaKey {
        rgb,
        similarity: (effect.amount * MAX_SIMILARITY).max(0.001),
        blend: effect.softness.unwrap_or(0.3) * MAX_BLEND,
        spill: effect.spill.unwrap_or(0.5),
        screen: if g >= r && g >= b {
            Some(Screen::Green)
        } else if b > r {
            Some(Screen::Blue)
        } else {
            None
        },
    }
}

pub fn is_keyed(effects: &[Effect]) -> bool {
    effects.iter().any(|e| e.kind == EffectKind::ChromaKey)
}

pub fn label(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Grayscale => "Grayscale",
        EffectKind::Transform => "Transform",
        EffectKind::ChromaKey => "Chroma key",
        EffectKind::DropShadow => "Drop shadow",
    }
}

pub fn animatable(kind: EffectKind) -> &'static [AnimField] {
    use AnimField::*;
    match kind {
        EffectKind::Grayscale => &[Amount],
        EffectKind::Transform => &[Size, Height, X, Y, Rotation],
        EffectKind::ChromaKey => &[Amount, Softness, Spill],
        EffectKind::DropShadow => &[Amount, Distance, Blur, Angle],
    }
}

pub fn on_text(kind: EffectKind) -> bool {
    matches!(kind, EffectKind::Grayscale | EffectKind::DropShadow)
}

pub fn on_adjust(kind: EffectKind) -> bool {
    matches!(kind, EffectKind::Grayscale | EffectKind::Transform)
}

pub fn new_effect(kind: EffectKind, id: String) -> Effect {
    let mut effect = Effect {
        id,
        kind,
        enabled: true,
        amount: 1.0,
        size: None,
        height: None,
        x: None,
        y: None,
        rotation: None,
        color: None,
        softness: None,
        spill: None,
        distance: None,
        blur: None,
        angle: None,
        keys: None,
    };
    match kind {
        EffectKind::ChromaKey => {
            effect.color = Some(DEFAULT_KEY_COLOR.into());
            effect.amount = 0.3;
            effect.softness = Some(0.3);
            effect.spill = Some(0.5);
        }
        EffectKind::DropShadow => {
            effect.color = Some("#000000".into());
            effect.amount = 0.75;
            effect.distance = Some(0.01);
            effect.blur = Some(0.015);
            effect.angle = Some(45.0);
        }
        _ => {}
    }
    effect
}

pub fn drop_shadow_of(effect: &Effect, frame_h: f64) -> DropShadow {
    let rgb = effect.color.as_deref().filter(|c| c.starts_with('#')).and_then(hex_rgb).unwrap_or([0, 0, 0]);
    let rad = effect.angle.unwrap_or(45.0).to_radians();
    let distance = effect.distance.unwrap_or(0.01) * frame_h;
    DropShadow { dx: rad.cos() * distance, dy: rad.sin() * distance, blur: (effect.blur.unwrap_or(0.015) * frame_h).max(0.0), rgb, opacity: effect.amount.min(1.0) }
}

pub fn active_effects(clip: &Clip, local_sec: Option<f64>) -> Vec<Effect> {
    let on = clip.effects().iter().filter(|e| e.enabled && (e.amount > 0.0 || e.keys.as_ref().is_some_and(|k| k.get(&AnimField::Amount).is_some_and(|v| !v.is_empty()))));
    match local_sec {
        None => on.cloned().collect(),
        Some(local) => {
            let source = source_at(clip, local);
            on.map(|e| effect_at(e, source)).collect()
        }
    }
}

fn clamp_size(size: f64) -> f64 {
    size.clamp(MIN_SIZE, MAX_SIZE)
}

pub fn transform_of(effects: &[Effect]) -> Transform {
    effects.iter().filter(|e| e.kind == EffectKind::Transform).fold(IDENTITY, |t, e| Transform {
        sx: clamp_size(t.sx * e.size.unwrap_or(1.0)),
        sy: clamp_size(t.sy * e.height.or(e.size).unwrap_or(1.0)),
        x: t.x + e.x.unwrap_or(0.0),
        y: t.y + e.y.unwrap_or(0.0),
        rot: t.rot + e.rotation.unwrap_or(0.0),
    })
}

pub fn grayscale_amount(effects: &[Effect]) -> f64 {
    effects.iter().filter(|e| e.kind == EffectKind::Grayscale).fold(0.0, |total: f64, e| 1.0 - (1.0 - total) * (1.0 - e.amount.clamp(0.0, 1.0)))
}
