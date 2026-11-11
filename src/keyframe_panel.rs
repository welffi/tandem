use crate::app::App;
use crate::curve_editor::{self, CurveState};
use crate::curve_presets;
use crate::format::format_clock;
use crate::inspector_clip::seek_source;
use crate::key_tags::{self, Tag};
use crate::keyframe_rows::{KeyRow, animated_layers, format_value, key_fraction, layer_name, playhead_fraction, rows_of};
use crate::speed::local_at;
use crate::store::key_ref;
use crate::theme::*;
use crate::timeline::clip_duration;
use crate::timeline_drag::move_keys;
use crate::timeline_draw::diamond;
use crate::timeline_ops::{keys_by_clip, pick_keys};
use crate::types::{Clip, Keyframe, Project};
use crate::widgets::truncated;
use eframe::egui::{Align2, Area, Color32, FontId, Id, Order, Rect, ScrollArea, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

const PAD: f32 = 4.0;
const MAX_ROWS: f32 = 5.0;
const MAX_SHARE: f32 = 0.4;
const ROW_H: f32 = 22.0;
const TITLE_H: f32 = 16.0;
const GROUP_W: f32 = 70.0;
const LABEL_W: f32 = 142.0;
const LANE_INSET: f32 = 14.0;
const KEY_R: f32 = 5.0;
const CURVE_W: f32 = 190.0;
const PRESETS_W: f32 = 204.0;
const POPOVER_W: f32 = CURVE_W + PRESETS_W;
const POPOVER_H: f32 = 240.0;
const POPOVER_GAP: f32 = 8.0;
const POPOVER_LIFT: f32 = 44.0;
const POPOVER_TOP_CLEAR: f32 = 44.0;

#[derive(Default)]
pub struct KeyframeState {
    drag: Option<KeyDrag>,
    pub curve: CurveState,
}

struct KeyDrag {
    base: Project,
    clip_id: String,
    key_ids: Vec<String>,
    down_x: f32,
    lane_w: f32,
    anchor_local: f64,
}

pub fn height(app: &App, upper_h: f32) -> f32 {
    let layers = animated_layers(app.store.project(), &app.store.selection);
    if layers.is_empty() {
        return 0.0;
    }
    let rows: usize = layers.iter().map(|(_, clip)| rows_of(clip).len()).sum();
    let titles = if layers.len() > 1 { layers.len() } else { 0 };
    let wanted = rows as f32 * ROW_H + titles as f32 * TITLE_H;
    (wanted.min(MAX_ROWS * ROW_H) + PAD * 2.0).min(upper_h * MAX_SHARE)
}

fn pressed(ui: &Ui) -> bool {
    ui.input(|i| i.pointer.primary_pressed())
}

fn grab(app: &mut App, clip: &Clip, key: &Keyframe, lane_w: f32, additive: bool, down_x: f32) {
    app.store.selected_keys = pick_keys(&app.store.selected_keys, &key_ref(&clip.id, &key.id), additive);
    seek_source(app, clip, key.source_sec);
    let key_ids = keys_by_clip(&app.store.selected_keys).remove(&clip.id).unwrap_or_default();
    app.keyframes.drag = Some(KeyDrag { base: app.store.project().clone(), clip_id: clip.id.clone(), key_ids, down_x, lane_w, anchor_local: local_at(clip, key.source_sec) });
}

fn drag(app: &mut App, clip: &Clip, x: f32) {
    let Some(d) = &app.keyframes.drag else { return };
    if d.clip_id != clip.id {
        return;
    }
    let length = clip_duration(clip);
    let delta = (x - d.down_x) as f64 / d.lane_w as f64 * length;
    let next = move_keys(&d.base, &d.clip_id, &d.key_ids, delta);
    let playhead = clip.timeline_start_sec + (d.anchor_local + delta).clamp(0.0, length);
    app.store.edit(next, Some("key-drag"));
    app.seek(playhead);
}

fn key_x(clip: &Clip, key: &Keyframe, track: Rect) -> f32 {
    track.left() + key_fraction(clip, key) * track.width()
}

fn key_button(ui: &mut Ui, app: &mut App, clip: &Clip, row: &KeyRow, key: &Keyframe, track: Rect) {
    let center = pos2(key_x(clip, key, track), track.center().y);
    let response = ui.interact(Rect::from_center_size(center, vec2(16.0, 18.0)), Id::unique(("kf-key", &clip.id, &key.id)), Sense::click_and_drag());
    let selected = app.store.selected_keys.contains(&key_ref(&clip.id, &key.id));
    let radius = if response.hovered() { KEY_R + 1.0 } else { KEY_R };
    ui.painter().add(Shape::convex_polygon(diamond(center, radius), if selected { Color32::WHITE } else { KEY_FILL }, Stroke::new(1.0, KEY_LINE)));
    let when = format_clock(clip.timeline_start_sec + local_at(clip, key.source_sec), false);
    let tip = format!("{when}: {}. Shift-click to add to the selection, drag to move.", format_value(row, key.value));
    response.clone().on_hover_text(tip);
    let pointer_x = ui.input(|i| i.pointer.interact_pos()).map_or(center.x, |p| p.x);
    if response.hovered() && pressed(ui) {
        let additive = ui.input(|i| i.modifiers.shift || i.modifiers.command);
        grab(app, clip, key, track.width(), additive, pointer_x);
    }
    if response.dragged() {
        drag(app, clip, pointer_x);
    }
}

fn scrub(ui: &mut Ui, app: &mut App, clip: &Clip, row: &KeyRow, lane: Rect, track: Rect) {
    let response = ui.interact(lane, Id::unique(("kf-lane", &clip.id, &row.id)), Sense::click_and_drag());
    if response.hovered() && pressed(ui) && !ui.input(|i| i.modifiers.shift) {
        app.store.selected_keys.clear();
    }
    if response.is_pointer_button_down_on()
        && let Some(at) = response.interact_pointer_pos()
    {
        let fraction = ((at.x - track.left()) / track.width()).clamp(0.0, 1.0);
        app.seek(clip.timeline_start_sec + clip_duration(clip) * fraction as f64);
    }
}

fn tags_of(app: &App, clip: &Clip, row: &KeyRow, track: Rect) -> Vec<Tag> {
    let selected = |key: &Keyframe| app.store.selected_keys.contains(&key_ref(&clip.id, &key.id));
    row.keys.iter().map(|key| Tag { x: key_x(clip, key, track), text: format_value(row, key.value), selected: selected(key) }).collect()
}

fn lane_row(ui: &mut Ui, app: &mut App, clip: &Clip, row: &KeyRow, first_in_group: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
    let lane = Rect::from_min_max(pos2(rect.left() + LABEL_W, rect.top() + 2.0), pos2(rect.right(), rect.bottom() - 2.0));
    let track = lane.shrink2(vec2(LANE_INSET, 0.0));
    let painter = ui.painter();
    if first_in_group {
        let group = Rect::from_min_max(rect.min, pos2(rect.left() + GROUP_W - 4.0, rect.bottom()));
        truncated(painter, group, Align2::LEFT_CENTER, &row.group, FontId::proportional(10.5), ACCENT_SOFT);
    }
    let label = Rect::from_min_max(pos2(rect.left() + GROUP_W, rect.top()), pos2(lane.left() - 6.0, rect.bottom()));
    truncated(painter, label, Align2::LEFT_CENTER, row.label, FontId::proportional(11.5), TEXT);
    painter.rect_filled(lane, 3.0, LANE_A);
    painter.hline(track.x_range(), lane.center().y, Stroke::new(1.0, RULER_TICK));
    if let Some(fraction) = playhead_fraction(clip, app.store.playhead_sec) {
        painter.vline(track.left() + fraction * track.width(), lane.y_range(), Stroke::new(1.0, PLAYHEAD));
    }
    scrub(ui, app, clip, row, lane, track);
    key_tags::draw(ui.painter(), lane.center().y, &tags_of(app, clip, row, track), lane.left(), lane.right());
    for key in &row.keys {
        key_button(ui, app, clip, row, key, track);
    }
}

fn clip_lanes(ui: &mut Ui, app: &mut App, title: Option<String>, clip: &Clip) {
    if let Some(title) = title {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), TITLE_H), Sense::hover());
        truncated(ui.painter(), rect, Align2::LEFT_CENTER, &title, FontId::proportional(10.5), DIM);
    }
    let mut group = String::new();
    for row in rows_of(clip) {
        let first_in_group = row.group != group;
        group = row.group.clone();
        lane_row(ui, app, clip, &row, first_in_group);
    }
}

