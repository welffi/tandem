use crate::app::App;
use crate::edits::retime_clips;
use crate::format::format_clock;
use crate::inspector_ui::{hint, number_row, section, toggle};
use crate::speed::{MAX_SPEED, MIN_SPEED, clamp_speed, sorted_keys, source_at, speed_at};
use crate::timeline::clip_duration;
use crate::types::{Clip, SpeedKey};
use crate::widgets::{row_label, small_button};
use eframe::egui::{DragValue, Ui};

const SAME_KEY_SEC: f64 = 0.01;

pub fn single_speed(keys: &[SpeedKey], source_in_sec: f64, speed: f64) -> Vec<SpeedKey> {
    if speed == 1.0 { vec![] } else { vec![SpeedKey { source_sec: keys.first().map_or(source_in_sec, |k| k.source_sec), speed: clamp_speed(speed) }] }
}

pub fn with_key(keys: &[SpeedKey], key: SpeedKey) -> Vec<SpeedKey> {
    let mut next: Vec<SpeedKey> = keys.iter().filter(|k| (k.source_sec - key.source_sec).abs() > SAME_KEY_SEC).copied().collect();
    next.push(key);
    next.sort_by(|a, b| a.source_sec.total_cmp(&b.source_sec));
    next
}

fn retime(app: &mut App, ids: &[String], keys: Vec<SpeedKey>) {
    let next = retime_clips(app.store.project(), ids, |c| Clip { speed_keys: (!keys.is_empty()).then(|| keys.clone()), ..c.clone() });
    app.store.edit(next, None);
}

fn ramp_row(ui: &mut Ui, app: &mut App, ids: &[String], clip: &Clip, keys: &[SpeedKey], index: usize) {
    let key = keys[index];
    let mut speed = key.speed;
    let mut remove = false;
    ui.horizontal(|ui| {
        row_label(ui, &format_clock(clip.timeline_start_sec + crate::speed::local_at(clip, key.source_sec), false));
        ui.add(DragValue::new(&mut speed).range(MIN_SPEED..=MAX_SPEED).speed(0.1).max_decimals(2).suffix("×"));
        remove = small_button(ui, "×").on_hover_text("Remove this speed key").clicked();
    });
    if remove {
        retime(app, ids, keys.iter().enumerate().filter(|(i, _)| *i != index).map(|(_, k)| *k).collect());
    } else if speed != key.speed {
        retime(app, ids, keys.iter().enumerate().map(|(i, k)| if i == index { SpeedKey { speed: clamp_speed(speed), ..*k } } else { *k }).collect());
    }
}

fn buttons(ui: &mut Ui, app: &mut App, ids: &[String], clip: &Clip, keys: &[SpeedKey]) {
    let local = app.store.playhead_sec - clip.timeline_start_sec;
    let inside = (0.0..=clip_duration(clip)).contains(&local);
    ui.horizontal_wrapped(|ui| {
        ui.add_enabled_ui(inside, |ui| {
            if small_button(ui, "Add key at playhead").on_hover_text("Add a speed key where the playhead is. Two or more keys make a ramp.").clicked() {
                retime(app, ids, with_key(keys, SpeedKey { source_sec: source_at(clip, local), speed: speed_at(clip, local) }));
            }
        });
        if !keys.is_empty() && small_button(ui, "Reset").clicked() {
            retime(app, ids, vec![]);
        }
        if let Some(on) = toggle(ui, "Keep pitch", clip.keep_pitch, "Keep the audio pitch when the speed changes") {
            let next = retime_clips(app.store.project(), ids, |c| Clip { keep_pitch: on, ..c.clone() });
            app.store.edit(next, None);
        }
    });
}

pub fn show(ui: &mut Ui, app: &mut App, clip: &Clip, ids: &[String]) {
    section(ui, "Speed");
    let keys = sorted_keys(clip);
    if keys.len() > 1 {
        for index in 0..keys.len() {
            ramp_row(ui, app, ids, clip, &keys, index);
        }
    } else if let Some(speed) = number_row(ui, "Speed", keys.first().map_or(1.0, |k| k.speed), (MIN_SPEED, MAX_SPEED), 0.1, "×") {
        retime(app, ids, single_speed(&keys, clip.source_in_sec, speed));
    }
    buttons(ui, app, ids, clip, &keys);
    if keys.len() > 1 {
        hint(ui, "Speeds blend smoothly between the keys.");
    }
}
