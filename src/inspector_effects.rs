use crate::app::App;
use crate::edits::update_clips;
use crate::effects::{DEFAULT_KEY_COLOR, EFFECT_KINDS, label, new_effect, on_adjust, on_text};
use crate::ids::new_id;
use crate::inspector_clip::seek_source;
use crate::inspector_keys::{Outcome, Spot, spot};
use crate::inspector_ui::{Row, color_row, keyed_row, section, toggle};
use crate::keyframes::{effect_at, stop_animating};
use crate::types::{AnimField, Clip, Effect, EffectKind};
use crate::widgets::{row_label, small_button};
use eframe::egui::Ui;

pub enum Change {
    Replace(Effect, Option<String>),
    Move(String, isize),
    Remove(String),
    Add(Effect),
}

pub fn changed(list: &[Effect], change: &Change) -> Vec<Effect> {
    let mut next = list.to_vec();
    match change {
        Change::Replace(effect, _) => next.iter_mut().filter(|e| e.id == effect.id).for_each(|e| *e = effect.clone()),
        Change::Remove(id) => next.retain(|e| &e.id != id),
        Change::Add(effect) => next.push(effect.clone()),
        Change::Move(id, by) => {
            if let Some(from) = next.iter().position(|e| &e.id == id) {
                let to = from as isize + by;
                if (0..next.len() as isize).contains(&to) {
                    let item = next.remove(from);
                    next.insert(to as usize, item);
                }
            }
        }
    }
    next
}

struct Panel<'a> {
    clip: &'a Clip,
    spot: Spot,
    change: Option<Change>,
    seek: Option<f64>,
}

