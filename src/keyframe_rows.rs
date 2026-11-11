use crate::effects::{animatable, label};
use crate::keyframes::{clip_animated, keys_of};
use crate::speed::local_at;
use crate::timeline::clip_duration;
use crate::types::{AnimField, Clip, Effect, EffectKind, Keyframe, Project, Track};

pub struct KeyRow {
    pub id: String,
    pub group: String,
    pub label: &'static str,
    pub field: AnimField,
    pub text: bool,
    pub keys: Vec<Keyframe>,
}

const TEXT_FIELDS: [(AnimField, &str); 3] = [(AnimField::X, "Left / right"), (AnimField::Y, "Up / down"), (AnimField::Size, "Size")];

pub fn field_label(effect: &Effect, field: AnimField) -> &'static str {
    match field {
        AnimField::Amount if effect.kind == EffectKind::ChromaKey => "Tolerance",
        AnimField::Amount if effect.kind == EffectKind::DropShadow => "Opacity",
        AnimField::Size if effect.height.is_some() => "Width",
        AnimField::Amount => "Amount",
        AnimField::Size => "Size",
        AnimField::Height => "Height",
        AnimField::X => "X",
        AnimField::Y => "Y",
        AnimField::Rotation => "Rotation",
        AnimField::Softness => "Softness",
        AnimField::Spill => "Spill",
        AnimField::Distance => "Distance",
        AnimField::Blur => "Blur",
        AnimField::Angle => "Angle",
    }
}

pub fn rows_of(clip: &Clip) -> Vec<KeyRow> {
    let text_rows = clip.text.iter().flat_map(|text| {
        TEXT_FIELDS.iter().filter_map(move |(field, name)| {
            let keys = keys_of(text, *field);
            (!keys.is_empty()).then(|| KeyRow { id: format!("text:{field:?}"), group: "Text".into(), label: name, field: *field, text: true, keys: keys.to_vec() })
        })
    });
    let effect_rows = clip.effects().iter().flat_map(|effect| {
        animatable(effect.kind).iter().filter_map(move |field| {
            let keys = keys_of(effect, *field);
            (!keys.is_empty()).then(|| KeyRow {
                id: format!("{}:{field:?}", effect.id),
                group: label(effect.kind).into(),
                label: field_label(effect, *field),
                field: *field,
                text: false,
                keys: keys.to_vec(),
            })
        })
    });
    text_rows.chain(effect_rows).collect()
}

pub fn format_value(row: &KeyRow, value: f64) -> String {
    match row.field {
        AnimField::Rotation | AnimField::Angle => format!("{}°", value.round()),
        AnimField::Size if row.text => format!("{}%", (value * 10.0).round() / 10.0),
        _ => format!("{}%", (value * 1000.0).round() / 10.0),
    }
}

pub fn key_fraction(clip: &Clip, key: &Keyframe) -> f32 {
    let length = clip_duration(clip);
    if length <= 0.0 { 0.0 } else { (local_at(clip, key.source_sec) / length).clamp(0.0, 1.0) as f32 }
}

pub fn playhead_fraction(clip: &Clip, playhead_sec: f64) -> Option<f32> {
    let (local, length) = (playhead_sec - clip.timeline_start_sec, clip_duration(clip));
    (length > 0.0 && (0.0..=length).contains(&local)).then(|| (local / length) as f32)
}

pub fn animated_layers<'a>(project: &'a Project, selection: &[String]) -> Vec<(&'a Track, &'a Clip)> {
    project.tracks.iter().flat_map(|t| t.clips.iter().filter(|c| selection.contains(&c.id) && clip_animated(c)).map(move |c| (t, c))).collect()
}

pub fn layer_name(clip: &Clip, project: &Project) -> String {
    if let Some(text) = &clip.text {
        return text.content.lines().next().filter(|l| !l.is_empty()).unwrap_or("Text").to_string();
    }
    if clip.adjustment {
        return "Adjustment".into();
    }
    project.assets.iter().find(|a| a.id == clip.asset_id).map_or("Clip".into(), |a| a.name.clone())
}
