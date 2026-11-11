#![windows_subsystem = "windows"]
#![allow(dead_code)]

mod actions;
mod app;
mod asset_drop;
mod assets;
mod audio_out;
mod cache;
mod clip_menu;
mod compose;
mod curve_editor;
mod curve_math;
mod curve_presets;
mod decode;
mod dialogs;
mod edge_fill;
mod editor_view;
mod edits;
mod effects;
mod export;
mod export_audio;
mod export_dialog;
mod export_options;
mod export_run;
mod ffmpeg;
mod ffmpeg_install;
mod fonts;
mod format;
mod formats;
mod framing;
mod freeze;
mod icons;
mod ids;
mod insert;
mod inspector;
mod inspector_clip;
mod inspector_effects;
mod inspector_keys;
mod inspector_project;
mod inspector_speed;
mod inspector_text;
mod inspector_ui;
mod jobs;
mod key_tags;
mod keyframe_panel;
mod keyframe_rows;
mod keyframes;
mod keys;
mod level_meter;
mod library;
mod library_actions;
mod library_edit;
mod library_view;
mod looks;
mod media;
mod media_bin;
mod mix;
mod overlays;
mod player_controls;
mod player_view;
mod presets;
mod preview;
mod preview_edit;
mod preview_hit;
mod preview_overlay;
mod probe;
mod project_file;
mod raster;
mod render_plan;
mod reverse;
mod reverse_run;
mod silence;
mod speed;
mod store;
#[cfg(test)]
mod tests;
mod text_render;
mod theme;
mod timeline;
mod timeline_assets;
mod timeline_drag;
mod timeline_draw;
mod timeline_hit;
mod timeline_input;
mod timeline_math;
mod timeline_menu;
mod timeline_ops;
mod timeline_state;
mod timeline_view;
mod track_headers;
mod types;
mod widgets;
mod win;

use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
    let launch: Vec<String> = std::env::args().skip(1).collect();
    if !win::claim_single_instance() {
        win::send_to_running_copy(&launch);
        return Ok(());
    }
    let viewport = ViewportBuilder::default().with_title("Tandem").with_inner_size([1400.0, 860.0]).with_min_inner_size([720.0, 480.0]).with_icon(theme::window_icon()).with_drag_and_drop(true);
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    let result = eframe::run_native("Tandem", options, Box::new(|cc| Ok(Box::new(app::App::new(&cc.egui_ctx, launch)))));
    win::release_single_instance();
    result
}
