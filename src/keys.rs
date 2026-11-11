use crate::app::App;
use crate::store::Mode;
use crate::timeline_ops::ZOOM_STEP;
use eframe::egui::{Context, Event, Key, Modifiers};

pub fn handle(app: &mut App, ctx: &Context) {
    let presses: Vec<(Key, Modifiers)> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    });
    for (key, modifiers) in presses {
        if app.store.export_open {
            continue;
        }
        if ctx.egui_wants_keyboard_input() {
            continue;
        }
        match app.store.mode {
            Mode::Player => player_key(app, ctx, key, modifiers),
            Mode::Editor => editor_key(app, key, modifiers),
        }
    }
}

fn seek_by(app: &mut App, delta: f64) {
    app.seek((app.store.playhead_sec + delta).min(app.store.duration()));
}

pub fn nudge_volume(volume: f32, delta: f32) -> f32 {
    ((volume + delta).clamp(0.0, 1.0) * 100.0).round() / 100.0
}

fn number_of(key: Key) -> Option<usize> {
    let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
    keys.iter().position(|k| *k == key)
}

fn player_key(app: &mut App, ctx: &Context, key: Key, modifiers: Modifiers) {
    if modifiers.command {
        if key == Key::O {
            app.pick_and_open();
        }
        return;
    }
    if let Some(index) = number_of(key) {
        let found = app.store.audio_tracks().nth(index).map(|t| (t.id.clone(), t.muted));
        if let Some((id, muted)) = found {
            app.store.set_mix(&id, Some(!muted), None);
        }
        return;
    }
    let volume = |app: &mut App, delta: f32| {
        app.store.master_volume = nudge_volume(app.store.master_volume, delta);
        app.store.master_muted = false;
    };
    match key {
        Key::Space | Key::K => app.toggle_play(),
        Key::ArrowLeft => seek_by(app, -5.0),
        Key::ArrowRight => seek_by(app, 5.0),
        Key::J => seek_by(app, -10.0),
        Key::L => seek_by(app, 10.0),
        Key::ArrowUp => volume(app, 0.05),
        Key::ArrowDown => volume(app, -0.05),
        Key::Home => app.seek(0.0),
        Key::End => app.seek(app.store.duration()),
        Key::Escape if App::is_fullscreen(ctx) => App::toggle_fullscreen(ctx),
        Key::M => app.store.master_muted = !app.store.master_muted,
        Key::F => App::toggle_fullscreen(ctx),
        Key::E => app.enter_editor(),
        _ => {}
    }
}

fn editor_key(app: &mut App, key: Key, modifiers: Modifiers) {
    if modifiers.command {
        match key {
            Key::Z if modifiers.shift => app.store.redo(),
            Key::Z => app.store.undo(),
            Key::Y => app.store.redo(),
            Key::S => drop(app.save_project(modifiers.shift)),
            Key::O => app.open_project(None),
            Key::N => app.new_project(),
            Key::E => app.store.export_open = true,
            _ => {}
        }
        return;
    }
    let step = if modifiers.shift { 1.0 } else { 1.0 / app.store.project().fps };
    match key {
        Key::Space => app.toggle_play(),
        Key::Comma | Key::ArrowLeft => app.seek(app.store.playhead_sec - step),
        Key::Period | Key::ArrowRight => app.seek(app.store.playhead_sec + step),
        Key::Home => app.seek(0.0),
        Key::End => app.seek(app.store.duration()),
        Key::Escape => app.store.set_selection(Vec::new()),
        Key::N => app.store.snapping = !app.store.snapping,
        Key::Delete | Key::Backspace => delete_key(app, modifiers.shift),
        Key::S => app.store.split_at_playhead(),
        Key::A => app.store.add_adjustment_at_playhead(),
        Key::T => app.store.add_text_at_playhead(),
        Key::Equals | Key::Plus => app.store.zoom_by(ZOOM_STEP, app.store.stage_width),
        Key::Minus => app.store.zoom_by(1.0 / ZOOM_STEP, app.store.stage_width),
        _ => {}
    }
}

fn delete_key(app: &mut App, ripple: bool) {
    if app.store.delete_selected_keys() {
        return;
    }
    if ripple {
        app.store.ripple_delete_selection();
    } else {
        app.store.delete_selected_clips();
    }
}
