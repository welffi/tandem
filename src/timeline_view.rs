use crate::app::App;
use crate::store::stream_key;
use crate::theme::*;
use crate::timeline::project_duration;
use crate::timeline_assets;
use crate::timeline_draw::{ClipJob, ClipLook, ClipParts, Geo, draw_clip};
use crate::timeline_hit::hit_test;
use crate::timeline_input::{Pointer, cursor_for, drag, press, press_ruler, release};
use crate::timeline_math::{HEADER_W, Lane, RULER_H, follow_scroll, layout_lanes, ruler_label, ruler_step, zoom_about};
use crate::timeline_menu;
use crate::timeline_state::TimelineState;
use crate::track_headers;
use crate::types::{Project, TrackKind};
use crate::widgets::small_button;
use eframe::egui::{Align, Align2, FontId, Id, Layout, Painter, Rect, Sense, Shape, Stroke, Ui, UiBuilder, Vec2, pos2, vec2};

fn background(painter: &Painter, geo: &Geo, lanes: &[Lane]) {
    painter.rect_filled(geo.stage, 0.0, LANE_B);
    for lane in lanes {
        let top = geo.lane_top(lane);
        let rect = Rect::from_min_size(pos2(geo.stage.left(), top), vec2(geo.stage.width(), lane.h));
        painter.rect_filled(rect, 0.0, if lane.index % 2 == 0 { LANE_A } else { LANE_B });
        painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, HEADER_LINE));
    }
}

fn ruler(painter: &Painter, geo: &Geo, ruler: Rect) {
    painter.rect_filled(ruler, 0.0, RULER_BG);
    let step = ruler_step(geo.px);
    let end = geo.time_at(ruler.right());
    let mut t = (geo.scroll_sec / step).floor() * step;
    let font = FontId::proportional(10.0);
    let tick = Stroke::new(1.0, RULER_TICK);
    while t <= end {
        let x = geo.x_at(t);
        painter.vline(x, (ruler.bottom() - 9.0)..=ruler.bottom(), tick);
        painter.text(pos2(x + 4.0, ruler.top() + 6.0), Align2::LEFT_TOP, ruler_label(t, step), font.clone(), RULER_TEXT);
        for i in 1..5 {
            painter.vline(geo.x_at(t + step * i as f64 / 5.0), (ruler.bottom() - 4.0)..=ruler.bottom(), tick);
        }
        t += step;
    }
    painter.hline(ruler.x_range(), ruler.bottom() - 0.5, Stroke::new(1.0, HEADER_LINE));
}

fn clips(painter: &Painter, ctx: &eframe::egui::Context, app: &App, state: &mut TimelineState, project: &Project, geo: &Geo, lanes: &[Lane]) {
    let selected: Vec<String> = match &state.drag {
        Some(d) => crate::timeline_drag::dragged_ids(d).unwrap_or(&app.store.selection).to_vec(),
        None => app.store.selection.clone(),
    };
    for lane in lanes {
        let Some(track) = project.tracks.get(lane.index) else { continue };
        let top = geo.lane_top(lane);
        if top + lane.h < geo.stage.top() || top > geo.stage.bottom() {
            continue;
        }
        for clip in &track.clips {
            let key = stream_key(&clip.asset_id, clip.stream.unwrap_or(0));
            let asset = project.assets.iter().find(|a| a.id == clip.asset_id);
            let parts = ClipParts { asset, thumbs: app.store.thumbs.get(&clip.asset_id), peaks: app.store.peaks.get(&key), peaks_key: key };
            let is_selected = selected.contains(&clip.id);
            let hovered = state.hover_clip.as_deref() == Some(clip.id.as_str());
            let look = ClipLook { selected: is_selected, dim: track.hidden || track.muted, handles: is_selected || hovered, linked: clip.group_id.is_some() };
            draw_clip(painter, state, ctx, ClipJob { geo, lane, track, clip, parts, look, selected_keys: &app.store.selected_keys });
        }
    }
}

fn markers(painter: &Painter, geo: &Geo, ruler: Rect, project: &Project, playhead: f64, snap: Option<f64>) {
    let end_x = geo.x_at(project_duration(project));
    if end_x < geo.stage.right() {
        painter.rect_filled(Rect::from_min_max(pos2(end_x.max(geo.stage.left()), geo.stage.top()), geo.stage.max), 0.0, END_MARK);
    }
    let span = ruler.top()..=geo.stage.bottom();
    if let Some(sec) = snap {
        painter.vline(geo.x_at(sec), span.clone(), Stroke::new(1.0, SNAP_LINE));
    }
    let x = geo.x_at(playhead);
    if x < geo.stage.left() - 1.0 || x > geo.stage.right() + 1.0 {
        return;
    }
    painter.vline(x, span, Stroke::new(1.5, PLAYHEAD));
    painter.add(Shape::convex_polygon(vec![pos2(x - 6.0, ruler.top() + 1.0), pos2(x + 6.0, ruler.top() + 1.0), pos2(x, ruler.top() + 12.0)], PLAYHEAD, Stroke::NONE));
}

fn corner(ui: &mut Ui, app: &mut App, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, RULER_BG);
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, HEADER_LINE));
    let mut row = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(8.0, 2.0))).layout(Layout::left_to_right(Align::Center)));
    row.spacing_mut().item_spacing.x = 4.0;
    for (label, tip, kind) in [("+ Video", "Add a video track", TrackKind::Video), ("+ Audio", "Add an audio track", TrackKind::Audio), ("+ Text", "Add a text track", TrackKind::Text)] {
        if small_button(&mut row, label).on_hover_text(tip).clicked() {
            app.store.add_track_of(kind);
        }
    }
}

