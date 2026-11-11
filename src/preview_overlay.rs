use crate::app::App;
use crate::preview_edit::{Target, TextGrab, TransformGrab, dragged_text, moved_effect, scaled_effect, text_grab, text_under, transform_grab, transform_target, with_effect, with_text_position};
use crate::preview_hit::{Spot, corners, spot_at};
use crate::theme::{ACCENT, SELECTION_DASH, TEXT};
use eframe::egui::{Context, CursorIcon, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

const HANDLE_SIZE: f32 = 8.0;

pub enum Session {
    Text(TextGrab),
    Move(TransformGrab),
    Scale { grab: TransformGrab, center: Pos2, start_dist: f32 },
}

enum Under {
    Text(String),
    Spot(Spot),
    Nothing,
}

fn target_rect(frame: Rect, target: &Target) -> Rect {
    let r = target.rect;
    Rect::from_min_size(pos2(frame.left() + r.x as f32 * frame.width(), frame.top() + r.y as f32 * frame.height()), vec2(r.w as f32 * frame.width(), r.h as f32 * frame.height()))
}

fn under(app: &App, frame: Rect, target: Option<&Target>, at: Pos2) -> Under {
    let unit = (((at.x - frame.left()) / frame.width()) as f64, ((at.y - frame.top()) / frame.height()) as f64);
    if let Some(id) = text_under(&app.preview.texts, app.preview.size, unit) {
        return Under::Text(id.to_string());
    }
    target.and_then(|t| spot_at(target_rect(frame, t), t.rot as f32, at)).map_or(Under::Nothing, Under::Spot)
}

pub fn show(ui: &mut Ui, app: &mut App, frame: Rect) {
    let response = ui.interact(frame, Id::unique("preview-frame"), Sense::click_and_drag());
    let target = if app.store.playing { None } else { transform_target(app.store.project(), &app.store.selection, app.store.playhead_sec) };
    paint(&ui.painter().with_clip_rect(frame.expand(HANDLE_SIZE)), app, frame, target.as_ref());
    let ctx = ui.ctx().clone();
    if response.drag_started() {
        begin(app, frame, target.as_ref(), &ctx);
    }
    if response.dragged() {
        drag(app, frame, &ctx);
    }
    if response.drag_stopped() {
        app.preview_drag = None;
    }
    if let (true, Some(at)) = (response.clicked(), response.interact_pointer_pos()) {
        click(app, frame, target.as_ref(), at);
    }
    if let Some(icon) = cursor(app, frame, target.as_ref(), response.hover_pos()) {
        ctx.set_cursor_icon(icon);
    }
}

fn click(app: &mut App, frame: Rect, target: Option<&Target>, at: Pos2) {
    match under(app, frame, target, at) {
        Under::Text(id) => app.store.set_selection(vec![id]),
        Under::Spot(_) => {}
        Under::Nothing => app.toggle_play(),
    }
}

fn begin(app: &mut App, frame: Rect, target: Option<&Target>, ctx: &Context) {
    let Some(origin) = ctx.input(|i| i.pointer.press_origin()) else { return };
    let playhead = app.store.playhead_sec;
    app.preview_drag = match under(app, frame, target, origin) {
        Under::Text(id) => {
            app.store.set_selection(vec![id.clone()]);
            text_grab(app.store.project(), &id, playhead).map(Session::Text)
        }
        Under::Spot(spot) => target.and_then(|t| {
            let grab = transform_grab(app.store.project(), &t.clip_id, playhead)?;
            let center = target_rect(frame, t).center();
            Some(if spot == Spot::Corner { Session::Scale { grab, center, start_dist: origin.distance(center).max(1.0) } } else { Session::Move(grab) })
        }),
        Under::Nothing => None,
    };
}

fn drag(app: &mut App, frame: Rect, ctx: &Context) {
    let (Some(session), Some(origin), Some(pointer)) = (&app.preview_drag, ctx.input(|i| i.pointer.press_origin()), ctx.input(|i| i.pointer.latest_pos())) else { return };
    let delta = (((pointer.x - origin.x) / frame.width()) as f64, ((pointer.y - origin.y) / frame.height()) as f64);
    let snap = !ctx.input(|i| i.modifiers.alt);
    let project = app.store.project();
    let (next, key) = match session {
        Session::Text(grab) => (with_text_position(project, grab, dragged_text(grab, delta, snap)), format!("text-move:{}", grab.clip_id)),
        Session::Move(grab) => (with_effect(project, &grab.clip_id, &moved_effect(grab, delta, snap)), format!("transform-drag:{}", grab.clip_id)),
        Session::Scale { grab, center, start_dist } => {
            (with_effect(project, &grab.clip_id, &scaled_effect(grab, (pointer.distance(*center) / start_dist) as f64)), format!("transform-drag:{}", grab.clip_id))
        }
    };
    app.store.edit(next, Some(&key));
    ctx.request_repaint();
}

fn cursor(app: &App, frame: Rect, target: Option<&Target>, hover: Option<Pos2>) -> Option<CursorIcon> {
    match &app.preview_drag {
        Some(Session::Scale { .. }) => return Some(CursorIcon::ResizeNwSe),
        Some(_) => return Some(CursorIcon::Move),
        None => {}
    }
    match under(app, frame, target, hover?) {
        Under::Text(_) | Under::Spot(Spot::Inside) => Some(CursorIcon::Move),
        Under::Spot(Spot::Corner) => Some(CursorIcon::ResizeNwSe),
        Under::Nothing => None,
    }
}

fn paint(painter: &Painter, app: &App, frame: Rect, target: Option<&Target>) {
    let (w, h) = (app.preview.size.0.max(1) as f32, app.preview.size.1.max(1) as f32);
    for (_, b) in app.preview.texts.iter().filter(|(id, _)| app.store.selection.contains(id)) {
        let min = pos2(frame.left() + b.x as f32 / w * frame.width(), frame.top() + b.y as f32 / h * frame.height());
        let bounds = Rect::from_min_size(min, vec2(b.w as f32 / w * frame.width(), b.h as f32 / h * frame.height()));
        let outline = [bounds.left_top(), bounds.right_top(), bounds.right_bottom(), bounds.left_bottom(), bounds.left_top()];
        painter.extend(Shape::dashed_line(&outline, Stroke::new(1.0, SELECTION_DASH), 6.0, 4.0));
    }
    let Some(target) = target else { return };
    let points = corners(target_rect(frame, target), target.rot as f32);
    painter.add(Shape::closed_line(points.to_vec(), Stroke::new(1.5, ACCENT)));
    for corner in points {
        painter.rect(Rect::from_center_size(corner, vec2(HANDLE_SIZE, HANDLE_SIZE)), 1.0, ACCENT, Stroke::new(1.0, TEXT), eframe::egui::StrokeKind::Middle);
    }
}
