use crate::audio_out::Player;
use crate::dialogs;
use crate::export_dialog::ExportState;
use crate::ffmpeg::installed;
use crate::ffmpeg_install::{self, InstallState};
use crate::inspector::InspectorState;
use crate::jobs::Jobs;
use crate::keyframe_panel::KeyframeState;
use crate::level_meter::Meter;
use crate::library_view::LibraryState;
use crate::mix::MASTER;
use crate::preview::Preview;
use crate::preview_overlay::Session;
use crate::project_file::{clear_recovery, take_recovery, write_recovery};
use crate::store::{Mode, Store};
use crate::timeline_state::TimelineState;
use crate::{editor_view, keys, overlays, player_view, theme};
use eframe::egui::{self, Context, ViewportCommand};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

const SEEK_SETTLE: Duration = Duration::from_millis(80);
const RECOVERY_DELAY: Duration = Duration::from_millis(500);

pub struct App {
    pub store: Store,
    pub audio: Player,
    pub preview: Preview,
    pub jobs: Jobs,
    pub requested: HashSet<String>,
    pub meters: HashMap<String, Meter>,
    pub mixer_open: bool,
    pub export: ExportState,
    pub inspector: InspectorState,
    pub keyframes: KeyframeState,
    pub library_ui: LibraryState,
    pub preview_drag: Option<Session>,
    pub timeline: TimelineState,
    pub last_activity: Instant,
    inbox: Receiver<Vec<String>>,
    install: Option<ffmpeg_install::Shared>,
    launch: Option<Vec<String>>,
    title: String,
    seek_pending: Option<Instant>,
    synced: Option<(u64, f32)>,
    recovery_seen: (u64, bool, Option<PathBuf>),
    recovery_due: Option<Instant>,
}

impl App {
    pub fn new(ctx: &Context, launch: Vec<String>) -> App {
        theme::apply(ctx);
        let repaint = ctx.clone();
        let inbox = crate::win::watch_inbox(move || repaint.request_repaint());
        let install = (!installed()).then(|| {
            let repaint = ctx.clone();
            ffmpeg_install::start(move || repaint.request_repaint())
        });
        let app = App {
            store: Store::new(),
            audio: Player::new(),
            preview: Preview::new(ctx),
            jobs: Jobs::new(ctx),
            requested: HashSet::new(),
            meters: HashMap::new(),
            mixer_open: true,
            export: ExportState::default(),
            inspector: InspectorState::default(),
            keyframes: KeyframeState::default(),
            library_ui: LibraryState::default(),
            preview_drag: None,
            timeline: TimelineState::default(),
            last_activity: Instant::now(),
            inbox,
            install,
            launch: Some(launch),
            title: String::new(),
            seek_pending: None,
            synced: None,
            recovery_seen: (0, false, None),
            recovery_due: None,
        };
        app.load_library();
        app
    }

    pub fn install_state(&self) -> Option<InstallState> {
        self.install.as_ref().map(|s| s.lock().unwrap().clone()).filter(|s| *s != InstallState::Done)
    }

    pub fn retry_install(&mut self, ctx: &Context) {
        let repaint = ctx.clone();
        self.install = Some(ffmpeg_install::start(move || repaint.request_repaint()));
    }

    pub fn seek(&mut self, sec: f64) {
        let limit = if self.store.mode == Mode::Editor { f64::MAX } else { self.store.duration() };
        self.store.playhead_sec = sec.clamp(0.0, limit);
        if self.store.playing {
            self.audio.pause();
            self.seek_pending = Some(Instant::now());
        }
    }

    pub fn toggle_play(&mut self) {
        self.store.playing = !self.store.playing;
    }

    fn start_up(&mut self) {
        let Some(paths) = self.launch.take() else { return };
        if let Some(opened) = take_recovery() {
            let name = opened.file_path.as_deref().map_or(opened.project.name.clone(), |p| crate::assets::file_name(&p.to_string_lossy()));
            if dialogs::confirm(&format!("Tandem closed before \"{name}\" was saved. Restore your unsaved changes?")) {
                self.load_opened(opened, true);
            } else {
                clear_recovery();
            }
        }
        self.open_files(paths);
    }

