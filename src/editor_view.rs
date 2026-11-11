use crate::app::App;
use crate::export_dialog;
use crate::format::format_clock;
use crate::icons;
use crate::inspector;
use crate::keyframe_panel;
use crate::level_meter;
use crate::media_bin;
use crate::mix::MASTER;
use crate::player_view::{fit, picture};
use crate::preview_overlay;
use crate::theme::*;
use crate::timeline_ops::ZOOM_STEP;
use crate::timeline_view;
use crate::types::{Project, TrackKind};
use crate::widgets::{Look, Pill, button, icon_button, pill};
use eframe::egui::{Align, Align2, FontId, Id, Layout, Rect, Stroke, TextEdit, Ui, UiBuilder, pos2, vec2};

const TOPBAR_H: f32 = 40.0;
const TOOLBAR_H: f32 = 34.0;
const TRANSPORT_H: f32 = 36.0;
const METER_W: f32 = 64.0;
const BIN_W: f32 = 260.0;
const INSPECTOR_W: f32 = 300.0;
const HINT: &str = "Click selects a clip and its partner. Alt+click picks one. Alt+drag copies.";

fn row(ui: &mut Ui, rect: Rect) -> Ui {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0))).layout(Layout::left_to_right(Align::Center)));
    child.spacing_mut().item_spacing.x = 6.0;
    child
}

fn band(ui: &Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, PANEL);
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, BORDER));
}

fn rename_project(app: &mut App, name: String) {
    let next = Project { name, ..app.store.project().clone() };
    app.store.edit(next, Some("rename-project"));
}

fn topbar(ui: &mut Ui, app: &mut App, rect: Rect) {
    band(ui, rect);
    let mut bar = row(ui, rect);
    if button(&mut bar, "← Player", Look::Plain, true).clicked() {
        app.leave_editor();
    }
    if button(&mut bar, "New", Look::Quiet, true).on_hover_text("New project (Ctrl+N)").clicked() {
        app.new_project();
    }
    if button(&mut bar, "Open", Look::Quiet, true).on_hover_text("Open a project (Ctrl+O)").clicked() {
        app.open_project(None);
    }
    if button(&mut bar, "Save", Look::Quiet, true).on_hover_text("Save (Ctrl+S)").clicked() {
        app.save_project(false);
    }
    if button(&mut bar, "Undo", Look::Quiet, app.store.can_undo()).on_hover_text("Undo (Ctrl+Z)").clicked() {
        app.store.undo();
    }
    if button(&mut bar, "Redo", Look::Quiet, app.store.can_redo()).on_hover_text("Redo (Ctrl+Y)").clicked() {
        app.store.redo();
    }
    let mut name = app.store.project().name.clone();
    if bar.add(TextEdit::singleline(&mut name).id(Id::unique("project-name")).desired_width(220.0)).changed() {
        rename_project(app, name);
    }
    if app.store.dirty {
        bar.colored_label(DIM, "•  unsaved");
    }
    let mut right = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0))).layout(Layout::right_to_left(Align::Center)));
    if button(&mut right, "Export", Look::Primary, true).on_hover_text("Export a video (Ctrl+E)").clicked() {
        app.store.export_open = true;
    }
}

fn toolbar(ui: &mut Ui, app: &mut App, rect: Rect) {
    band(ui, rect);
    let ready = app.store.duration() > 0.0;
    let has_selection = !app.store.selection.is_empty();
    let has_keys = !app.store.selected_keys.is_empty();
    let width = ui.available_width().min(rect.width());
    let mut bar = row(ui, rect);
    let snap = if app.store.snapping { Pill::On } else { Pill::Plain };
    if pill(&mut bar, "Snap", snap).on_hover_text("Snap to edges and the playhead (N)").clicked() {
        app.store.snapping = !app.store.snapping;
    }
    if button(&mut bar, "Split", Look::Plain, ready).on_hover_text("Split at the playhead (S)").clicked() {
        app.store.split_at_playhead();
    }
    if button(&mut bar, "Delete", Look::Plain, has_selection || has_keys).on_hover_text("Delete the selection (Del)").clicked() && !app.store.delete_selected_keys() {
        app.store.delete_selected_clips();
    }
    if button(&mut bar, "Ripple delete", Look::Plain, has_selection).on_hover_text("Delete and close the gap (Shift+Del)").clicked() {
        app.store.ripple_delete_selection();
    }
    if button(&mut bar, "Add text", Look::Plain, true).on_hover_text("Add text at the playhead (T)").clicked() {
        app.store.add_text_at_playhead();
    }
    if button(&mut bar, "Add adjustment", Look::Plain, true).on_hover_text("Add an adjustment layer at the playhead (A)").clicked() {
        app.store.add_adjustment_at_playhead();
    }
    let left_end = bar.min_rect().right();
    let mut zoom = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0))).layout(Layout::right_to_left(Align::Center)));
    zoom.spacing_mut().item_spacing.x = 6.0;
    let stage = app.store.stage_width;
    if button(&mut zoom, "Fit", Look::Plain, true).on_hover_text("Fit the whole project").clicked() {
        app.store.zoom_to_fit(stage);
    }
    if button(&mut zoom, "+", Look::Plain, true).on_hover_text("Zoom in (=)").clicked() {
        app.store.zoom_by(ZOOM_STEP, stage);
    }
    if button(&mut zoom, "−", Look::Plain, true).on_hover_text("Zoom out (-)").clicked() {
        app.store.zoom_by(1.0 / ZOOM_STEP, stage);
    }
    let gap = Rect::from_min_max(pos2(left_end + 12.0, rect.top()), pos2(zoom.min_rect().left() - 12.0, rect.bottom()));
    if width > 0.0 && gap.width() > 200.0 {
        crate::widgets::truncated(ui.painter(), gap, Align2::LEFT_CENTER, HINT, FontId::proportional(11.0), DIM);
    }
}

