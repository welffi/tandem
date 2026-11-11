use crate::edits::{blank_project, carry_mix, update_track};
use crate::media::Thumbnails;
use crate::timeline::project_duration;
use crate::types::{Library, Peaks, Project, TrackKind};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const HISTORY_LIMIT: usize = 200;
const COALESCE: Duration = Duration::from_millis(1500);
const TOAST_FOR: Duration = Duration::from_secs(6);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Player,
    Editor,
}

pub struct Toast {
    pub text: String,
    pub at: Instant,
}

pub fn stream_key(asset_id: &str, stream: usize) -> String {
    format!("{asset_id}:{stream}")
}

pub fn key_ref(clip_id: &str, key_id: &str) -> String {
    format!("{clip_id}/{key_id}")
}

pub struct Store {
    pub mode: Mode,
    project: Project,
    pub rev: u64,
    pub file_path: Option<PathBuf>,
    pub dirty: bool,
    past: Vec<Project>,
    future: Vec<Project>,
    history_key: Option<String>,
    history_at: Option<Instant>,
    pub selection: Vec<String>,
    pub selected_keys: Vec<String>,
    pub playhead_sec: f64,
    pub playing: bool,
    pub px_per_sec: f64,
    pub scroll_sec: f64,
    pub stage_width: f64,
    pub snapping: bool,
    pub master_volume: f32,
    pub master_muted: bool,
    pub peaks: HashMap<String, Peaks>,
    pub thumbs: HashMap<String, Thumbnails>,
    pub missing: HashSet<String>,
    pub toast: Option<Toast>,
    pub export_open: bool,
    pub library: Library,
    pub library_loaded: bool,
    pub library_folder_id: Option<String>,
}

impl Store {
    pub fn new() -> Store {
        Store {
            mode: Mode::Player,
            project: blank_project("Untitled"),
            rev: 0,
            file_path: None,
            dirty: false,
            past: Vec::new(),
            future: Vec::new(),
            history_key: None,
            history_at: None,
            selection: Vec::new(),
            selected_keys: Vec::new(),
            playhead_sec: 0.0,
            playing: false,
            px_per_sec: 80.0,
            scroll_sec: 0.0,
            stage_width: 800.0,
            snapping: true,
            master_volume: 1.0,
            master_muted: false,
            peaks: HashMap::new(),
            thumbs: HashMap::new(),
            missing: HashSet::new(),
            toast: None,
            export_open: false,
            library: Library::default(),
            library_loaded: false,
            library_folder_id: None,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    fn put(&mut self, project: Project) {
        self.project = project;
        self.rev += 1;
        let alive: HashSet<&str> = self.project.tracks.iter().flat_map(|t| &t.clips).map(|c| c.id.as_str()).collect();
        self.selection.retain(|id| alive.contains(id.as_str()));
    }

    pub fn edit(&mut self, next: Project, coalesce_key: Option<&str>) {
        if next == self.project {
            return;
        }
        let now = Instant::now();
        let coalesce = coalesce_key.is_some() && coalesce_key == self.history_key.as_deref() && self.history_at.is_some_and(|at| now - at < COALESCE);
        if !coalesce {
            if self.past.len() >= HISTORY_LIMIT {
                self.past.remove(0);
            }
            self.past.push(self.project.clone());
        }
        self.future.clear();
        self.history_key = coalesce_key.map(String::from);
        self.history_at = Some(now);
        self.dirty = true;
        self.put(next);
    }

    pub fn update(&mut self, coalesce_key: Option<&str>, change: impl FnOnce(&mut Project)) {
        let mut next = self.project.clone();
        change(&mut next);
        self.edit(next, coalesce_key);
    }

    pub fn change(&mut self, next: Project) {
        if self.mode == Mode::Editor {
            self.edit(next, None);
        } else {
            self.put(next);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn undo(&mut self) {
        let Some(previous) = self.past.pop() else { return };
        let project = carry_mix(&self.project, &previous);
        self.future.insert(0, std::mem::replace(&mut self.project, previous));
        self.history_key = None;
        self.dirty = true;
        self.put(project);
    }

    pub fn redo(&mut self) {
        if self.future.is_empty() {
            return;
        }
        let next = self.future.remove(0);
        let project = carry_mix(&self.project, &next);
        self.past.push(std::mem::replace(&mut self.project, next));
        self.history_key = None;
        self.dirty = true;
        self.put(project);
    }

    pub fn load_project(&mut self, project: Project, file_path: Option<PathBuf>) {
        self.file_path = file_path;
        self.dirty = false;
        self.past.clear();
        self.future.clear();
        self.history_key = None;
        self.selection.clear();
        self.playhead_sec = 0.0;
        self.playing = false;
        self.scroll_sec = 0.0;
        self.put(project);
    }

    pub fn rename(&mut self, name: &str) {
        let mut next = self.project.clone();
        next.name = name.to_string();
        self.put(next);
    }

    pub fn mark_saved(&mut self, file_path: PathBuf) {
        self.file_path = Some(file_path);
        self.dirty = false;
    }

    pub fn set_mix(&mut self, track_id: &str, muted: Option<bool>, volume: Option<f64>) {
        let next = update_track(&self.project, track_id, |t| {
            let mut t = t.clone();
            t.muted = muted.unwrap_or(t.muted);
            t.volume = volume.unwrap_or(t.volume);
            t
        });
        self.dirty |= self.mode == Mode::Editor && self.file_path.is_some();
        self.put(next);
    }

    pub fn set_selection(&mut self, ids: Vec<String>) {
        self.selected_keys.retain(|r| ids.iter().any(|id| r.split('/').next() == Some(id.as_str())));
        self.selection = ids;
    }

    pub fn reset_media(&mut self) {
        self.peaks.clear();
        self.thumbs.clear();
        self.missing.clear();
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast { text: text.into(), at: Instant::now() });
    }

    pub fn live_toast(&self) -> Option<&str> {
        self.toast.as_ref().filter(|t| t.at.elapsed() < TOAST_FOR).map(|t| t.text.as_str())
    }

    pub fn duration(&self) -> f64 {
        project_duration(&self.project)
    }

    pub fn audio_tracks(&self) -> impl Iterator<Item = &crate::types::Track> {
        self.project.tracks.iter().filter(|t| t.kind == TrackKind::Audio)
    }

    pub fn master_gain(&self) -> f32 {
        if self.master_muted { 0.0 } else { self.master_volume }
    }

    pub fn title(&self) -> String {
        if self.project.assets.is_empty() && self.mode == Mode::Player {
            return "Tandem".into();
        }
        format!("{}{} - Tandem", if self.dirty { "• " } else { "" }, self.project.name)
    }
}
