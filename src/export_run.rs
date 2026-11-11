use crate::export::export_project;
use crate::ffmpeg::{CANCELLED, Cancel};
use crate::types::{ExportPhase, ExportProgress, ExportSettings, Project};
use eframe::egui::Context;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ExportRun {
    shared: Arc<Mutex<ExportProgress>>,
    cancel: Cancel,
    pub output: PathBuf,
}

impl ExportRun {
    pub fn start(ctx: &Context, project: Project, settings: ExportSettings) -> ExportRun {
        let shared = Arc::new(Mutex::new(ExportProgress { phase: ExportPhase::Audio, fraction: 0.0 }));
        let cancel = Cancel::default();
        let output = settings.output_path.clone();
        let (slot, stop, repaint) = (shared.clone(), cancel.clone(), ctx.clone());
        thread::spawn(move || {
            let report = |progress: ExportProgress| {
                *slot.lock().unwrap() = progress;
                repaint.request_repaint();
            };
            match export_project(&project, &settings, &stop, &report) {
                Ok(()) => {}
                Err(error) if error == CANCELLED => report(ExportProgress { phase: ExportPhase::Cancelled, fraction: 0.0 }),
                Err(error) => report(ExportProgress { phase: ExportPhase::Error(error), fraction: 0.0 }),
            }
        });
        ExportRun { shared, cancel, output }
    }

    pub fn progress(&self) -> ExportProgress {
        self.shared.lock().unwrap().clone()
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }
}
