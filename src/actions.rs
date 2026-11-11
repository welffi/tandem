use crate::app::App;
use crate::assets::{file_name, probe_asset};
use crate::dialogs;
use crate::edits::{base_name, empty_project};
use crate::formats::{PROJECT_EXTENSION, is_project};
use crate::insert::{AudioTarget, InsertTarget, insert_asset, project_from_media};
use crate::jobs::{Done, Probed, Purpose};
use crate::media::{stream_peaks, thumbnails};
use crate::project_file::{OpenedProject, read_project, write_project};
use crate::store::{Mode, stream_key};
use crate::types::{MediaAsset, MediaKind, Project};
use rayon::prelude::*;
use std::path::PathBuf;

impl App {
    pub fn open_files(&mut self, paths: Vec<String>) {
        if let Some(project) = paths.iter().find(|p| is_project(p)) {
            self.open_project(Some(PathBuf::from(project)));
            return;
        }
        let purpose = if self.store.mode == Mode::Editor { Purpose::Import } else { Purpose::Open };
        self.probe(paths, purpose);
    }

    pub fn add_audio(&mut self, paths: Vec<String>) {
        let start_sec = if self.store.mode == Mode::Editor { self.store.playhead_sec } else { 0.0 };
        self.probe(paths, Purpose::AddAudio { start_sec });
    }

    pub fn pick_and_open(&mut self) {
        let paths = dialogs::pick_media();
        self.open_files(paths);
    }

    pub fn pick_and_add_audio(&mut self) {
        let paths = dialogs::pick_audio();
        self.add_audio(paths);
    }

    fn probe(&self, paths: Vec<String>, purpose: Purpose) {
        if paths.is_empty() {
            return;
        }
        let known = self.store.project().assets.clone();
        self.jobs.run(move || Done::Probed(purpose, paths.par_iter().map(|path| probe_asset(path, &known).map_err(|e| (path.clone(), e))).collect()));
    }

    pub fn finish_probe(&mut self, purpose: Purpose, probed: Probed) {
        let mut assets = Vec::new();
        for result in probed {
            match result {
                Ok(asset) => assets.push(asset),
                Err((path, error)) => self.store.notify(format!("Could not open {}: {error}", base_name(&path))),
            }
        }
        match purpose {
            Purpose::Import => assets.into_iter().for_each(|a| self.register_asset(a)),
            Purpose::AddAudio { start_sec } => assets.into_iter().for_each(|a| self.insert_audio(a, start_sec)),
            Purpose::Open => self.play_assets(assets),
        }
    }

    fn play_assets(&mut self, mut assets: Vec<MediaAsset>) {
        let video = assets.iter().position(|a| a.kind == MediaKind::Video).map(|i| assets.remove(i));
        let mut extras: Vec<MediaAsset> = assets.into_iter().filter(|a| !a.audio_streams.is_empty()).collect();
        if video.is_none() && self.store.duration() > 0.0 {
            extras.into_iter().for_each(|a| self.insert_audio(a, 0.0));
            return;
        }
        let Some(main) = video.or_else(|| (!extras.is_empty()).then(|| extras.remove(0))) else { return };
        if !self.confirm_discard() {
            return;
        }
        self.reset_media();
        self.store.load_project(project_from_media(&main), None);
        extras.into_iter().for_each(|a| self.insert_audio(a, 0.0));
        self.store.playing = true;
    }

    fn insert_audio(&mut self, asset: MediaAsset, start_sec: f64) {
        if asset.audio_streams.is_empty() {
            self.store.notify(format!("{} has no audio.", asset.name));
            return;
        }
        self.register_asset(asset.clone());
        let as_audio = MediaAsset { kind: MediaKind::Audio, ..asset };
        let target = InsertTarget { video_track_id: None, audio: AudioTarget::New };
        let (next, _) = insert_asset(self.store.project(), &as_audio, &target, start_sec);
        self.store.change(next);
    }

    fn register_asset(&mut self, asset: MediaAsset) {
        if self.store.project().assets.iter().any(|a| a.id == asset.id) {
            return;
        }
        let mut next = self.store.project().clone();
        next.assets.push(asset.clone());
        self.store.change(next);
        if self.store.mode == Mode::Editor {
            self.load_editor_data(&asset);
        }
    }

    fn reset_media(&mut self) {
        self.requested.clear();
        self.store.reset_media();
    }

    pub fn load_editor_data(&mut self, asset: &MediaAsset) {
        if asset.kind == MediaKind::Video && self.requested.insert(format!("thumbs:{}", asset.id)) {
            let (id, path, duration) = (asset.id.clone(), asset.source_path.clone(), asset.duration_sec);
            self.jobs.run(move || Done::Thumbs(id, thumbnails(&path, duration).ok()));
        }
        for stream in 0..asset.audio_streams.len() {
            let key = stream_key(&asset.id, stream);
            if self.requested.insert(format!("peaks:{key}")) {
                let path = asset.source_path.clone();
                self.jobs.run(move || Done::Peaks(key, stream_peaks(&path, stream).ok()));
            }
        }
    }

