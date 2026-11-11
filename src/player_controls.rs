use crate::app::App;
use crate::format::format_clock;
use crate::icons;
use crate::level_meter;
use crate::theme::{ACCENT, BORDER, DANGER, DANGER_LINE, DIM, OVERLAY, OVERLAY_LINE, PANEL_2, SCRIM, SEEK_TRACK, TEXT, TIP, WASH};
use crate::widgets::{Pill, icon_button, pill, slider, truncated};
use eframe::egui::text::LayoutJob;
use eframe::egui::{Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Mesh, Rect, ScrollArea, Sense, Stroke, StrokeKind, TextFormat, Ui, UiBuilder, pos2, vec2};

const NARROW: f32 = 900.0;
const MIXER_WIDTH: f32 = 260.0;
const ROW_HEIGHT: f32 = 52.0;

pub fn controls(ui: &mut Ui, rect: Rect, app: &mut App, duration: f64) {
    let area = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 92.0), rect.max);
    shade(ui, area);
    let inner = Rect::from_min_max(pos2(area.left() + 14.0, area.top() + 28.0), pos2(area.right() - 14.0, area.bottom() - 10.0));
    let bar = Rect::from_min_size(inner.min, vec2(inner.width(), 16.0));
    seek_bar(ui, bar, app, duration);
    let row = Rect::from_min_max(pos2(inner.left(), bar.bottom() + 6.0), pos2(inner.right(), bar.bottom() + 38.0));
    control_row(ui, row, app, duration, rect.width() >= NARROW);
}

fn shade(ui: &Ui, rect: Rect) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.right_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.right_bottom(), SCRIM);
    mesh.colored_vertex(rect.left_bottom(), SCRIM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(mesh);
}

pub fn seek_fraction(x: f32, left: f32, width: f32) -> f32 {
    ((x - left) / width.max(1.0)).clamp(0.0, 1.0)
}

fn seek_bar(ui: &mut Ui, rect: Rect, app: &mut App, duration: f64) {
    let response = ui.interact(rect, Id::unique("seek-bar"), Sense::click_and_drag());
    let usable = duration > 0.0;
    let hover = response.hover_pos().filter(|_| usable).map(|p| seek_fraction(p.x, rect.left(), rect.width()));
    if usable
        && response.is_pointer_button_down_on()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        app.seek(seek_fraction(pointer.x, rect.left(), rect.width()) as f64 * duration);
    }
    let painter = ui.painter();
    let track = Rect::from_center_size(rect.center(), vec2(rect.width(), if hover.is_some() { 6.0 } else { 4.0 }));
    let at = |fraction: f32| track.left() + track.width() * fraction;
    let fraction = if usable { (app.store.playhead_sec / duration).min(1.0) as f32 } else { 0.0 };
    painter.rect_filled(track, 2.0, SEEK_TRACK);
    if let Some(h) = hover {
        painter.rect_filled(Rect::from_min_max(track.min, pos2(at(h), track.bottom())), 2.0, SEEK_TRACK);
    }
    painter.rect_filled(Rect::from_min_max(track.min, pos2(at(fraction), track.bottom())), 2.0, ACCENT);
    let Some(h) = hover else { return };
    painter.circle_filled(pos2(at(fraction), track.center().y), 6.0, Color32::WHITE);
    let tip = format_clock(h as f64 * duration, duration >= 3600.0);
    let galley = painter.layout_no_wrap(tip, FontId::proportional(11.0), TEXT);
    let size = galley.size() + vec2(12.0, 4.0);
    let tip_rect = Rect::from_min_size(pos2(at(h) - size.x / 2.0, rect.bottom() - 18.0 - size.y), size);
    painter.rect_filled(tip_rect, 4.0, TIP);
    painter.galley(tip_rect.center() - galley.size() / 2.0, galley, TEXT);
}

