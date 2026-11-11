use crate::app::App;
use crate::edits::{set_fade, update_clips};
use crate::inspector_effects;
use crate::inspector_speed;
use crate::inspector_text;
use crate::inspector_ui::{Row, color_row, hint, number_row, section, slider_row, toggle};
use crate::speed::local_at;
use crate::timeline::{TrimSide, clip_duration};
use crate::types::{Clip, Project, TrackKind};
use crate::widgets::{row_label, small_button};
use eframe::egui::Ui;

pub fn first_on(project: &Project, ids: &[String], kind: TrackKind) -> Option<Clip> {
    project.tracks.iter().filter(|t| t.kind == kind).flat_map(|t| &t.clips).find(|c| ids.contains(&c.id)).cloned()
}

pub fn seek_source(app: &mut App, clip: &Clip, source_sec: f64) {
    app.seek(clip.timeline_start_sec + local_at(clip, source_sec));
}

fn sound(ui: &mut Ui, app: &mut App, ids: &[String], audio: &Clip) {
    section(ui, "Sound");
    let row = Row::percent("Volume", 0.0, 100.0, 1.0);
    if let (Some(volume), _) = slider_row(ui, &row, audio.volume.unwrap_or(1.0), None) {
        let audio_ids: Vec<String> = app.store.project().tracks.iter().filter(|t| t.kind == TrackKind::Audio).flat_map(|t| &t.clips).filter(|c| ids.contains(&c.id)).map(|c| c.id.clone()).collect();
        let next = update_clips(app.store.project(), &audio_ids, |c| Clip { volume: (volume != 1.0).then_some(volume), ..c.clone() });
        app.store.edit(next, Some("clip-volume"));
    }
}

fn fades(ui: &mut Ui, app: &mut App, ids: &[String], first: &Clip) {
    let length = clip_duration(first);
    if let Some(sec) = number_row(ui, "Fade in", first.fade_in_sec.unwrap_or(0.0), (0.0, length), 0.05, " sec") {
        let next = set_fade(app.store.project(), ids, TrimSide::Start, sec);
        app.store.edit(next, Some("fade-in"));
    }
    if let Some(sec) = number_row(ui, "Fade out", first.fade_out_sec.unwrap_or(0.0), (0.0, length), 0.05, " sec") {
        let next = set_fade(app.store.project(), ids, TrimSide::End, sec);
        app.store.edit(next, Some("fade-out"));
    }
    ui.horizontal(|ui| {
        row_label(ui, "");
        if small_button(ui, "Swap in and out").on_hover_text("Swap the fade in and fade out lengths").clicked() {
            let next = update_clips(app.store.project(), ids, |c| Clip { fade_in_sec: Some(c.fade_out_sec.unwrap_or(0.0)), fade_out_sec: Some(c.fade_in_sec.unwrap_or(0.0)), ..c.clone() });
            app.store.edit(next, None);
        }
    });
}

fn video_fades(ui: &mut Ui, app: &mut App, ids: &[String], video: &Clip) {
    ui.horizontal(|ui| {
        row_label(ui, "");
        if let Some(on) = toggle(ui, "Cross dissolve", video.dissolve, "Fades blend into the clip next to it on the same track instead of fading to a color") {
            let next = update_clips(app.store.project(), ids, |c| if c.text.is_some() { c.clone() } else { Clip { dissolve: on, ..c.clone() } });
            app.store.edit(next, None);
        }
    });
    if video.dissolve {
        return;
    }
    let current = video.fade_color.clone().unwrap_or_else(|| "#000000".into());
    if let Some(color) = color_row(ui, "Fade color", &current, "#000000") {
        let fade_color = (color != "#000000").then_some(color);
        let next = update_clips(app.store.project(), ids, |c| if c.text.is_some() { c.clone() } else { Clip { fade_color: fade_color.clone(), ..c.clone() } });
        app.store.edit(next, Some("fade-color"));
    }
}

pub fn show(ui: &mut Ui, app: &mut App, clips: &[Clip]) {
    let ids: Vec<String> = clips.iter().map(|c| c.id.clone()).collect();
    let text = clips.iter().find(|c| c.text.is_some());
    let first = text.unwrap_or(&clips[0]);
    let project = app.store.project().clone();
    let (video, audio) = (first_on(&project, &ids, TrackKind::Video), first_on(&project, &ids, TrackKind::Audio));
    if let Some(clip) = text
        && let Some(style) = &clip.text
    {
        inspector_text::show(ui, app, clip, style);
    } else if !first.adjustment && first.freeze_at.is_none() {
        inspector_speed::show(ui, app, first, &ids);
    }
    if let Some(audio) = &audio {
        sound(ui, app, &ids, audio);
    }
    if let Some(effects_clip) = text.cloned().or(video.clone()) {
        inspector_effects::show(ui, app, &effects_clip, &ids, text.is_some());
    }
    if text.is_none() {
        section(ui, "Fades");
    }
    fades(ui, app, &ids, first);
    if text.is_none()
        && let Some(video) = video.filter(|v| !v.adjustment)
    {
        video_fades(ui, app, &ids, &video);
    }
    ui.add_space(6.0);
    let note = if clips.len() > 1 { format!("Fades apply to all {} selected clips. ", clips.len()) } else { String::new() };
    hint(ui, &format!("{note}You can also drag the small handles at the top corners of a clip."));
}
