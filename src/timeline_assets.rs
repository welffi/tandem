use crate::app::App;
use crate::asset_drop::{DropPlan, DropPointer, plan_drop};
use crate::insert::insert_asset;
use crate::theme::*;
use crate::timeline_draw::Geo;
use crate::timeline_hit::lane_under;
use crate::timeline_math::{LANE_PAD, Lane, lane_height};
use crate::timeline_state::TimelineState;
use crate::types::Project;
use eframe::egui::{Context, Painter, Rect, Stroke, StrokeKind, pos2, vec2};

fn plan_at(app: &App, geo: &Geo, lanes: &[Lane], asset_id: &str, ctx: &Context) -> Option<DropPlan> {
    let (at, alt) = ctx.input(|i| (i.pointer.interact_pos(), i.modifiers.alt));
    let at = at.filter(|p| geo.stage.contains(*p))?;
    let project = app.store.project();
    let asset = project.assets.iter().find(|a| a.id == asset_id)?;
    let lane = lane_under(lanes, geo, at.y).and_then(|l| project.tracks.get(l.index)).map(|t| (t.id.as_str(), t.kind));
    let pointer = DropPointer { sec: geo.time_at(at.x), lane, snapping: app.store.snapping, alt, playhead: app.store.playhead_sec, px_per_sec: geo.px };
    Some(plan_drop(project, asset, &pointer))
}

fn commit(app: &mut App, asset_id: &str, plan: &DropPlan) {
    let Some(asset) = app.store.project().assets.iter().find(|a| a.id == asset_id).cloned() else { return };
    let (next, ids) = insert_asset(app.store.project(), &asset, &plan.target, plan.start_sec);
    app.store.edit(next, None);
    app.store.set_selection(ids);
}

pub fn drop_library_item(ctx: &Context, app: &mut App, state: &mut TimelineState, geo: &Geo) {
    let Some(id) = state.library_drag.clone() else { return };
    if !ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down()) {
        return;
    }
    state.library_drag = None;
    if let Some(at) = ctx.input(|i| i.pointer.interact_pos()).filter(|p| geo.stage.contains(*p)) {
        app.apply_library_item(&id, geo.time_at(at.x).max(0.0));
    }
}

pub fn update(ctx: &Context, app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane]) -> Option<DropPlan> {
    let asset_id = state.asset_drag.clone()?;
    let plan = plan_at(app, geo, lanes, &asset_id, ctx);
    let finished = ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down());
    if !finished {
        return plan;
    }
    state.asset_drag = None;
    if let Some(plan) = plan {
        commit(app, &asset_id, &plan);
    }
    None
}

pub fn ghost(painter: &Painter, geo: &Geo, lanes: &[Lane], project: &Project, plan: &DropPlan) {
    let mut spare = lanes.last().map_or(0.0, |l| l.y + l.h);
    for piece in &plan.pieces {
        let lane = piece.track_id.as_ref().and_then(|id| project.tracks.iter().position(|t| t.id == *id)).and_then(|i| lanes.get(i));
        let (y, h) = match lane {
            Some(lane) => (lane.y, lane.h),
            None => {
                let h = lane_height(piece.kind);
                spare += h;
                (spare - h, h)
            }
        };
        let top = geo.stage.top() + y - geo.v_scroll + LANE_PAD;
        let rect = Rect::from_min_size(pos2(geo.x_at(plan.start_sec), top), vec2((plan.duration_sec * geo.px) as f32, h - LANE_PAD * 2.0));
        painter.rect_filled(rect, 3.0, GHOST_FILL);
        painter.rect_stroke(rect, 3.0, Stroke::new(1.5, GHOST_LINE), StrokeKind::Inside);
    }
}