fn scroll_and_zoom(ctx: &eframe::egui::Context, app: &mut App, state: &mut TimelineState, area: Rect, body: Rect, max_scroll: f32) {
    let (zoom, scroll, pointer) = ctx.input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.pointer.hover_pos()));
    let Some(p) = pointer.filter(|p| area.contains(*p)) else { return };
    let store = &mut app.store;
    if zoom != 1.0 {
        (store.px_per_sec, store.scroll_sec) = zoom_about(store.scroll_sec, store.px_per_sec, (p.x - body.left()) as f64, store.px_per_sec * zoom as f64);
    } else if scroll != Vec2::ZERO {
        if p.x < body.left() {
            state.v_scroll = (state.v_scroll - scroll.y).clamp(0.0, max_scroll);
        } else {
            store.scroll_sec = (store.scroll_sec - (scroll.x + scroll.y) as f64 / store.px_per_sec).max(0.0);
        }
    }
}

fn follow(app: &mut App, state: &mut TimelineState, width: f64) {
    let store = &mut app.store;
    let moved = (store.playhead_sec - state.followed).abs() > 1e-9;
    state.followed = store.playhead_sec;
    if let Some(scroll) = follow_scroll(store.scroll_sec, store.px_per_sec, width, store.playhead_sec, store.playing, moved) {
        store.scroll_sec = scroll;
    }
}

fn input(ui: &mut Ui, app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane], ruler_rect: Rect) {
    let ctx = ui.ctx().clone();
    let on_body = ui.interact(geo.stage, Id::unique("timeline-body"), Sense::click_and_drag());
    let on_ruler = ui.interact(ruler_rect, Id::unique("timeline-ruler"), Sense::click_and_drag());
    let (at, pressed, down, modifiers) = ctx.input(|i| (i.pointer.interact_pos(), i.pointer.primary_pressed(), i.pointer.primary_down(), i.modifiers));
    let Some(at) = at else {
        if !down {
            release(app, state);
        }
        return;
    };
    let pointer = Pointer { at, modifiers };
    if pressed && state.drag.is_none() {
        if on_ruler.hovered() {
            press_ruler(app, state, geo, at);
        } else if on_body.hovered() {
            press(app, state, geo, lanes, &pointer);
        }
    }
    timeline_menu::show(app, state, geo, lanes, &on_body);
    if state.drag.is_some() {
        if down {
            drag(app, state, geo, lanes, &pointer);
            ctx.set_cursor_icon(cursor_for(&crate::timeline_hit::Hit::Empty, state.drag.as_ref()));
        } else {
            release(app, state);
        }
        return;
    }
    let hit = if on_body.hovered() { hit_test(app.store.project(), lanes, geo, &app.store.selection, at) } else { crate::timeline_hit::Hit::Empty };
    state.hover_clip = hit.clip_id().map(String::from);
    if on_body.hovered() {
        ctx.set_cursor_icon(cursor_for(&hit, None));
    }
}

pub fn show(ui: &mut Ui, app: &mut App, area: Rect) {
    let mut state = std::mem::take(&mut app.timeline);
    state.begin_frame();
    draw(ui, app, &mut state, area);
    app.timeline = state;
}

fn draw(ui: &mut Ui, app: &mut App, state: &mut TimelineState, area: Rect) {
    let ctx = ui.ctx().clone();
    let corner_rect = Rect::from_min_size(area.min, vec2(HEADER_W, RULER_H));
    let ruler_rect = Rect::from_min_max(pos2(area.left() + HEADER_W, area.top()), pos2(area.right(), area.top() + RULER_H));
    let body = Rect::from_min_max(pos2(area.left() + HEADER_W, ruler_rect.bottom()), area.max);
    let shown = state.view.as_ref().map_or(app.store.project(), |v| &v.project);
    let (lanes, total) = layout_lanes(&shown.tracks);
    let max_scroll = (total - body.height()).max(0.0);
    state.v_scroll = state.v_scroll.clamp(0.0, max_scroll);
    app.store.stage_width = body.width() as f64;
    follow(app, state, body.width() as f64);
    scroll_and_zoom(&ctx, app, state, area, body, max_scroll);
    let geo = Geo { stage: body, v_scroll: state.v_scroll, scroll_sec: app.store.scroll_sec, px: app.store.px_per_sec };
    input(ui, app, state, &geo, &lanes, ruler_rect);
    let incoming = timeline_assets::update(&ctx, app, state, &geo, &lanes);
    timeline_assets::drop_library_item(&ctx, app, state, &geo);
    let view = state.view.take();
    let project = view.as_ref().map_or(app.store.project(), |v| &v.project);
    let painter = ui.painter().with_clip_rect(area);
    let body_painter = painter.with_clip_rect(body);
    background(&body_painter, &geo, &lanes);
    clips(&body_painter, &ctx, app, state, project, &geo, &lanes);
    if let Some(plan) = &incoming {
        timeline_assets::ghost(&body_painter, &geo, &lanes, project, plan);
    }
    ruler(&painter.with_clip_rect(ruler_rect), &geo, ruler_rect);
    let snap = view.as_ref().and_then(|v| v.snap_sec).or(incoming.as_ref().and_then(|p| p.snap_sec));
    markers(&painter, &geo, ruler_rect, project, app.store.playhead_sec, snap);
    let tracks = project.tracks.clone();
    state.view = view;
    corner(ui, app, corner_rect);
    track_headers::show(ui, app, state, &geo, &lanes, &tracks);
}
