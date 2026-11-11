use crate::app::App;
use crate::dialogs;
use crate::edits::remove_asset;
use crate::format::format_duration;
use crate::icons;
use crate::library_view;
use crate::theme::*;
use crate::types::{MediaAsset, MediaKind};
use crate::widgets::{menu_item, small_button, truncated};
use eframe::egui::{Align, Align2, CursorIcon, FontId, Id, LayerId, Layout, Order, Rect, ScrollArea, Sense, Stroke, Ui, UiBuilder, pos2, scroll_area, vec2};

const HEADER_H: f32 = 36.0;
const ROW_H: f32 = 52.0;
const THUMB: [f32; 2] = [64.0, 36.0];
const HINT: &str = "Drop video or audio files here, or click Import.\nThen drag them onto the timeline.";

enum Action {
    Remove(String),
    Relink(String),
    SaveToLibrary(String),
}

pub fn asset_meta(asset: &MediaAsset) -> String {
    let streams = asset.audio_streams.len();
    let mut parts = vec![if asset.still { "Image".to_string() } else { format_duration(asset.duration_sec) }];
    if let (Some(w), Some(h)) = (asset.width, asset.height) {
        parts.push(format!("{w}×{h}"));
    }
    if streams > 1 {
        parts.push(format!("{streams} audio tracks"));
    }
    if asset.kind == MediaKind::Video && !asset.still && streams == 0 {
        parts.push("no audio".into());
    }
    parts.join(" · ")
}

fn thumb_texture(app: &mut App, ui: &Ui, asset: &MediaAsset) -> Option<eframe::egui::TextureId> {
    let thumbs = app.store.thumbs.get(&asset.id)?;
    let path = thumbs.files.get(1.min(thumbs.files.len().saturating_sub(1)))?;
    app.timeline.texture(ui.ctx(), path)
}

fn paint_thumb(ui: &Ui, app: &mut App, asset: &MediaAsset, rect: Rect) {
    ui.painter().rect_filled(rect, 3.0, PANEL_2);
    match thumb_texture(app, ui, asset) {
        Some(texture) => {
            ui.painter().image(texture, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), eframe::egui::Color32::WHITE);
        }
        None if asset.kind == MediaKind::Audio => icons::music(ui.painter(), icons::centered(rect, 18.0), DIM),
        None => icons::film(ui.painter(), icons::centered(rect, 18.0), DIM),
    }
}

fn used_count(app: &App, asset_id: &str) -> usize {
    app.store.project().tracks.iter().flat_map(|t| &t.clips).filter(|c| c.asset_id == asset_id).count()
}

fn row(ui: &mut Ui, app: &mut App, asset: &MediaAsset) -> Option<Action> {
    let missing = app.store.missing.contains(&asset.id);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click_and_drag());
    let button_rect = Rect::from_center_size(pos2(rect.right() - 18.0, rect.center().y), vec2(24.0, 24.0));
    let mut action = None;
    if missing {
        let locate = Rect::from_center_size(pos2(rect.right() - 32.0, rect.center().y), vec2(54.0, 22.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(locate));
        if small_button(&mut child, "Locate").clicked() {
            action = Some(Action::Relink(asset.id.clone()));
        }
    } else {
        let remove = ui.interact(button_rect, Id::unique(("asset-remove", &asset.id)), Sense::click());
        if remove.clicked() {
            action = Some(Action::Remove(asset.id.clone()));
        }
        if response.hovered() || remove.hovered() {
            let color = if remove.hovered() { DANGER } else { DIM };
            icons::cross(ui.painter(), icons::centered(button_rect, 10.0), color);
        }
        remove.on_hover_text("Remove from project");
    }
    if response.drag_started() && !missing {
        app.timeline.asset_drag = Some(asset.id.clone());
    }
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, HOVER);
        ui.ctx().set_cursor_icon(if missing { CursorIcon::Default } else { CursorIcon::Grab });
    }
    let thumb = Rect::from_min_size(pos2(rect.left() + 6.0, rect.center().y - THUMB[1] / 2.0), vec2(THUMB[0], THUMB[1]));
    paint_thumb(ui, app, asset, thumb);
    let text_x = thumb.right() + 10.0;
    let text_w = (rect.right() - if missing { 74.0 } else { 8.0 } - text_x).max(0.0);
    let name = Rect::from_min_size(pos2(text_x, rect.center().y - 15.0), vec2(text_w, 14.0));
    let meta = Rect::from_min_size(pos2(text_x, rect.center().y + 2.0), vec2(text_w, 13.0));
    let name_color = if missing { DANGER } else { TEXT };
    truncated(ui.painter(), name, Align2::LEFT_TOP, &asset.name, FontId::proportional(12.0), name_color);
    let meta_text = if missing { "File not found".to_string() } else { asset_meta(asset) };
    truncated(ui.painter(), meta, Align2::LEFT_TOP, &meta_text, FontId::proportional(10.5), DIM);
    let mut saved = None;
    response.on_hover_text(&asset.source_path).context_menu(|ui| {
        if menu_item(ui, "Save to library", "", !missing) {
            saved = Some(Action::SaveToLibrary(asset.id.clone()));
        }
    });
    saved.or(action)
}