fn timecode(playhead: f64, duration: f64) -> LayoutJob {
    let font = FontId::monospace(13.0);
    let mut job = LayoutJob::default();
    job.append(&format_clock(playhead, duration >= 3600.0), 0.0, TextFormat::simple(font.clone(), TEXT));
    job.append(&format!("/ {}", format_clock(duration, false)), 6.0, TextFormat::simple(font, DIM));
    job
}

fn control_row(ui: &mut Ui, row: Rect, app: &mut App, duration: f64, wide: bool) {
    let ctx = ui.ctx().clone();
    let empty = duration <= 0.0;
    let mut left = ui.new_child(UiBuilder::new().max_rect(row).layout(Layout::left_to_right(Align::Center)));
    left.spacing_mut().item_spacing.x = 8.0;
    let playing = app.store.playing;
    if icon_button(&mut left, vec2(40.0, 32.0), !empty, |p, r, c| icons::play(p, icons::centered(r, 18.0), playing, c)).on_hover_text("Play / pause (Space)").clicked() {
        app.toggle_play();
    }
    let (volume, muted) = (app.store.master_volume, app.store.master_muted);
    let speaker = icon_button(&mut left, vec2(34.0, 32.0), true, |p, r, c| icons::speaker(p, icons::centered(r, 16.0), muted || volume == 0.0, volume as f64, c));
    if speaker.on_hover_text("Mute (M)").clicked() {
        app.store.master_muted = !muted;
    }
    if wide {
        let mut shown = if muted { 0.0 } else { volume as f64 };
        if slider(&mut left, &mut shown, 90.0).on_hover_text(format!("Volume {}%", (volume * 100.0).round())).changed() {
            app.store.master_volume = shown as f32;
            app.store.master_muted = false;
        }
    }
    left.label(timecode(app.store.playhead_sec, duration));
    let left_end = left.min_rect().right();

    let mut right = ui.new_child(UiBuilder::new().max_rect(row).layout(Layout::right_to_left(Align::Center)));
    right.spacing_mut().item_spacing.x = 8.0;
    let fullscreen = App::is_fullscreen(&ctx);
    if icon_button(&mut right, vec2(34.0, 32.0), true, |p, r, c| icons::fullscreen(p, icons::centered(r, 16.0), fullscreen, c)).on_hover_text("Fullscreen (F)").clicked() {
        App::toggle_fullscreen(&ctx);
    }
    if pill(&mut right, "Edit", Pill::Accent).on_hover_text("Open the editor (E)").clicked() {
        app.enter_editor();
    }
    if pill(&mut right, "Open", Pill::Plain).on_hover_text("Open a video (Ctrl+O)").clicked() {
        app.pick_and_open();
    }
    if pill(&mut right, "Add audio", Pill::Plain).on_hover_text("Add an audio file as a new track").clicked() {
        app.pick_and_add_audio();
    }
    let tracks = app.store.audio_tracks().count();
    if tracks > 0 {
        let label = format!("{tracks} audio {}", if tracks == 1 { "track" } else { "tracks" });
        let kind = if app.mixer_open { Pill::On } else { Pill::Plain };
        if pill(&mut right, &label, kind).on_hover_text("Show the audio tracks").clicked() {
            app.mixer_open = !app.mixer_open;
        }
    }
    let right_start = right.min_rect().left();

    if wide && !empty {
        let gap = Rect::from_min_max(pos2(left_end + 8.0, row.top()), pos2(right_start - 8.0, row.bottom()));
        truncated(ui.painter(), gap, Align2::CENTER_CENTER, &app.store.project().name, FontId::proportional(13.0), DIM);
    }
}