impl Panel<'_> {
    fn field(&mut self, ui: &mut Ui, effect: &Effect, row: Row, field: AnimField, fallback: f64) {
        match keyed_row(ui, &row, effect, field, &self.spot, fallback) {
            Some(Outcome::Item(next)) => self.change = Some(Change::Replace(next, Some(format!("fx:{}:{:?}", effect.id, field)))),
            Some(Outcome::Seek(source_sec)) => self.seek = Some(source_sec),
            None => {}
        }
    }

    fn color(&mut self, ui: &mut Ui, effect: &Effect, name: &str, fallback: &str) {
        let current = effect.color.clone().unwrap_or_else(|| fallback.into());
        if let Some(color) = color_row(ui, name, &current, fallback) {
            self.change = Some(Change::Replace(Effect { color: Some(color), ..effect.clone() }, Some(format!("fx:{}:color", effect.id))));
        }
    }

    fn uniform(&mut self, ui: &mut Ui, effect: &Effect) {
        ui.horizontal(|ui| {
            row_label(ui, "Scale");
            let tip = "Keep width and height scaled together";
            if let Some(on) = toggle(ui, "Uniform", effect.height.is_none(), tip) {
                let next = if on {
                    Effect { height: None, ..stop_animating(effect, AnimField::Height, 1.0) }
                } else {
                    Effect { height: Some(effect_at(effect, self.spot.source_sec).size.unwrap_or(1.0)), ..effect.clone() }
                };
                self.change = Some(Change::Replace(next, None));
            }
        });
    }

    fn transform(&mut self, ui: &mut Ui, effect: &Effect) {
        self.uniform(ui, effect);
        let uniform = effect.height.is_none();
        self.field(ui, effect, Row::percent(if uniform { "Size" } else { "Width" }, 10.0, 400.0, 1.0), AnimField::Size, 1.0);
        if !uniform {
            self.field(ui, effect, Row::percent("Height", 10.0, 400.0, 1.0), AnimField::Height, 1.0);
        }
        self.field(ui, effect, Row::percent("X", -100.0, 100.0, 0.0), AnimField::X, 0.0);
        self.field(ui, effect, Row::percent("Y", -100.0, 100.0, 0.0), AnimField::Y, 0.0);
        self.field(ui, effect, Row::plain("Rotation", -180.0, 180.0, 0.0, "°"), AnimField::Rotation, 0.0);
    }

    fn chroma_key(&mut self, ui: &mut Ui, effect: &Effect) {
        self.color(ui, effect, "Key color", DEFAULT_KEY_COLOR);
        self.field(ui, effect, Row::percent("Tolerance", 0.0, 100.0, 0.3), AnimField::Amount, 0.3);
        self.field(ui, effect, Row::percent("Softness", 0.0, 100.0, 0.3), AnimField::Softness, 0.3);
        self.field(ui, effect, Row::percent("Spill", 0.0, 100.0, 0.5), AnimField::Spill, 0.5);
    }

    fn drop_shadow(&mut self, ui: &mut Ui, effect: &Effect) {
        self.color(ui, effect, "Color", "#000000");
        self.field(ui, effect, Row::percent("Opacity", 0.0, 100.0, 0.75), AnimField::Amount, 0.75);
        self.field(ui, effect, Row::percent("Distance", 0.0, 10.0, 0.01).step(0.1), AnimField::Distance, 0.01);
        self.field(ui, effect, Row::percent("Blur", 0.0, 10.0, 0.015).step(0.1), AnimField::Blur, 0.015);
        self.field(ui, effect, Row::plain("Angle", -180.0, 180.0, 45.0, "°"), AnimField::Angle, 45.0);
    }

    fn header(&mut self, ui: &mut Ui, effect: &Effect, index: usize, count: usize) {
        ui.horizontal(|ui| {
            if let Some(on) = toggle(ui, label(effect.kind), effect.enabled, "Turn this effect on or off") {
                self.change = Some(Change::Replace(Effect { enabled: on, ..effect.clone() }, None));
            }
            ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                if small_button(ui, "Remove").clicked() {
                    self.change = Some(Change::Remove(effect.id.clone()));
                }
                ui.add_enabled_ui(index + 1 < count, |ui| {
                    if small_button(ui, "Down").clicked() {
                        self.change = Some(Change::Move(effect.id.clone(), 1));
                    }
                });
                ui.add_enabled_ui(index > 0, |ui| {
                    if small_button(ui, "Up").clicked() {
                        self.change = Some(Change::Move(effect.id.clone(), -1));
                    }
                });
            });
        });
    }

    fn effect(&mut self, ui: &mut Ui, effect: &Effect, index: usize, count: usize) {
        self.header(ui, effect, index, count);
        match effect.kind {
            EffectKind::Transform => self.transform(ui, effect),
            EffectKind::ChromaKey => self.chroma_key(ui, effect),
            EffectKind::DropShadow => self.drop_shadow(ui, effect),
            EffectKind::Grayscale => self.field(ui, effect, Row::percent("Amount", 0.0, 100.0, 1.0), AnimField::Amount, 1.0),
        }
    }
}

fn addable(kind: EffectKind, text: bool, adjustment: bool) -> bool {
    (!text || on_text(kind)) && (!adjustment || on_adjust(kind))
}

pub fn show(ui: &mut Ui, app: &mut App, clip: &Clip, ids: &[String], text: bool) {
    section(ui, "Effects");
    let mut panel = Panel { clip, spot: spot(clip, app.store.playhead_sec), change: None, seek: None };
    let effects = clip.effects.clone().unwrap_or_default();
    for (index, effect) in effects.iter().enumerate() {
        ui.add_space(4.0);
        panel.effect(ui, effect, index, effects.len());
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for kind in EFFECT_KINDS.into_iter().filter(|k| addable(*k, text, clip.adjustment)) {
            if small_button(ui, &format!("+ {}", label(kind))).clicked() {
                panel.change = Some(Change::Add(new_effect(kind, new_id())));
            }
        }
    });
    if let Some(source_sec) = panel.seek {
        seek_source(app, panel.clip, source_sec);
    }
    if let Some(change) = panel.change {
        let key = if let Change::Replace(_, key) = &change { key.clone() } else { None };
        let next = update_clips(app.store.project(), ids, |c| {
            let list = changed(c.effects.as_deref().unwrap_or(&[]), &change);
            Clip { effects: (!list.is_empty()).then_some(list), ..c.clone() }
        });
        app.store.edit(next, key.as_deref());
    }
}
