use crate::media::Thumbnails;
use crate::reverse::Range;
use crate::types::{Library, MediaAsset, Peaks};
use eframe::egui::Context;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Purpose {
    Open,
    AddAudio { start_sec: f64 },
    Import,
}

pub type Reversed = Result<(Range, MediaAsset), String>;

pub type Probed = Vec<Result<MediaAsset, (String, String)>>;

pub enum Done {
    Probed(Purpose, Probed),
    Peaks(String, Option<Peaks>),
    Thumbs(String, Option<Thumbnails>),
    Reversed(Vec<Reversed>),
    LibraryLoaded(Result<Library, String>),
    LibrarySaved(Library, Result<Library, String>),
}

pub struct Jobs {
    send: Sender<Done>,
    recv: Receiver<Done>,
    ctx: Context,
}

impl Jobs {
    pub fn new(ctx: &Context) -> Jobs {
        let (send, recv) = channel();
        Jobs { send, recv, ctx: ctx.clone() }
    }

    pub fn run(&self, work: impl FnOnce() -> Done + Send + 'static) {
        let (send, ctx) = (self.send.clone(), self.ctx.clone());
        thread::spawn(move || {
            let _ = send.send(work());
            ctx.request_repaint();
        });
    }

    pub fn finished(&self) -> Vec<Done> {
        self.recv.try_iter().collect()
    }
}
