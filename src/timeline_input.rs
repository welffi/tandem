use crate::app::App;
use crate::edits::{clips_sharing_edge, find_clip, group_of, snap_points};
use crate::store::key_ref;
use crate::timeline::TrimSide;
use crate::timeline_drag::{Drag, Pointing, compute};
use crate::timeline_draw::Geo;
use crate::timeline_hit::{Hit, hit_test, lane_under};
use crate::timeline_math::{Lane, SNAP_PX, scrub_time, trim_limits};
use crate::timeline_ops::{keys_by_clip, pick_keys};
use crate::timeline_state::TimelineState;
use eframe::egui::{CursorIcon, Modifiers, Pos2};

const MOVE_THRESHOLD: f32 = 3.0;

pub struct Pointer {
    pub at: Pos2,
    pub modifiers: Modifiers,
}

fn target_ids(app: &mut App, id: &str, single: bool) -> Vec<String> {
    if app.store.selection.iter().any(|s| s == id) {
        return app.store.selection.clone();
    }
    let ids = if single { vec![id.to_string()] } else { group_of(app.store.project(), id) };
    app.store.set_selection(ids.clone());
    ids
}

fn scrub(app: &mut App, geo: &Geo, at: Pos2) {
    let points = if app.store.snapping { snap_points(app.store.project(), &[], &[]) } else { Vec::new() };
    let sec = scrub_time(geo.time_at(at.x), app.store.snapping, &points, SNAP_PX / geo.px);
    app.seek(sec);
}

pub fn press_ruler(app: &mut App, state: &mut TimelineState, geo: &Geo, at: Pos2) {
    state.drag = Some(Drag::Scrub);
    scrub(app, geo, at);
}

fn press_body(app: &mut App, state: &mut TimelineState, geo: &Geo, id: &str, pointer: &Pointer) {
    let project = app.store.project().clone();
    let Some((track, clip)) = find_clip(&project, id) else { return };
    let toggle_keys = pointer.modifiers.shift || pointer.modifiers.command;
    let selected = app.store.selection.iter().any(|s| s == id);
    let single = pointer.modifiers.alt;
    let mut toggle = false;
    if toggle_keys && selected {
        toggle = true;
    } else if toggle_keys {
        let mut ids = app.store.selection.clone();
        ids.extend(if single { vec![id.to_string()] } else { group_of(&project, id) });
        ids.dedup();
        app.store.set_selection(ids);
    } else if !selected || single {
        let ids = if single { vec![id.to_string()] } else { group_of(&project, id) };
        app.store.set_selection(ids);
    }
    let clip_ids = app.store.selection.clone();
    state.drag = Some(Drag::Move {
        base: project.clone(),
        anchor: id.to_string(),
        clip_ids,
        track_id: track.id.clone(),
        grab_offset: geo.time_at(pointer.at.x) - clip.timeline_start_sec,
        down: pointer.at,
        active: false,
        toggle,
        copy: pointer.modifiers.alt,
    });
}

fn press_edge(app: &mut App, state: &mut TimelineState, id: &str, side: TrimSide) {
    let project = app.store.project().clone();
    let ids = target_ids(app, id, false);
    let Some((_, clip)) = find_clip(&project, id) else { return };
    let clip_ids = clips_sharing_edge(&project, &ids, clip, side);
    let range = trim_limits(&project, &clip_ids, side);
    state.drag = Some(Drag::Trim { base: project, anchor: id.to_string(), clip_ids, side, range });
}

fn press_fade(app: &mut App, state: &mut TimelineState, id: &str, side: TrimSide) {
    let project = app.store.project().clone();
    let clip_ids = target_ids(app, id, false);
    state.drag = Some(Drag::Fade { base: project, anchor: id.to_string(), clip_ids, side });
}

fn press_key(app: &mut App, state: &mut TimelineState, geo: &Geo, clip_id: &str, key_id: &str, pointer: &Pointer) {
    let reference = key_ref(clip_id, key_id);
    let additive = pointer.modifiers.shift || pointer.modifiers.command;
    let held = app.store.selected_keys.contains(&reference);
    app.store.selected_keys = pick_keys(&app.store.selected_keys, &reference, additive);
    if additive && held {
        return;
    }
    let key_ids = keys_by_clip(&app.store.selected_keys).remove(clip_id).unwrap_or_default();
    state.drag = Some(Drag::Key { base: app.store.project().clone(), anchor: clip_id.to_string(), key_ids, down_sec: geo.time_at(pointer.at.x) });
}

pub fn press(app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane], pointer: &Pointer) {
    let hit = hit_test(app.store.project(), lanes, geo, &app.store.selection, pointer.at);
    match hit {
        Hit::Empty => {
            if !pointer.modifiers.shift && !pointer.modifiers.command {
                app.store.set_selection(Vec::new());
                app.store.selected_keys.clear();
            }
            state.drag = Some(Drag::Scrub);
            scrub(app, geo, pointer.at);
        }
        Hit::Body(id) => press_body(app, state, geo, &id, pointer),
        Hit::Edge(id, side) => press_edge(app, state, &id, side),
        Hit::Fade(id, side) => press_fade(app, state, &id, side),
        Hit::Key(clip, key) => press_key(app, state, geo, &clip, &key, pointer),
    }
}

pub fn drag(app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane], pointer: &Pointer) {
    let Some(current) = state.drag.as_mut() else { return };
    if matches!(current, Drag::Scrub) {
        scrub(app, geo, pointer.at);
        return;
    }
    if let Drag::Move { down, active: false, .. } = current
        && (pointer.at - *down).length() > MOVE_THRESHOLD
    {
        current.start_moving();
    }
    if let Drag::Move { active: false, .. } = current {
        return;
    }
    let lane = lane_under(lanes, geo, pointer.at.y).and_then(|l| app.store.project().tracks.get(l.index)).map(|t| (t.id.clone(), t.kind));
    let at = Pointing { sec: geo.time_at(pointer.at.x), lane, snapping: app.store.snapping, alt: pointer.modifiers.alt, playhead: app.store.playhead_sec, px_per_sec: geo.px };
    state.view = compute(current, &at);
}

pub fn release(app: &mut App, state: &mut TimelineState) {
    let drag = state.drag.take();
    let view = state.view.take();
    let Some(drag) = drag else { return };
    if let Drag::Move { active: false, toggle, anchor, clip_ids, .. } = &drag {
        let group = group_of(app.store.project(), anchor);
        if *toggle {
            let kept: Vec<String> = app.store.selection.iter().filter(|id| !group.contains(id)).cloned().collect();
            app.store.set_selection(kept);
        } else if clip_ids.len() > group.len() && *clip_ids == app.store.selection {
            app.store.set_selection(group);
        }
        return;
    }
    let Some(view) = view else { return };
    app.store.edit(view.project, None);
    if let Drag::Move { clip_ids, copy: true, .. } = drag {
        app.store.set_selection(clip_ids);
    }
}

pub fn cursor_for(hit: &Hit, dragging: Option<&Drag>) -> CursorIcon {
    match dragging {
        Some(Drag::Move { active: true, .. }) | Some(Drag::Key { .. }) => return CursorIcon::Grabbing,
        Some(Drag::Trim { .. }) | Some(Drag::Fade { .. }) => return CursorIcon::ResizeHorizontal,
        Some(_) => return CursorIcon::Default,
        None => {}
    }
    match hit {
        Hit::Empty => CursorIcon::Default,
        Hit::Body(_) | Hit::Key(..) => CursorIcon::Grab,
        Hit::Edge(..) | Hit::Fade(..) => CursorIcon::ResizeHorizontal,
    }
}
