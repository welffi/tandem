use crate::app::App;
use crate::clip_menu::{ClipMenu, clip_menu};
use crate::edits::group_of;
use crate::timeline_draw::Geo;
use crate::timeline_hit::hit_test;
use crate::timeline_math::Lane;
use crate::timeline_state::TimelineState;
use crate::types::TrackKind;
use crate::widgets::menu_item as item;
use eframe::egui::{Response, Ui};

fn clip_items(ui: &mut Ui, app: &mut App, clip_id: &str, menu: &ClipMenu) {
    if item(ui, "Split at playhead", "S", menu.split) {
        app.store.split_at_playhead();
    }
    if let Some(enabled) = menu.freeze
        && item(ui, "Add Freeze Frame", "", enabled)
    {
        app.store.add_freeze_frame(clip_id);
    }
    if let Some(enabled) = menu.end_freeze
        && item(ui, "End Freeze Frame", "", enabled)
    {
        app.store.end_freeze_frame(clip_id);
    }
    if menu.trim_silence {
        ui.separator();
        if item(ui, "Trim silence (front and back)", "", true) {
            app.store.trim_silence_selection();
        }
    }
    if menu.reverse && item(ui, "Reverse", "", true) {
        app.reverse_selection();
    }
    ui.separator();
    if item(ui, "Delete", "Del", true) {
        app.store.delete_selected_clips();
    }
    if item(ui, "Ripple delete", "Shift+Del", true) {
        app.store.ripple_delete_selection();
    }
}

fn empty_items(ui: &mut Ui, app: &mut App) {
    if item(ui, "Add text at playhead", "T", true) {
        app.store.add_text_at_playhead();
    }
    if item(ui, "Add adjustment layer at playhead", "A", true) {
        app.store.add_adjustment_at_playhead();
    }
    ui.separator();
    if item(ui, "Add video track", "", true) {
        app.store.add_track_of(TrackKind::Video);
    }
    if item(ui, "Add audio track", "", true) {
        app.store.add_track_of(TrackKind::Audio);
    }
}

fn open_at(app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane], response: &Response) {
    let Some(at) = response.interact_pointer_pos() else { return };
    let hit = hit_test(app.store.project(), lanes, geo, &app.store.selection, at);
    state.menu_clip = hit.clip_id().map(String::from);
    if let Some(id) = &state.menu_clip
        && !app.store.selection.contains(id)
    {
        let group = group_of(app.store.project(), id);
        app.store.set_selection(group);
    }
}

pub fn show(app: &mut App, state: &mut TimelineState, geo: &Geo, lanes: &[Lane], response: &Response) {
    if response.secondary_clicked() {
        open_at(app, state, geo, lanes, response);
    }
    response.context_menu(|ui| {
        let menu = state.menu_clip.as_deref().and_then(|id| clip_menu(app.store.project(), &app.store.selection, id, app.store.playhead_sec).map(|m| (id.to_string(), m)));
        match menu {
            Some((id, menu)) => clip_items(ui, app, &id, &menu),
            None => empty_items(ui, app),
        }
    });
}