fn transport(ui: &mut Ui, app: &mut App, rect: Rect) {
    let mut bar = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)));
    bar.add_space(rect.width() / 2.0 - 90.0);
    let playing = app.store.playing;
    let ready = app.store.duration() > 0.0;
    if icon_button(&mut bar, vec2(40.0, 30.0), ready, |p, r, c| icons::play(p, icons::centered(r, 16.0), playing, c)).on_hover_text("Play / pause (Space)").clicked() {
        app.toggle_play();
    }
    let duration = app.store.duration();
    let clock = format!("{} / {}", format_clock(app.store.playhead_sec, duration >= 3600.0), format_clock(duration, false));
    bar.label(eframe::egui::RichText::new(clock).monospace().size(13.0));
}

fn preview(ui: &mut Ui, app: &mut App, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, BG);
    let stage = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - TRANSPORT_H)).shrink(6.0);
    let project = app.store.project();
    let (frame_w, frame_h) = (project.width as f32, project.height as f32);
    let has_picture = project.tracks.iter().any(|t| t.kind != TrackKind::Audio && t.clips.iter().any(|c| !t.hidden || c.adjustment));
    if app.store.duration() > 0.0 && has_picture {
        picture(ui, stage, app);
        preview_overlay::show(ui, app, fit(stage, frame_w, frame_h));
    }
    transport(ui, app, Rect::from_min_max(pos2(rect.left(), rect.bottom() - TRANSPORT_H), rect.max));
}

fn master(ui: &mut Ui, app: &mut App, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, PANEL);
    ui.painter().vline(rect.left() + 0.5, rect.y_range(), Stroke::new(1.0, HEADER_LINE));
    ui.painter().text(pos2(rect.center().x, rect.top() + 13.0), Align2::CENTER_CENTER, "Master", FontId::proportional(10.0), DIM);
    let meter_rect = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 30.0), pos2(rect.right() - 10.0, rect.bottom() - 10.0));
    if let Some(meter) = app.meters.get_mut(MASTER) {
        level_meter::show(ui, meter_rect, meter, true, true);
    }
}

pub fn show(ui: &mut Ui, app: &mut App) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BG);
    let remaining = rect.height() - TOPBAR_H - TOOLBAR_H;
    let timeline_h = (remaining * 0.45).max(200.0).min(remaining - 100.0).max(100.0);
    let top = Rect::from_min_size(rect.min, vec2(rect.width(), TOPBAR_H));
    let upper = Rect::from_min_max(pos2(rect.left(), top.bottom()), pos2(rect.right(), rect.bottom() - timeline_h - TOOLBAR_H));
    let bar = Rect::from_min_size(pos2(rect.left(), upper.bottom()), vec2(rect.width(), TOOLBAR_H));
    let lower = Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max);
    let bin = Rect::from_min_size(upper.min, vec2(BIN_W, upper.height()));
    let column = Rect::from_min_max(pos2(upper.left() + BIN_W, upper.top()), pos2(upper.right() - INSPECTOR_W, upper.bottom()));
    let keys_h = keyframe_panel::height(app, upper.height());
    let stage = Rect::from_min_max(column.min, pos2(column.right(), column.bottom() - keys_h));
    let keys = Rect::from_min_max(pos2(column.left(), stage.bottom()), column.max);
    let panel = Rect::from_min_max(pos2(upper.right() - INSPECTOR_W, upper.top()), upper.max);
    topbar(ui, app, top);
    media_bin::show(ui, app, bin);
    preview(ui, app, stage);
    if keys_h > 0.0 {
        keyframe_panel::show(ui, app, keys);
    }
    inspector::show(ui, app, panel);
    toolbar(ui, app, bar);
    let (timeline, meter) = (Rect::from_min_max(lower.min, pos2(lower.right() - METER_W, lower.bottom())), Rect::from_min_max(pos2(lower.right() - METER_W, lower.top()), lower.max));
    timeline_view::show(ui, app, timeline);
    master(ui, app, meter);
    media_bin::drag_chip(ui, app);
    if app.store.export_open {
        export_dialog::show(&ui.ctx().clone(), app);
    }
}
