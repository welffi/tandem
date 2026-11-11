use crate::app::App;
use crate::dialogs;
use crate::export_options::{FRAME_RATES, QUALITIES, RESOLUTIONS, frame_rate_label, is_running, resolution_label, status_text, summary};
use crate::export_run::ExportRun;
use crate::theme::*;
use crate::types::{AudioMode, ExportPhase, ExportProgress, ExportSettings, Quality};
use crate::widgets::{Look, Pill, button, choose, pill, row_label};
use eframe::egui::{Align, Context, Id, Layout, Modal, ProgressBar, RichText, Ui};

pub struct ExportState {
    pub quality: Quality,
    pub audio_mode: AudioMode,
    pub run: Option<ExportRun>,
}

impl Default for ExportState {
    fn default() -> ExportState {
        ExportState { quality: Quality::High, audio_mode: AudioMode::Mix, run: None }
    }
}

fn options(ui: &mut Ui, app: &mut App, enabled: bool) {
    let project = app.store.project().clone();
    choose(ui, "export-resolution", "Resolution", enabled, resolution_label(&project), |ui| {
        for (label, width, height) in RESOLUTIONS {
            if ui.selectable_label((project.width, project.height) == (width, height), label).clicked() {
                app.store.update(None, |p| (p.width, p.height) = (width, height));
            }
        }
    });
    choose(ui, "export-fps", "Frame rate", enabled, frame_rate_label(project.fps), |ui| {
        for fps in FRAME_RATES {
            if ui.selectable_label(project.fps == fps, frame_rate_label(fps)).clicked() {
                app.store.update(None, |p| p.fps = fps);
            }
        }
    });
    let quality_label = QUALITIES.iter().find(|(q, _)| *q == app.export.quality).map_or("", |(_, label)| *label);
    choose(ui, "export-quality", "Quality", enabled, quality_label.to_string(), |ui| {
        for (quality, label) in QUALITIES {
            if ui.selectable_label(app.export.quality == quality, label).clicked() {
                app.export.quality = quality;
            }
        }
    });
    ui.horizontal(|ui| {
        row_label(ui, "Audio");
        ui.add_enabled_ui(enabled, |ui| {
            for (mode, label) in [(AudioMode::Mix, "Mix into one track"), (AudioMode::Separate, "Keep tracks separate")] {
                if pill(ui, label, if app.export.audio_mode == mode { Pill::On } else { Pill::Plain }).clicked() {
                    app.export.audio_mode = mode;
                }
            }
        });
    });
}

fn start(ctx: &Context, app: &mut App) {
    let name = format!("{}.mp4", app.store.project().name);
    let Some(output_path) = dialogs::save_video(&name) else { return };
    let settings = ExportSettings { output_path, quality: app.export.quality, audio_mode: app.export.audio_mode };
    app.export.run = Some(ExportRun::start(ctx, app.store.project().clone(), settings));
}

fn status(ui: &mut Ui, progress: &ExportProgress) {
    let fraction = match progress.phase {
        ExportPhase::Video => progress.fraction as f32,
        ExportPhase::Done => 1.0,
        _ => 0.0,
    };
    ui.add(ProgressBar::new(fraction).desired_width(ui.available_width()).fill(ACCENT));
    let color = if matches!(progress.phase, ExportPhase::Error(_)) { DANGER } else { DIM };
    ui.label(RichText::new(status_text(progress)).color(color));
}

fn buttons(ui: &mut Ui, ctx: &Context, app: &mut App, progress: Option<&ExportProgress>) {
    let running = progress.is_some_and(|p| is_running(&p.phase));
    let done = progress.is_some_and(|p| p.phase == ExportPhase::Done);
    let ready = app.store.duration() > 0.0;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if running {
            if button(ui, "Cancel export", Look::Plain, true).clicked() {
                app.export.run.as_ref().inspect(|run| run.cancel());
            }
            return;
        }
        let label = if progress.is_some() { "Export again…" } else { "Export…" };
        if button(ui, label, Look::Primary, ready).clicked() {
            start(ctx, app);
        }
        if button(ui, "Close", Look::Plain, true).clicked() {
            close(app);
        }
        if done && button(ui, "Show in folder", Look::Plain, true).clicked() {
            app.export.run.as_ref().inspect(|run| dialogs::show_in_folder(&run.output));
        }
    });
}

fn close(app: &mut App) {
    app.store.export_open = false;
    app.export.run = None;
}

pub fn show(ctx: &Context, app: &mut App) {
    let progress = app.export.run.as_ref().map(ExportRun::progress);
    let running = progress.as_ref().is_some_and(|p| is_running(&p.phase));
    let response = Modal::new(Id::unique("export-dialog")).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.heading("Export video");
        ui.add_space(8.0);
        options(ui, app, !running);
        ui.add_space(6.0);
        ui.label(RichText::new(summary(app.store.project(), app.export.audio_mode)).color(DIM));
        ui.add_space(10.0);
        if let Some(progress) = &progress {
            status(ui, progress);
            ui.add_space(6.0);
        }
        buttons(ui, ctx, app, progress.as_ref());
    });
    if response.should_close() && !running {
        close(app);
    }
}