    fn tick_playback(&mut self, ctx: &Context) {
        if let Some(at) = self.seek_pending {
            if at.elapsed() < SEEK_SETTLE {
                ctx.request_repaint_after(SEEK_SETTLE);
                return;
            }
            self.seek_pending = None;
            if self.store.playing {
                self.start_audio();
            }
        }
        let duration = self.store.duration();
        if self.store.playing != self.audio.is_playing() {
            if !self.store.playing {
                self.audio.pause();
            } else if duration <= 0.0 {
                self.store.playing = false;
            } else {
                if self.store.playhead_sec >= duration - 1e-3 {
                    self.store.playhead_sec = 0.0;
                }
                self.start_audio();
            }
        }
        if !self.store.playing {
            return;
        }
        let state = (self.store.rev, self.store.master_gain());
        if self.synced != Some(state) {
            self.audio.sync(self.store.project(), state.1);
            self.synced = Some(state);
        }
        let time = self.audio.time();
        if time >= duration {
            self.store.playing = false;
            self.audio.pause();
        }
        self.store.playhead_sec = time.min(duration);
        ctx.request_repaint();
    }

    fn start_audio(&mut self) {
        self.audio.play(self.store.project(), self.store.playhead_sec, self.store.master_gain());
        self.synced = Some((self.store.rev, self.store.master_gain()));
    }

    fn tick_meters(&mut self, ctx: &Context) {
        let now = Instant::now();
        let keys: Vec<String> = self.store.audio_tracks().map(|t| t.id.clone()).chain(Some(MASTER.to_string())).collect();
        self.meters.retain(|k, _| keys.contains(k));
        for key in keys {
            let levels = self.audio.take_levels(&key);
            self.meters.entry(key).or_default().update(levels, now);
        }
        if self.meters.values().any(|m| !m.is_resting()) {
            ctx.request_repaint();
        }
    }

    fn tick_recovery(&mut self, ctx: &Context) {
        let seen = (self.store.rev, self.store.dirty, self.store.file_path.clone());
        if seen != self.recovery_seen {
            self.recovery_seen = seen;
            if self.store.dirty {
                self.recovery_due = Some(Instant::now() + RECOVERY_DELAY);
            } else {
                self.recovery_due = None;
                clear_recovery();
            }
        }
        let Some(due) = self.recovery_due else { return };
        if Instant::now() < due {
            ctx.request_repaint_after(due - Instant::now());
            return;
        }
        self.recovery_due = None;
        let _ = write_recovery(self.store.project(), self.store.file_path.as_deref());
    }

    fn tick_files(&mut self, ctx: &Context) {
        for done in self.jobs.finished() {
            self.finish_job(done);
        }
        let dropped: Vec<String> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_string_lossy().into_owned()).filter(|p| !p.is_empty()).collect());
        if !dropped.is_empty() {
            self.open_files(dropped);
        }
        while let Ok(paths) = self.inbox.try_recv() {
            self.open_files(paths);
        }
    }

    fn update_title(&mut self, ctx: &Context) {
        let title = self.store.title();
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    pub fn is_fullscreen(ctx: &Context) -> bool {
        ctx.input(|i| i.viewport().fullscreen.unwrap_or(false))
    }

    pub fn toggle_fullscreen(ctx: &Context) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!App::is_fullscreen(ctx)));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(state) = self.install_state() {
            overlays::install_screen(ui, self, &state);
            return;
        }
        self.start_up();
        self.tick_files(&ctx);
        keys::handle(self, &ctx);
        self.tick_playback(&ctx);
        self.tick_meters(&ctx);
        self.tick_recovery(&ctx);
        self.update_title(&ctx);
        self.preview.poll(&ctx);
        match self.store.mode {
            Mode::Player => player_view::show(ui, self),
            Mode::Editor => editor_view::show(ui, self),
        }
        overlays::drop_hint(ui, self.store.mode);
        overlays::toast(ui, &mut self.store);
    }
}