fn apply(app: &mut App, action: Action) {
    match action {
        Action::Relink(id) => app.relink_asset(&id),
        Action::SaveToLibrary(id) => app.save_asset_to_library(&id),
        Action::Remove(id) => {
            let used = used_count(app, &id);
            let name = app.store.project().assets.iter().find(|a| a.id == id).map(|a| a.name.clone()).unwrap_or_default();
            if used > 0 && !dialogs::confirm(&format!("Remove {name} and its {used} clip(s) from the timeline?")) {
                return;
            }
            let next = remove_asset(app.store.project(), &id);
            app.store.edit(next, None);
            app.store.missing.remove(&id);
        }
    }
}

fn tab(ui: &mut Ui, text: &str, on: bool, height: f32) -> bool {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(12.0), TEXT);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 16.0, height), Sense::click());
    let color = if on || response.hovered() { TEXT } else { DIM };
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    if on {
        ui.painter().hline(rect.x_range().shrink(6.0), rect.bottom() - 1.5, Stroke::new(2.0, ACCENT));
    }
    response.clicked()
}

fn header(ui: &mut Ui, app: &mut App, rect: Rect) {
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, BORDER));
    let mut tabs = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(4.0, 0.0))).layout(Layout::left_to_right(Align::Center)));
    tabs.spacing_mut().item_spacing.x = 0.0;
    let library = app.library_ui.open;
    if tab(&mut tabs, "Media", !library, rect.height()) {
        app.library_ui.open = false;
    }
    if tab(&mut tabs, "Library", library, rect.height()) {
        app.library_ui.open = true;
    }
    if library {
        return;
    }
    let mut right = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0))).layout(Layout::right_to_left(Align::Center)));
    if small_button(&mut right, "Import…").on_hover_text("Add video, audio or images").clicked() {
        app.pick_and_open();
    }
}

fn empty_hint(ui: &Ui, rect: Rect) {
    let galley = ui.painter().layout(HINT.to_string(), FontId::proportional(11.0), DIM, rect.width() - 24.0);
    ui.painter().galley(pos2(rect.left() + 12.0, rect.top() + 16.0), galley, DIM);
}

pub fn show(ui: &mut Ui, app: &mut App, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, PANEL);
    ui.painter().vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, BORDER));
    header(ui, app, Rect::from_min_size(rect.min, vec2(rect.width(), HEADER_H)));
    let list_rect = Rect::from_min_max(pos2(rect.left() + 6.0, rect.top() + HEADER_H + 4.0), pos2(rect.right() - 6.0, rect.bottom() - 4.0));
    if app.library_ui.open {
        library_view::show(ui, app, list_rect);
        return;
    }
    let assets = app.store.project().assets.clone();
    if assets.is_empty() {
        empty_hint(ui, list_rect);
        return;
    }
    let mut action = None;
    let mut list = ui.new_child(UiBuilder::new().max_rect(list_rect));
    ScrollArea::vertical().scroll_source(scroll_area::ScrollSource::MOUSE_WHEEL | scroll_area::ScrollSource::SCROLL_BAR).auto_shrink([false, false]).show(&mut list, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for asset in &assets {
            action = row(ui, app, asset).or(action.take());
        }
    });
    if let Some(action) = action {
        apply(app, action);
    }
}

pub fn drag_chip(ui: &Ui, app: &App) {
    let asset_name = app.timeline.asset_drag.as_ref().and_then(|id| app.store.project().assets.iter().find(|a| a.id == *id)).map(|a| a.name.clone());
    let Some(name) = asset_name.or_else(|| library_view::drag_label(app)) else { return };
    let Some(pos) = ui.ctx().pointer_latest_pos() else { return };
    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    let painter = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::unique("asset-chip")));
    let galley = painter.layout_no_wrap(name, FontId::proportional(11.0), TEXT);
    let rect = Rect::from_min_size(pos + vec2(14.0, 14.0), galley.size() + vec2(16.0, 8.0));
    painter.rect_filled(rect, 5.0, TIP);
    painter.galley(rect.min + vec2(8.0, 4.0), galley, TEXT);
}
