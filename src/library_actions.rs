use crate::app::App;
use crate::curve_math::{CurveChange, changed};
use crate::dialogs;
use crate::ids::new_id;
use crate::jobs::Done;
use crate::library::{load_library, save_library};
use crate::library_edit::{add_folder, add_item, child_count, delete_folder, delete_item, move_to, rename, replace_item, set_color};
use crate::presets::{apply_preset, make_preset, preset_from_asset};
use crate::types::Library;

impl App {
    pub fn load_library(&self) {
        self.jobs.run(|| Done::LibraryLoaded(load_library()));
    }

    fn store_library(&mut self, change: impl FnOnce(&Library) -> Library) {
        if !self.store.library_loaded {
            self.store.notify("The library has not loaded yet");
            return;
        }
        let next = change(&self.store.library);
        self.store.library = next.clone();
        self.jobs.run(move || Done::LibrarySaved(next.clone(), save_library(&next)));
    }

    pub fn store_curves(&mut self, change: &CurveChange) {
        self.store_library(|library| Library { curves: Some(changed(library.curves.as_deref().unwrap_or(&[]), change)), ..library.clone() });
    }

    pub fn save_selection_to_library(&mut self, name: &str) -> bool {
        let Some(item) = make_preset(self.store.project(), &self.store.selection, name) else { return false };
        let folder = self.store.library_folder_id.clone();
        self.store_library(|library| add_item(library, item, folder));
        true
    }

    pub fn save_asset_to_library(&mut self, asset_id: &str) {
        let Some(item) = self.store.project().assets.iter().find(|a| a.id == asset_id).and_then(preset_from_asset) else { return };
        let (name, folder) = (item.name.clone(), self.store.library_folder_id.clone());
        self.store_library(|library| add_item(library, item, folder));
        self.store.notify(format!("Saved {name} to the library"));
    }

    pub fn update_library_item_from_selection(&mut self, id: &str) {
        let Some(fresh) = make_preset(self.store.project(), &self.store.selection, "") else { return };
        let name = self.store.library.items.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        if dialogs::confirm(&format!("Replace what \"{name}\" holds with the selected clips?")) {
            self.store_library(|library| replace_item(library, id, fresh));
        }
    }

    pub fn new_library_folder(&mut self) -> String {
        let (id, parent) = (new_id(), self.store.library_folder_id.clone());
        let made = id.clone();
        self.store_library(|library| add_folder(library, made, parent));
        id
    }

    pub fn rename_library_entry(&mut self, id: &str, name: &str) {
        self.store_library(|library| rename(library, id, name));
    }

    pub fn recolor_library_entry(&mut self, id: &str, color: Option<String>) {
        self.store_library(|library| set_color(library, id, color));
    }

    pub fn move_library_entry(&mut self, id: &str, folder_id: Option<&str>) {
        self.store_library(|library| move_to(library, id, folder_id));
    }

    pub fn delete_library_item(&mut self, id: &str) {
        let name = self.store.library.items.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        if dialogs::confirm(&format!("Delete \"{name}\" from the library?")) {
            self.store_library(|library| delete_item(library, id));
        }
    }

    pub fn delete_library_folder(&mut self, id: &str) {
        let count = child_count(&self.store.library, id);
        let name = self.store.library.folders.iter().find(|f| f.id == id).map(|f| f.name.clone()).unwrap_or_default();
        if count == 0 || dialogs::confirm(&format!("Delete the folder \"{name}\"? Its {count} item(s) move up one level.")) {
            self.store_library(|library| delete_folder(library, id));
        }
    }

    pub fn apply_library_item(&mut self, id: &str, at_sec: f64) {
        let Some(item) = self.store.library.items.iter().find(|p| p.id == id) else { return };
        let applied = apply_preset(self.store.project(), item, at_sec);
        self.store.edit(applied.project, None);
        applied.new_assets.iter().for_each(|asset| self.load_editor_data(asset));
        self.store.set_selection(applied.clip_ids);
    }
}
