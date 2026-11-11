use crate::app::App;
use crate::edits::{remove_track, update_track};
use crate::icons;
use crate::level_meter::{self, FLOOR_DB, to_db};
use crate::theme::*;
use crate::timeline_draw::Geo;
use crate::timeline_math::{HEADER_W, Lane};
use crate::timeline_state::TimelineState;
use crate::types::{Track, TrackKind};
use crate::widgets::{icon_button, slider};
use eframe::egui::{Align, Color32, FontId, Id, Layout, Rect, TextEdit, Ui, UiBuilder, pos2, vec2};

const HOT_DB: f32 = -0.05;

enum Action {
    Rename(String, String),
    Toggle(String),
    Remove(String),
    Volume(String, f64),
}

pub fn peak_note(peak: Option<f32>) -> Option<(String, bool)> {
    let db = to_db(peak?);
    if db <= FLOOR_DB { Some(("silent".into(), false)) } else { Some((format!("{db:.1} dB"), db > HOT_DB)) }
}

fn name_edit(ui: &mut Ui, track: &Track, width: f32, actions: &mut Vec<Action>) {
    let mut name = track.name.clone();
    let edit = TextEdit::singleline(&mut name).id(Id::unique(("track-name", &track.id))).desired_width(width).font(FontId::proportional(12.0));
    if ui.add(edit).changed() {
        actions.push(Action::Rename(track.id.clone(), name));
    }
}

fn toggle_button(ui: &mut Ui, track: &Track, actions: &mut Vec<Action>) {
    let off = if track.kind == TrackKind::Audio { track.muted } else { track.hidden };
    let (tip, kind) = (if off { "Turn on" } else { "Turn off" }, track.kind);
    let response = icon_button(ui, vec2(22.0, 22.0), true, |p, r, c| {
        let r = icons::centered(r, 14.0);
        if kind == TrackKind::Audio { icons::speaker(p, r, off, 1.0, c) } else { icons::eye(p, r, off, c) }
    });
    if response.on_hover_text(tip).clicked() {
        actions.push(Action::Toggle(track.id.clone()));
    }
}

fn remove_button(ui: &mut Ui, track: &Track, actions: &mut Vec<Action>) {
    if icon_button(ui, vec2(22.0, 22.0), true, |p, r, c| icons::cross(p, icons::centered(r, 12.0), c)).on_hover_text("Remove track").clicked() {
        actions.push(Action::Remove(track.id.clone()));
    }
}

fn mix_row(ui: &mut Ui, app: &mut App, state: &mut TimelineState, track: &Track, actions: &mut Vec<Action>) {
    let mut volume = track.volume;
    if slider(ui, &mut volume, 70.0).on_hover_text(format!("Volume {}%", (track.volume * 100.0).round())).changed() {
        actions.push(Action::Volume(track.id.clone(), volume));
    }
    if let Some((text, hot)) = peak_note(state.track_peak(&app.store, track)) {
        let color: Color32 = if hot { HOT } else { DIM };
        ui.label(eframe::egui::RichText::new(text).size(10.0).color(color));
    }
    let meter_rect = Rect::from_min_size(ui.cursor().min + vec2(0.0, 3.0), vec2(ui.available_width().max(0.0), 10.0));
    if let Some(meter) = app.meters.get_mut(&track.id) {
        level_meter::show(ui, meter_rect, meter, false, false);
    }
}

fn header(ui: &mut Ui, app: &mut App, state: &mut TimelineState, track: &Track, rect: Rect, actions: &mut Vec<Action>) {
    let inner = rect.shrink2(vec2(8.0, 4.0));
    let first = Rect::from_min_size(inner.min, vec2(inner.width(), 22.0));
    let mut row = ui.new_child(UiBuilder::new().max_rect(first).layout(Layout::left_to_right(Align::Center)));
    row.spacing_mut().item_spacing.x = 2.0;
    name_edit(&mut row, track, inner.width() - 50.0, actions);
    toggle_button(&mut row, track, actions);
    remove_button(&mut row, track, actions);
    if track.kind != TrackKind::Audio {
        return;
    }
    let second = Rect::from_min_size(pos2(inner.left(), first.bottom() + 4.0), vec2(inner.width(), 20.0));
    let mut mix = ui.new_child(UiBuilder::new().max_rect(second).layout(Layout::left_to_right(Align::Center)));
    mix.spacing_mut().item_spacing.x = 6.0;
    mix_row(&mut mix, app, state, track, actions);
}

fn apply(app: &mut App, actions: Vec<Action>) {
    for action in actions {
        match action {
            Action::Rename(id, name) => {
                let next = update_track(app.store.project(), &id, |t| Track { name: name.clone(), ..t.clone() });
                app.store.edit(next, Some(&format!("rename:{id}")));
            }
            Action::Toggle(id) => {
                let Some(track) = app.store.project().tracks.iter().find(|t| t.id == id).cloned() else { continue };
                if track.kind == TrackKind::Audio {
                    app.store.set_mix(&id, Some(!track.muted), None);
                } else {
                    let next = update_track(app.store.project(), &id, |t| Track { hidden: !t.hidden, ..t.clone() });
                    app.store.edit(next, None);
                }
            }
            Action::Remove(id) => {
                let next = remove_track(app.store.project(), &id);
                app.store.edit(next, None);
            }
            Action::Volume(id, volume) => app.store.set_mix(&id, None, Some(volume)),
        }
    }
}

pub fn show(ui: &mut Ui, app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane], tracks: &[Track]) {
    let column = Rect::from_min_max(pos2(geo.stage.left() - HEADER_W, geo.stage.top()), pos2(geo.stage.left(), geo.stage.bottom()));
    let mut actions = Vec::new();
    let mut clipped = ui.new_child(UiBuilder::new().max_rect(column));
    clipped.set_clip_rect(column);
    for lane in lanes {
        let rect = Rect::from_min_size(pos2(column.left(), geo.lane_top(lane)), vec2(HEADER_W, lane.h));
        if rect.bottom() < column.top() || rect.top() > column.bottom() {
            continue;
        }
        let fill = if lane.index % 2 == 0 { LANE_A } else { LANE_B };
        clipped.painter().rect_filled(rect, 0.0, fill);
        clipped.painter().hline(rect.x_range(), rect.bottom() - 0.5, eframe::egui::Stroke::new(1.0, HEADER_LINE));
        let Some(track) = tracks.get(lane.index) else { continue };
        header(&mut clipped, app, state, track, rect, &mut actions);
    }
    clipped.painter().vline(column.right() - 0.5, column.y_range(), eframe::egui::Stroke::new(1.0, HEADER_LINE));
    apply(app, actions);
}