fn lanes(ui: &mut Ui, app: &mut App, rect: Rect) {
    let project = app.store.project();
    let layers: Vec<(String, Clip)> = animated_layers(project, &app.store.selection).iter().map(|(t, c)| (format!("{}  {}", t.name, layer_name(c, project)), (*c).clone())).collect();
    let many = layers.len() > 1;
    let mut inner = ui.new_child(UiBuilder::new().max_rect(rect));
    ScrollArea::vertical().id_salt("keyframes").auto_shrink([false, false]).show(&mut inner, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for (title, clip) in &layers {
            clip_lanes(ui, app, many.then(|| title.clone()), clip);
        }
    });
}

fn popover(ui: &mut Ui, app: &mut App, panel: Rect) {
    let top = (panel.top() - POPOVER_H - POPOVER_LIFT).max(ui.max_rect().top() + POPOVER_TOP_CLEAR);
    let pos = pos2(panel.right() - POPOVER_W - POPOVER_GAP, top);
    Area::new(Id::unique("curve-popover")).order(Order::Foreground).fixed_pos(pos).show(&ui.ctx().clone(), |ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(POPOVER_W, POPOVER_H), Sense::click());
        ui.painter().rect(rect, 8.0, PANEL, Stroke::new(1.0, PILL_LINE), StrokeKind::Inside);
        let curve = Rect::from_min_size(rect.min, vec2(CURVE_W, POPOVER_H));
        ui.painter().vline(curve.right() + 0.5, rect.y_range().shrink(8.0), Stroke::new(1.0, BORDER));
        curve_editor::show(ui, app, curve);
        curve_presets::show(ui, app, Rect::from_min_max(pos2(curve.right() + 8.0, rect.top() + 8.0), rect.max - vec2(8.0, 8.0)));
    });
}

pub fn show(ui: &mut Ui, app: &mut App, rect: Rect) {
    if !ui.input(|i| i.pointer.any_down()) {
        app.keyframes.drag = None;
    }
    ui.painter().rect_filled(rect, 0.0, PANEL);
    ui.painter().hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, BORDER));
    lanes(ui, app, rect.shrink2(vec2(12.0, PAD)));
    if !app.store.selected_keys.is_empty() {
        popover(ui, app, rect);
    }
}
