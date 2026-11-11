use crate::app::App;
use crate::export_options::{FRAME_RATES, frame_rate_label};
use crate::formats::{DEFAULT_LIMIT_DB, MIN_LIMIT_DB};
use crate::framing::{MAX_SCALE, MIN_SCALE, frame_look};
use crate::inspector_ui::{Row, choice, hint, section, slider_row, toggle};
use crate::types::{EdgeFill, Framing};
use crate::widgets::{choose, row_label};
use eframe::egui::{DragValue, Ui};

const ASPECTS: [(&str, u32, u32); 6] = [("16:9", 16, 9), ("9:16", 9, 16), ("1:1", 1, 1), ("4:5", 4, 5), ("4:3", 4, 3), ("21:9", 21, 9)];
const SIZES: [u32; 5] = [480, 720, 1080, 1440, 2160];
const HINT: &str = "Select a clip in the timeline to change its speed, effects and fades. Use Add text in the timeline toolbar for titles.";

fn even(n: f64) -> u32 {
    ((n / 2.0).round() * 2.0).max(2.0) as u32
}

pub fn size_for(ratio: f64, short_side: u32) -> (u32, u32) {
    let short = short_side as f64;
    if ratio >= 1.0 { (even(short * ratio), even(short)) } else { (even(short), even(short / ratio)) }
}

pub fn aspect_label(width: u32, height: u32) -> String {
    let ratio = width as f64 / height as f64;
    ASPECTS.iter().find(|(_, w, h)| (*w as f64 / *h as f64 - ratio).abs() < 0.01).map_or("Custom".to_string(), |(label, ..)| label.to_string())
}

fn size_label(short_side: u32) -> String {
    if short_side == 2160 { "4K (2160p)".into() } else { format!("{short_side}p") }
}

fn resize(app: &mut App, (width, height): (u32, u32)) {
    app.store.update(None, |p| (p.width, p.height) = (width, height));
}

fn output(ui: &mut Ui, app: &mut App) {
    section(ui, "Output");
    let (width, height) = (app.store.project().width, app.store.project().height);
    let (ratio, short_side) = (width as f64 / height as f64, width.min(height));
    choose(ui, "inspector-aspect", "Aspect", true, aspect_label(width, height), |ui| {
        for (label, w, h) in ASPECTS {
            if ui.selectable_label(aspect_label(width, height) == label, label).clicked() {
                resize(app, size_for(w as f64 / h as f64, short_side));
            }
        }
    });
    choose(ui, "inspector-resolution", "Resolution", true, size_label(short_side), |ui| {
        for size in SIZES {
            if ui.selectable_label(short_side == size, size_label(size)).clicked() {
                resize(app, size_for(ratio, size));
            }
        }
    });
    let (mut next_w, mut next_h) = (width, height);
    ui.horizontal(|ui| {
        row_label(ui, "Size");
        ui.add(DragValue::new(&mut next_w).range(16..=8192).speed(2));
        ui.label("×");
        ui.add(DragValue::new(&mut next_h).range(16..=8192).speed(2));
    });
    if (next_w, next_h) != (width, height) {
        resize(app, (even(next_w as f64), even(next_h as f64)));
    }
    let fps = app.store.project().fps;
    choose(ui, "inspector-fps", "Frame rate", true, frame_rate_label(fps), |ui| {
        for rate in FRAME_RATES {
            if ui.selectable_label(fps == rate, frame_rate_label(rate)).clicked() {
                app.store.update(None, |p| p.fps = rate);
            }
        }
    });
}

fn frame(ui: &mut Ui, app: &mut App) {
    let look = frame_look(app.store.project());
    section(ui, "Video in the frame");
    hint(ui, "How every video clip sits inside the output frame.");
    if let Some(framing) = choice(ui, "Framing", &[(Framing::Fit, "Fit (bars)"), (Framing::Fill, "Fill (crop)")], look.framing) {
        app.store.update(None, |p| p.framing = Some(framing));
    }
    let zoom = Row::percent("Zoom", MIN_SCALE * 100.0, MAX_SCALE * 100.0, 1.0);
    if let (Some(scale), _) = slider_row(ui, &zoom, look.scale, None) {
        app.store.update(Some("video-scale"), |p| p.video_scale = Some(scale));
    }
    section(ui, "Empty space");
    hint(ui, "What fills the frame where the video does not reach.");
    if let Some(fill) = choice(ui, "Fill", &[(EdgeFill::Black, "Black"), (EdgeFill::Mirror, "Mirror"), (EdgeFill::Blur, "Blur")], look.edge_fill) {
        app.store.update(None, |p| p.edge_fill = Some(fill));
    }
    if look.edge_fill == EdgeFill::Black {
        return;
    }
    if let (Some(blur), _) = slider_row(ui, &Row::plain("Blur", 0.0, 100.0, 40.0, ""), look.edge_blur, None) {
        app.store.update(Some("edge-blur"), |p| p.edge_blur = Some(blur.round()));
    }
    if let (Some(shadow), _) = slider_row(ui, &Row::plain("Shadow", 0.0, 100.0, 0.0, ""), look.shadow, None) {
        app.store.update(Some("shadow"), |p| p.shadow = Some(shadow.round()));
    }
    if look.shadow > 0.0
        && let (Some(size), _) = slider_row(ui, &Row::plain("Shadow size", 0.0, 100.0, 40.0, ""), look.shadow_size, None)
    {
        app.store.update(Some("shadow-size"), |p| p.shadow_size = Some(size.round()));
    }
}

fn audio(ui: &mut Ui, app: &mut App) {
    section(ui, "Audio");
    let limit = app.store.project().limit_db;
    ui.horizontal(|ui| {
        row_label(ui, "Limiter");
        let tip = "Keep the loudest moments under a ceiling without clipping. Quieter parts are left alone.";
        if let Some(on) = toggle(ui, if limit.is_some() { "On" } else { "Off" }, limit.is_some(), tip) {
            app.store.update(None, |p| p.limit_db = on.then_some(DEFAULT_LIMIT_DB));
        }
    });
    if let Some(ceiling) = limit
        && let (Some(next), _) = slider_row(ui, &Row::plain("Ceiling", MIN_LIMIT_DB, 0.0, DEFAULT_LIMIT_DB, " dB").step(0.5), ceiling, None)
    {
        app.store.update(Some("limit"), |p| p.limit_db = Some(next));
    }
}

pub fn show(ui: &mut Ui, app: &mut App) {
    output(ui, app);
    frame(ui, app);
    audio(ui, app);
    ui.add_space(6.0);
    hint(ui, HINT);
}
