use crate::app::App;
use crate::curve_editor::Naming;
use crate::icons;
use crate::library_edit::{child_count, entry_name, folder_meta, folder_path, item_meta};
use crate::theme::*;
use crate::types::Library;
use crate::widgets::{menu_item, small_button, truncated};
use eframe::egui::{Align, Align2, Color32, CursorIcon, FontId, Id, Key, Layout, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, pos2, scroll_area, vec2};

const SAVE_H: f32 = 24.0;
const CRUMB_H: f32 = 22.0;
const ROW_H: f32 = 46.0;
const HINT: &str = "Select clips on the timeline and click Save selection,\nor right-click a file in Media to save it here.\nAdding an item puts its layers back at the playhead.";

#[derive(Default)]
pub struct LibraryState {
    pub open: bool,
    name: String,
    renaming: Option<Naming>,
}

struct Entry {
    id: String,
    name: String,
    meta: String,
    color: Option<String>,
    folder: bool,
}

enum Action {
    Open(Option<String>),
    Add(String),
    Update(String),
    Rename(String, String),
    Recolor(String, Option<String>),
    DeleteItem(String),
    DeleteFolder(String),
    Move(String, Option<String>),
}

impl Entry {
    fn open_or_add(&self) -> Action {
        if self.folder { Action::Open(Some(self.id.clone())) } else { Action::Add(self.id.clone()) }
    }

    fn delete(&self) -> Action {
        if self.folder { Action::DeleteFolder(self.id.clone()) } else { Action::DeleteItem(self.id.clone()) }
    }

    fn tint(&self) -> Option<Color32> {
        self.color.as_deref().and_then(tint_color)
    }
}

fn entries(library: &Library, current: Option<&str>) -> Vec<Entry> {
    let folders = library.folders.iter().filter(|f| f.parent_id.as_deref() == current).map(|f| Entry {
        id: f.id.clone(),
        name: f.name.clone(),
        meta: folder_meta(child_count(library, &f.id)),
        color: f.color.clone(),
        folder: true,
    });
    let items = library.items.iter().filter(|p| p.folder_id.as_deref() == current).map(|p| Entry { id: p.id.clone(), name: p.name.clone(), meta: item_meta(p), color: p.color.clone(), folder: false });
    folders.chain(items).collect()
}

fn dragging_over(ui: &Ui, app: &App, rect: Rect, own: Option<&str>) -> bool {
    app.timeline.library_drag.as_deref().is_some_and(|id| Some(id) != own) && ui.rect_contains_pointer(rect)
}

fn dropped_on(ui: &Ui, app: &App, rect: Rect, own: Option<&str>) -> Option<String> {
    let released = ui.input(|i| i.pointer.primary_released());
    app.timeline.library_drag.clone().filter(|_| released && dragging_over(ui, app, rect, own))
}

fn swatches(ui: &mut Ui, current: Option<&str>) -> Option<Option<String>> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for tint in std::iter::once(None).chain(TINTS.iter().map(|t| Some(*t))) {
            let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
            match tint.and_then(tint_color) {
                Some(color) => drop(ui.painter().circle_filled(rect.center(), 6.0, color)),
                None => drop(ui.painter().circle_stroke(rect.center(), 6.0, Stroke::new(1.0, DIM))),
            }
            if current == tint || response.hovered() {
                ui.painter().circle_stroke(rect.center(), 8.0, Stroke::new(1.5, TEXT));
            }
            if response.clicked() {
                picked = Some(tint.map(String::from));
                ui.close();
            }
        }
    });
    picked
}

fn rename_field(ui: &mut Ui, app: &mut App, rect: Rect) {
    let Some(mut naming) = app.library_ui.renaming.take() else { return };
    let response = ui.put(rect, TextEdit::singleline(&mut naming.name).id(Id::unique("library-rename")));
    if !naming.asked {
        response.request_focus();
        naming.asked = true;
    }
    if !response.lost_focus() {
        app.library_ui.renaming = Some(naming);
    } else if !ui.input(|i| i.key_pressed(Key::Escape))
        && let Some(id) = &naming.id
    {
        app.rename_library_entry(id, &naming.name);
    }
}

