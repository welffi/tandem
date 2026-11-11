use crate::app::App;
use crate::icons;
use crate::player_controls;
use crate::theme::{ACCENT, BG, DIM, EMPTY_GLOW, LOGO_END, bold};
use crate::types::TrackKind;
use crate::widgets::{Look, button};
use eframe::egui::text::LayoutJob;
use eframe::egui::{Align, Color32, CursorIcon, FontId, Id, Layout, Mesh, Painter, Pos2, Rect, Sense, TextFormat, Ui, UiBuilder, Vec2, pos2, vec2};
use std::f32::consts::TAU;
use std::time::Duration;

const IDLE: Duration = Duration::from_millis(2500);

pub fn fit(outer: Rect, width: f32, height: f32) -> Rect {
    let scale = (outer.width() / width.max(1.0)).min(outer.height() / height.max(1.0));
    Rect::from_center_size(outer.center(), vec2(width, height) * scale)
}

pub fn show(ui: &mut Ui, app: &mut App) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BG);
    let ctx = ui.ctx().clone();
    if ctx.input(|i| i.pointer.delta() != Vec2::ZERO || i.pointer.any_down()) {
        app.last_activity = std::time::Instant::now();
    }
    let idle_for = app.last_activity.elapsed();
    let hide_ui = app.store.playing && idle_for >= IDLE;
    if app.store.playing && !hide_ui {
        ctx.request_repaint_after(IDLE - idle_for);
    }

    let duration = app.store.duration();
    let empty = duration <= 0.0;
    let project = app.store.project();
    let has_clips = |kind: TrackKind| project.tracks.iter().any(|t| t.kind == kind && !t.clips.is_empty());
    let has_video = has_clips(TrackKind::Video);
    let has_picture = has_video || has_clips(TrackKind::Text);

    let stage = ui.interact(rect, Id::unique("player-stage"), Sense::click());
    if stage.double_clicked() {
        App::toggle_fullscreen(&ctx);
    } else if stage.clicked() && !empty {
        app.toggle_play();
    }

    if empty {
        empty_state(ui, rect, app);
    } else if has_picture {
        picture(ui, rect, app);
    }
    if !empty && !has_video {
        audio_only(ui.painter(), rect, &app.store.project().name);
    }

    if hide_ui {
        ctx.set_cursor_icon(CursorIcon::None);
        return;
    }
    if app.mixer_open && app.store.audio_tracks().next().is_some() {
        player_controls::mixer(ui, rect, app);
    }
    player_controls::controls(ui, rect, app, duration);
}

pub fn picture(ui: &mut Ui, rect: Rect, app: &mut App) {
    let project = app.store.project();
    let frame = fit(rect, project.width as f32, project.height as f32);
    let ppp = ui.ctx().pixels_per_point();
    let w = ((frame.width() * ppp).round() as usize).min(project.width as usize);
    let h = ((frame.height() * ppp).round() as usize).min(project.height as usize);
    let last_frame = (app.store.duration() - 0.5 / project.fps).max(0.0);
    app.preview.request(project, app.store.rev, app.store.playhead_sec.min(last_frame), w, h, app.store.playing);
    if let Some(texture) = &app.preview.texture {
        let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
        ui.painter().image(texture.id(), frame, uv, Color32::WHITE);
    }
}

fn audio_only(painter: &Painter, rect: Rect, name: &str) {
    let center = rect.center();
    let icon = Rect::from_center_size(center - vec2(0.0, 18.0), Vec2::splat(56.0));
    icons::music(painter, icon, DIM);
    painter.text(pos2(center.x, icon.bottom() + 14.0), eframe::egui::Align2::CENTER_TOP, name, FontId::proportional(16.0), DIM);
}

fn radial_glow(painter: &Painter, rect: Rect) {
    let center = pos2(rect.center().x, rect.top() + rect.height() * 0.4);
    let corner = [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()].iter().map(|c| c.distance(center)).fold(0.0, f32::max);
    let radius = corner * 0.7;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, EMPTY_GLOW);
    let steps = 64;
    for i in 0..=steps {
        let angle = TAU * i as f32 / steps as f32;
        mesh.colored_vertex(center + vec2(angle.cos(), angle.sin()) * radius, BG);
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    painter.with_clip_rect(rect).add(mesh);
}

fn logo() -> LayoutJob {
    let text = "Tandem";
    let count = text.chars().count().max(2) - 1;
    let mut job = LayoutJob::default();
    for (i, c) in text.chars().enumerate() {
        let color = ACCENT.lerp_to_gamma(LOGO_END, i as f32 / count as f32);
        let format = TextFormat { font_id: FontId::new(44.0, bold()), color, extra_letter_spacing: 44.0 * 0.04, ..Default::default() };
        job.append(&c.to_string(), 0.0, format);
    }
    job
}

fn empty_state(ui: &mut Ui, rect: Rect, app: &mut App) {
    radial_glow(ui.painter(), rect);
    let block = Rect::from_center_size(rect.center(), vec2(rect.width(), 150.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(block).layout(Layout::top_down(Align::Center)));
    child.spacing_mut().item_spacing.y = 14.0;
    child.label(logo());
    child.colored_label(DIM, "Drop a video here to play every audio track at once.");
    if button(&mut child, "Open…", Look::Primary, true).clicked() {
        app.pick_and_open();
    }
}