pub fn mixer(ui: &mut Ui, rect: Rect, app: &mut App) {
    let outer = Rect::from_min_size(pos2(rect.right() - 14.0 - MIXER_WIDTH, rect.top() + 14.0), vec2(MIXER_WIDTH, (rect.height() - 120.0).max(60.0)));
    let mut child = ui.new_child(UiBuilder::new().max_rect(outer).layout(Layout::top_down(Align::Min)));
    let frame = Frame::NONE.fill(OVERLAY).stroke(Stroke::new(1.0, OVERLAY_LINE)).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(8));
    frame.show(&mut child, |ui| {
        ui.set_width(MIXER_WIDTH - 18.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let header = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover()).0;
        let format = FontId::new(11.0, crate::theme::semibold());
        ui.painter().text(pos2(header.left() + 4.0, header.top() + 2.0), Align2::LEFT_TOP, spaced("AUDIO TRACKS"), format, DIM);
        ScrollArea::vertical().max_height(outer.height() - 40.0).show(ui, |ui| {
            let tracks: Vec<(String, String, bool, f64)> = app.store.audio_tracks().map(|t| (t.id.clone(), t.name.clone(), t.muted, t.volume)).collect();
            for (index, track) in tracks.into_iter().enumerate() {
                mixer_row(ui, app, index, track);
            }
        });
    });
}

fn spaced(text: &str) -> String {
    text.chars().map(String::from).collect::<Vec<_>>().join("\u{200A}")
}

fn mixer_row(ui: &mut Ui, app: &mut App, index: usize, (id, name, muted, volume): (String, String, bool, f64)) {
    let (row, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    if response.contains_pointer() {
        ui.painter().rect_filled(row, 6.0, WASH);
    }
    let inner = row.shrink2(vec2(4.0, 5.0));
    let mute = Rect::from_center_size(pos2(inner.left() + 14.0, inner.center().y), vec2(28.0, 28.0));
    let mute_response = ui.interact(mute, Id::unique(("mixer-mute", &id)), Sense::click());
    let hint = if index < 9 { format!("Mute or unmute ({})", index + 1) } else { "Mute or unmute".into() };
    ui.painter().circle(mute.center(), 14.0, PANEL_2, Stroke::new(1.0, if muted { DANGER_LINE } else { BORDER }));
    icons::speaker(ui.painter(), icons::centered(mute, 14.0), muted, volume, if muted { DANGER } else { TEXT });
    if mute_response.on_hover_text(hint).clicked() {
        app.store.set_mix(&id, Some(!muted), None);
    }

    let level = Rect::from_min_max(pos2(inner.right() - 24.0, inner.top()), inner.max);
    ui.painter().text(level.right_center(), Align2::RIGHT_CENTER, format!("{}", (volume * 100.0).round()), FontId::proportional(11.0), DIM);

    let main = Rect::from_min_max(pos2(mute.right() + 8.0, inner.top()), pos2(level.left() - 8.0, inner.bottom()));
    let mut body = ui.new_child(UiBuilder::new().max_rect(main).layout(Layout::top_down(Align::Min)));
    if muted {
        body.set_opacity(0.45);
    }
    let name_rect = Rect::from_min_size(main.min, vec2(main.width(), 16.0));
    let mut text_left = name_rect.left();
    if index < 9 {
        let badge = Rect::from_min_size(pos2(text_left, name_rect.center().y - 7.0), vec2(14.0, 14.0));
        body.painter().rect(badge, 3.0, OVERLAY_LINE, Stroke::NONE, StrokeKind::Inside);
        body.painter().text(badge.center(), Align2::CENTER_CENTER, (index + 1).to_string(), FontId::proportional(10.0), DIM);
        text_left = badge.right() + 5.0;
    }
    let text_rect = Rect::from_min_max(pos2(text_left, name_rect.top()), name_rect.max);
    truncated(body.painter(), text_rect, Align2::LEFT_CENTER, &name, FontId::proportional(12.0), TEXT);
    body.add_space(18.0);
    let mut value = volume;
    if slider(&mut body, &mut value, main.width()).changed() {
        app.store.set_mix(&id, Some(false), Some(value));
    }
    let meter_rect = Rect::from_min_size(pos2(main.left(), body.min_rect().bottom() + 3.0), vec2(main.width(), 5.0));
    if let Some(meter) = app.meters.get(&id) {
        level_meter::paint(body.painter(), meter_rect, meter, false, false);
    }
}
