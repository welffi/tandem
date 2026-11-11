use crate::app::App;
use crate::curve_editor::{Naming, curve_shape};
use crate::curve_math::{CurveChange, eased, same_curve, selected_ease};
use crate::keyframes::built_in_curves;
use crate::theme::*;
use crate::types::{Bezier, CurvePreset};
use crate::widgets::truncated;
use eframe::egui::{Align2, FontId, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, vec2};

const CELL: Vec2 = vec2(58.0, 52.0);
const SHAPE: f32 = 24.0;

fn thumb(ui: &mut Ui, preset: &CurvePreset, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(CELL, Sense::click());
    let line = if on {
        ACCENT
    } else if response.hovered() {
        PILL_LINE
    } else {
        BORDER
    };
    ui.painter().rect(rect.shrink(1.0), 6.0, if response.hovered() { HOVER } else { PANEL_2 }, Stroke::new(1.0, line), StrokeKind::Inside);
    let unit_box = Rect::from_center_size(rect.center_top() + vec2(0.0, 6.0 + SHAPE / 2.0), vec2(SHAPE, SHAPE));
    ui.painter().rect_stroke(unit_box, 0.0, Stroke::new(1.0, RULER_TICK), StrokeKind::Inside);
    ui.painter().add(curve_shape(unit_box, preset.bezier, Stroke::new(1.5, if on { ACCENT_SOFT } else { TEXT })));
    let label = Rect::from_min_max(rect.left_bottom() + vec2(3.0, -16.0), rect.right_bottom() + vec2(-3.0, -3.0));
    truncated(ui.painter(), label, Align2::CENTER_CENTER, &preset.name, FontId::proportional(9.5), DIM);
    response
}

pub fn show(ui: &mut Ui, app: &mut App, rect: Rect) {
    let refs = app.store.selected_keys.clone();
    let current = selected_ease(app.store.project(), &refs);
    let saved = app.store.library.curves.clone().unwrap_or_default();
    let mut pick: Option<Bezier> = None;
    let mut change: Option<CurveChange> = None;
    let mut rename: Option<Naming> = None;
    let mut body = ui.new_child(UiBuilder::new().max_rect(rect));
    ScrollArea::vertical().id_salt("curve-presets").auto_shrink([false, false]).show(&mut body, |ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        ui.horizontal_wrapped(|ui| {
            for preset in built_in_curves().iter().chain(&saved) {
                let response = thumb(ui, preset, same_curve(preset.bezier, current));
                if response.clicked() {
                    pick = Some(preset.bezier);
                }
                if saved.iter().any(|s| s.id == preset.id) {
                    response.on_hover_text(format!("{} (right-click to rename or delete)", preset.name)).context_menu(|ui| {
                        if ui.button("Rename").clicked() {
                            rename = Some(Naming::new(Some(preset.id.clone()), &preset.name));
                            ui.close();
                        }
                        if ui.button("Delete").clicked() {
                            change = Some(CurveChange::Delete(preset.id.clone()));
                            ui.close();
                        }
                    });
                } else {
                    response.on_hover_text(&preset.name);
                }
            }
        });
    });
    if let Some(ease) = pick {
        let next = eased(app.store.project(), &refs, ease);
        app.store.edit(next, None);
    }
    if let Some(change) = change {
        app.store_curves(&change);
    }
    if rename.is_some() {
        app.keyframes.curve.naming = rename;
    }
}
