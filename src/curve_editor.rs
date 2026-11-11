use crate::app::App;
use crate::curve_math::{CurveChange, dragged_handle, eased, from_screen, numbers, selected_ease, to_screen, unit_box_in, visible_y};
use crate::ids::new_id;
use crate::theme::*;
use crate::types::{Bezier, CurvePreset};
use crate::widgets::small_button;
use eframe::egui::epaint::CubicBezierShape;
use eframe::egui::{Align, Align2, Color32, FontId, Id, Key, Layout, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Vec2, pos2, vec2};

const MARGIN: Vec2 = vec2(14.0, 16.0);
const BELOW_VIEW: f32 = 6.0 + ROW_H + 4.0 + ROW_H + 8.0;
const HANDLE_R: f32 = 6.0;
const ROW_H: f32 = 22.0;
const NEW_NAME: &str = "My curve";

#[derive(Clone)]
pub struct Naming {
    pub id: Option<String>,
    pub name: String,
    pub asked: bool,
}

impl Naming {
    pub fn new(id: Option<String>, name: &str) -> Naming {
        Naming { id, name: name.into(), asked: false }
    }
}

#[derive(Default)]
pub struct CurveState {
    pub naming: Option<Naming>,
}

pub fn curve_shape(unit_box: Rect, bezier: Bezier, stroke: Stroke) -> Shape {
    let point = |x: f64, y: f64| to_screen(unit_box, (x, y));
    let [x1, y1, x2, y2] = bezier;
    Shape::CubicBezier(CubicBezierShape::from_points_stroke([point(0.0, 0.0), point(x1, y1), point(x2, y2), point(1.0, 1.0)], false, Color32::TRANSPARENT, stroke))
}

fn handle(ui: &mut Ui, view: Rect, unit_box: Rect, current: Bezier, index: usize) -> Option<Bezier> {
    let (anchor, tip) = if index == 0 { ((0.0, 0.0), (current[0], current[1])) } else { ((1.0, 1.0), (current[2], current[3])) };
    let (from, to) = (to_screen(unit_box, anchor), to_screen(unit_box, tip).clamp(view.min, view.max));
    ui.painter().line_segment([from, to], Stroke::new(1.0, DIM));
    let response = ui.interact(Rect::from_center_size(to, vec2(18.0, 18.0)), Id::unique(("curve-handle", index)), Sense::drag());
    let lit = response.hovered() || response.dragged();
    ui.painter().circle(to, HANDLE_R, if lit { Color32::WHITE } else { ACCENT }, Stroke::new(1.0, KEY_LINE));
    let at = response.interact_pointer_pos().filter(|_| response.dragged())?;
    Some(dragged_handle(current, index, from_screen(unit_box, at), ui.input(|i| i.modifiers.shift), visible_y(view, unit_box)))
}

fn child(ui: &mut Ui, rect: Rect) -> Ui {
    ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)))
}

fn finish_naming(app: &mut App, naming: &Naming, current: Bezier) {
    let name = naming.name.trim();
    if name.is_empty() {
        return;
    }
    let change = match &naming.id {
        Some(id) => CurveChange::Rename(id.clone(), name.into()),
        None => CurveChange::Add(CurvePreset { id: new_id(), name: name.into(), bezier: current }),
    };
    app.store_curves(&change);
}

fn naming_row(ui: &mut Ui, app: &mut App, rect: Rect, current: Bezier) {
    let Some(mut naming) = app.keyframes.curve.naming.clone() else { return };
    let mut row = child(ui, rect);
    row.label(RichText::new(if naming.id.is_some() { "Rename" } else { "Name" }).color(DIM));
    let response = row.add(TextEdit::singleline(&mut naming.name).id(Id::unique("curve-name")).desired_width(row.available_width()));
    if !naming.asked {
        response.request_focus();
        naming.asked = true;
    }
    if response.lost_focus() {
        if !row.input(|i| i.key_pressed(Key::Escape)) {
            finish_naming(app, &naming, current);
        }
        app.keyframes.curve.naming = None;
    } else {
        app.keyframes.curve.naming = Some(naming);
    }
}

pub fn show(ui: &mut Ui, app: &mut App, rect: Rect) {
    let refs = app.store.selected_keys.clone();
    let current = selected_ease(app.store.project(), &refs);
    ui.painter().text(rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Curve", FontId::proportional(11.0), DIM);
    let view = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 24.0), pos2(rect.right() - 10.0, rect.bottom() - BELOW_VIEW));
    let unit_box = unit_box_in(view, MARGIN);
    ui.painter().rect_filled(view, 4.0, PANEL_2);
    ui.painter().rect(unit_box, 0.0, LANE_A, Stroke::new(1.0, RULER_TICK), StrokeKind::Inside);
    ui.painter().add(curve_shape(unit_box, current, Stroke::new(2.0, ACCENT_SOFT)));
    let moved = (0..2).filter_map(|index| handle(ui, view, unit_box, current, index)).last();
    if let Some(ease) = moved {
        app.store.edit(eased(app.store.project(), &refs, ease), Some("curve"));
    }
    let controls = Rect::from_min_size(pos2(rect.left() + 10.0, view.bottom() + 6.0), vec2(rect.width() - 20.0, ROW_H));
    let mut row = child(ui, controls);
    row.label(RichText::new(numbers(current)).color(DIM).size(11.0));
    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if small_button(ui, "Save curve…").on_hover_text("Keep this curve as a preset").clicked() {
            app.keyframes.curve.naming = Some(Naming::new(None, NEW_NAME));
        }
    });
    naming_row(ui, app, controls.translate(vec2(0.0, ROW_H + 4.0)), current);
}