fn menu(ui: &mut Ui, entry: &Entry, has_selection: bool) -> Option<Action> {
    let mut action = None;
    if entry.folder {
        if menu_item(ui, "Open", "", true) {
            action = Some(entry.open_or_add());
        }
    } else {
        if menu_item(ui, "Add at playhead", "", true) {
            action = Some(entry.open_or_add());
        }
        if menu_item(ui, "Update from selection", "", has_selection) {
            action = Some(Action::Update(entry.id.clone()));
        }
    }
    if menu_item(ui, "Rename", "", true) {
        action = Some(Action::Rename(entry.id.clone(), entry.name.clone()));
    }
    ui.separator();
    if let Some(picked) = swatches(ui, entry.color.as_deref()) {
        action = Some(Action::Recolor(entry.id.clone(), picked));
    }
    ui.separator();
    if menu_item(ui, if entry.folder { "Delete folder" } else { "Delete" }, "", true) {
        action = Some(entry.delete());
    }
    action
}

fn buttons(ui: &mut Ui, entry: &Entry, rect: Rect) -> Option<Action> {
    let mut action = None;
    let cross = Rect::from_center_size(pos2(rect.right() - 18.0, rect.center().y), vec2(24.0, 24.0));
    let remove = ui.interact(cross, Id::unique(("library-remove", &entry.id)), Sense::click());
    icons::cross(ui.painter(), icons::centered(cross, 10.0), if remove.hovered() { DANGER } else { DIM });
    if remove.on_hover_text(if entry.folder { "Delete folder" } else { "Delete from library" }).clicked() {
        action = Some(entry.delete());
    }
    let strip = Rect::from_min_max(rect.min, pos2(cross.left() - 2.0, rect.bottom()));
    let mut child = ui.new_child(UiBuilder::new().max_rect(strip).layout(Layout::right_to_left(Align::Center)));
    if small_button(&mut child, if entry.folder { "Open" } else { "Add" }).clicked() {
        action = Some(entry.open_or_add());
    }
    action
}

fn paint_row(ui: &Ui, entry: &Entry, rect: Rect, lit: Option<bool>) -> f32 {
    match lit {
        Some(true) => drop(ui.painter().rect(rect, 6.0, GHOST_FILL, Stroke::new(1.0, GHOST_LINE), StrokeKind::Inside)),
        Some(false) => drop(ui.painter().rect_filled(rect, 6.0, HOVER)),
        None => {}
    }
    if let Some(color) = entry.tint() {
        ui.painter().rect_filled(Rect::from_min_size(rect.left_top() + vec2(0.0, 7.0), vec2(3.0, ROW_H - 14.0)), 1.5, color);
    }
    if !entry.folder {
        return rect.left() + 12.0;
    }
    icons::folder(ui.painter(), Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(20.0, 20.0)), entry.tint().unwrap_or(DIM));
    rect.left() + 38.0
}

fn row(ui: &mut Ui, app: &mut App, entry: &Entry) -> Option<Action> {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click_and_drag());
    let target = entry.folder && dragging_over(ui, app, rect, Some(&entry.id));
    let text_x = paint_row(ui, entry, rect, if target { Some(true) } else { response.hovered().then_some(false) });
    let text_w = (rect.right() - 92.0 - text_x).max(0.0);
    let renaming = app.library_ui.renaming.as_ref().is_some_and(|n| n.id.as_deref() == Some(entry.id.as_str()));
    if renaming {
        rename_field(ui, app, Rect::from_min_size(pos2(text_x, rect.center().y - 11.0), vec2(text_w + 20.0, 22.0)));
    } else {
        truncated(ui.painter(), Rect::from_min_size(pos2(text_x, rect.center().y - 15.0), vec2(text_w, 14.0)), Align2::LEFT_TOP, &entry.name, FontId::proportional(12.0), TEXT);
        truncated(ui.painter(), Rect::from_min_size(pos2(text_x, rect.center().y + 2.0), vec2(text_w, 13.0)), Align2::LEFT_TOP, &entry.meta, FontId::proportional(10.5), DIM);
    }
    let mut action = buttons(ui, entry, rect);
    if response.drag_started() {
        app.timeline.library_drag = Some(entry.id.clone());
    }
    if response.double_clicked() {
        action = Some(entry.open_or_add());
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    let has_selection = !app.store.selection.is_empty();
    let mut chosen = None;
    let hint = if entry.folder { "Double-click to open" } else { "Double-click or drag onto the timeline to add it" };
    response.on_hover_text(hint).context_menu(|ui| chosen = menu(ui, entry, has_selection));
    let moved = dropped_on(ui, app, rect, Some(&entry.id)).filter(|_| entry.folder).map(|id| Action::Move(id, Some(entry.id.clone())));
    moved.or(chosen).or(action)
}