    pub fn finish_job(&mut self, done: Done) {
        match done {
            Done::Probed(purpose, probed) => self.finish_probe(purpose, probed),
            Done::Peaks(key, Some(peaks)) => drop(self.store.peaks.insert(key, peaks)),
            Done::Peaks(key, None) => drop(self.requested.remove(&format!("peaks:{key}"))),
            Done::Thumbs(id, Some(thumbs)) => drop(self.store.thumbs.insert(id, thumbs)),
            Done::Thumbs(id, None) => drop(self.requested.remove(&format!("thumbs:{id}"))),
            Done::Reversed(results) => self.finish_reverse(results),
            Done::LibraryLoaded(Ok(library)) => {
                self.store.library = library;
                self.store.library_loaded = true;
            }
            Done::LibraryLoaded(Err(error)) => self.store.notify(format!("Could not load the library: {error}")),
            Done::LibrarySaved(sent, Ok(saved)) => {
                if self.store.library == sent {
                    self.store.library = saved;
                }
            }
            Done::LibrarySaved(_, Err(error)) => self.store.notify(format!("Could not save the library: {error}")),
        }
    }

    pub fn enter_editor(&mut self) {
        if self.store.project().tracks.is_empty() {
            let project = Project { name: self.store.project().name.clone(), assets: self.store.project().assets.clone(), ..empty_project() };
            self.store.change(project);
        }
        self.store.mode = Mode::Editor;
        let ready: Vec<MediaAsset> = self.store.project().assets.iter().filter(|a| !self.store.missing.contains(&a.id)).cloned().collect();
        ready.iter().for_each(|a| self.load_editor_data(a));
    }

    pub fn leave_editor(&mut self) {
        self.store.mode = Mode::Player;
        self.store.set_selection(Vec::new());
    }

    pub fn confirm_discard(&self) -> bool {
        !self.store.dirty || dialogs::confirm("You have unsaved changes. Discard them?")
    }

    pub fn new_project(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        self.reset_media();
        let project = if self.store.mode == Mode::Editor { empty_project() } else { crate::edits::blank_project("Untitled") };
        self.store.load_project(project, None);
    }

    pub fn save_project(&mut self, save_as: bool) -> bool {
        let target = match (&self.store.file_path, save_as) {
            (Some(path), false) => path.clone(),
            _ => match dialogs::save_project_as(&self.store.project().name) {
                Some(path) => path,
                None => return false,
            },
        };
        let name = project_name_of(&target);
        if self.store.project().name != name {
            self.store.rename(&name);
        }
        match write_project(self.store.project(), &target) {
            Ok(()) => {
                self.store.mark_saved(target);
                true
            }
            Err(error) => {
                self.store.notify(format!("Save failed: {error}"));
                false
            }
        }
    }

    pub fn open_project(&mut self, path: Option<PathBuf>) {
        if !self.confirm_discard() {
            return;
        }
        let Some(path) = path.or_else(dialogs::pick_project) else { return };
        match read_project(&path) {
            Ok(opened) => self.load_opened(opened, false),
            Err(error) => self.store.notify(format!("Could not open project: {error}")),
        }
    }

    pub fn load_opened(&mut self, opened: OpenedProject, recovered: bool) {
        let (project, missing) = relink_missing(&opened);
        let changed = project != opened.project;
        self.reset_media();
        self.store.load_project(project, opened.file_path);
        if recovered {
            self.store.mode = Mode::Editor;
        }
        self.store.missing = missing.into_iter().collect();
        self.store.dirty = recovered || changed;
        if self.store.mode == Mode::Editor {
            let ready: Vec<MediaAsset> = self.store.project().assets.iter().filter(|a| !self.store.missing.contains(&a.id)).cloned().collect();
            ready.iter().for_each(|a| self.load_editor_data(a));
        }
    }

    pub fn relink_asset(&mut self, asset_id: &str) {
        let Some(asset) = self.store.project().assets.iter().find(|a| a.id == asset_id).cloned() else { return };
        let Some(path) = dialogs::locate(&asset.name) else { return };
        let relinked = MediaAsset { name: file_name(&path), source_path: path, ..asset };
        let mut next = self.store.project().clone();
        next.assets.iter_mut().filter(|a| a.id == asset_id).for_each(|a| *a = relinked.clone());
        self.store.change(next);
        self.store.missing.remove(asset_id);
        self.requested.retain(|r| !r.contains(asset_id));
        if self.store.mode == Mode::Editor {
            self.load_editor_data(&relinked);
        }
    }
}

pub fn project_name_of(path: &std::path::Path) -> String {
    let is_project = path.extension().is_some_and(|e| e.eq_ignore_ascii_case(PROJECT_EXTENSION));
    let name = if is_project { path.file_stem() } else { path.file_name() };
    name.map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn relink_missing(opened: &OpenedProject) -> (Project, Vec<String>) {
    let mut project = opened.project.clone();
    let mut missing = opened.missing_asset_ids.clone();
    for asset in opened.project.assets.iter().filter(|a| opened.missing_asset_ids.contains(&a.id)) {
        if !dialogs::confirm(&format!("\"{}\" could not be found at\n{}\n\nLocate it now?", asset.name, asset.source_path)) {
            continue;
        }
        let Some(path) = dialogs::locate(&asset.name) else { continue };
        missing.retain(|id| *id != asset.id);
        project.assets.iter_mut().filter(|a| a.id == asset.id).for_each(|a| a.source_path = path.clone());
    }
    (project, missing)
}