fn crumb(ui: &mut Ui, app: &App, text: &str, last: bool, id: Option<&str>) -> Option<Action> {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(11.0), TEXT);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(8.0, 4.0), Sense::click());
    if dragging_over(ui, app, rect, id) {
        ui.painter().rect(rect, 4.0, GHOST_FILL, Stroke::new(1.0, GHOST_LINE), StrokeKind::Inside);
    }
    let color = if last || response.hovered() { TEXT } else { DIM };
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    if let Some(dragged) = dropped_on(ui, app, rect, id) {
        return Some(Action::Move(dragged, id.map(String::from)));
    }
    (!last && response.clicked()).then(|| Action::Open(id.map(String::from)))
}

fn crumbs(ui: &mut Ui, app: &App, rect: Rect) -> Option<Action> {
    let path = folder_path(&app.store.library, app.store.library_folder_id.as_deref());
    let mut bar = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)));
    bar.spacing_mut().item_spacing.x = 2.0;
    let mut action = crumb(&mut bar, app, "Library", path.is_empty(), None);
    for (i, folder) in path.iter().enumerate() {
        bar.label(RichText::new("/").color(DIM));
        action = crumb(&mut bar, app, &folder.name, i + 1 == path.len(), Some(&folder.id)).or(action);
    }
    action
}

fn save_row(ui: &mut Ui, app: &mut App, rect: Rect) {
    let fallback = format!("Preset {}", app.store.library.items.len() + 1);
    let selected = !app.store.selection.is_empty();
    let mut bar = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
    if small_button(&mut bar, "+ Folder").on_hover_text("New folder").clicked() {
        let id = app.new_library_folder();
        app.library_ui.renaming = Some(Naming::new(Some(id), "New folder"));
    }
    let save = bar.add_enabled_ui(selected, |ui| small_button(ui, "Save selection").on_hover_text("Save the selected clips, with their tracks and settings, to the library")).inner;
    let mut name = std::mem::take(&mut app.library_ui.name);
    let edit = bar.add(TextEdit::singleline(&mut name).hint_text(fallback.clone()).desired_width(bar.available_width()));
    let entered = edit.lost_focus() && bar.input(|i| i.key_pressed(Key::Enter));
    if selected && (save.clicked() || entered) {
        let label = if name.trim().is_empty() { fallback } else { name.trim().to_string() };
        if app.save_selection_to_library(&label) {
            name.clear();
        }
    }
    app.library_ui.name = name;
}

fn apply(app: &mut App, action: Action) {
    match action {
        Action::Open(id) => app.store.library_folder_id = id,
        Action::Add(id) => app.apply_library_item(&id, app.store.playhead_sec),
        Action::Update(id) => app.update_library_item_from_selection(&id),
        Action::Rename(id, name) => app.library_ui.renaming = Some(Naming::new(Some(id), &name)),
        Action::Recolor(id, color) => app.recolor_library_entry(&id, color),
        Action::DeleteItem(id) => app.delete_library_item(&id),
        Action::DeleteFolder(id) => app.delete_library_folder(&id),
        Action::Move(id, folder) => {
            app.move_library_entry(&id, folder.as_deref());
            app.timeline.library_drag = None;
        }
    }
}

pub fn drag_label(app: &App) -> Option<String> {
    entry_name(&app.store.library, app.timeline.library_drag.as_deref()?)
}

pub fn show(ui: &mut Ui, app: &mut App, rect: Rect) {
    let current = app.store.library_folder_id.clone();
    if current.is_some() && folder_path(&app.store.library, current.as_deref()).is_empty() {
        app.store.library_folder_id = None;
    }
    save_row(ui, app, Rect::from_min_size(rect.min, vec2(rect.width(), SAVE_H)));
    let crumb_rect = Rect::from_min_size(pos2(rect.left(), rect.top() + SAVE_H + 6.0), vec2(rect.width(), CRUMB_H));
    let mut action = crumbs(ui, app, crumb_rect);
    let list_rect = Rect::from_min_max(pos2(rect.left(), crumb_rect.bottom() + 4.0), rect.max);
    let shown = entries(&app.store.library, app.store.library_folder_id.as_deref());
    if shown.is_empty() {
        let galley = ui.painter().layout(HINT.to_string(), FontId::proportional(11.0), DIM, list_rect.width() - 12.0);
        ui.painter().galley(list_rect.min + vec2(6.0, 10.0), galley, DIM);
    } else {
        let mut list = ui.new_child(UiBuilder::new().max_rect(list_rect));
        ScrollArea::vertical().id_salt("library").scroll_source(scroll_area::ScrollSource::MOUSE_WHEEL | scroll_area::ScrollSource::SCROLL_BAR).auto_shrink([false, false]).show(&mut list, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for entry in &shown {
                action = row(ui, app, entry).or(action.take());
            }
        });
    }
    if let Some(action) = action {
        apply(app, action);
    }
}
