use crate::compose::compose_video;
use crate::decode::Pictures;
use crate::raster::Image;
use crate::render_plan::{RenderPlan, build_render_plan};
use crate::text_render::{TextBounds, draw_texts};
use crate::types::Project;
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const RETRY: Duration = Duration::from_millis(4);

#[derive(Clone)]
struct Request {
    project: Arc<Project>,
    rev: u64,
    frame: i64,
    w: usize,
    h: usize,
    wait: bool,
}

struct Rendered {
    image: Image,
    texts: Vec<(String, TextBounds)>,
}

#[derive(Default)]
struct Shared {
    request: Mutex<Option<Request>>,
    wake: Condvar,
    done: Mutex<Option<Rendered>>,
    stop: AtomicBool,
}

pub fn frame_at(sec: f64, fps: f64) -> i64 {
    (sec * fps + 1e-6).floor() as i64
}

pub struct Preview {
    shared: Arc<Shared>,
    asked: Option<(u64, i64, usize, usize)>,
    snapshot: Option<(u64, Arc<Project>)>,
    pub texture: Option<TextureHandle>,
    pub texts: Vec<(String, TextBounds)>,
    pub size: (usize, usize),
}

impl Preview {
    pub fn new(ctx: &Context) -> Preview {
        let shared = Arc::new(Shared::default());
        let (worker, ctx) = (shared.clone(), ctx.clone());
        thread::spawn(move || render_loop(&worker, &ctx));
        Preview { shared, asked: None, snapshot: None, texture: None, texts: Vec::new(), size: (0, 0) }
    }

    pub fn request(&mut self, project: &Project, rev: u64, sec: f64, w: usize, h: usize, playing: bool) {
        let frame = frame_at(sec, project.fps);
        let key = (rev, frame, w, h);
        if self.asked == Some(key) || w == 0 || h == 0 {
            return;
        }
        self.asked = Some(key);
        if self.snapshot.as_ref().is_none_or(|(r, _)| *r != rev) {
            self.snapshot = Some((rev, Arc::new(project.clone())));
        }
        let project = self.snapshot.as_ref().unwrap().1.clone();
        *self.shared.request.lock().unwrap() = Some(Request { project, rev, frame, w, h, wait: !playing });
        self.shared.wake.notify_one();
    }

    pub fn poll(&mut self, ctx: &Context) {
        let Some(done) = self.shared.done.lock().unwrap().take() else { return };
        let (w, h) = (done.image.w, done.image.h);
        let image = ColorImage::from_rgba_premultiplied([w, h], &done.image.px);
        match &mut self.texture {
            Some(texture) => texture.set(image, TextureOptions::LINEAR),
            None => self.texture = Some(ctx.load_texture("preview", image, TextureOptions::LINEAR)),
        }
        self.texts = done.texts;
        self.size = (w, h);
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.shared.wake.notify_one();
    }
}

fn next_request(shared: &Shared) -> Option<Request> {
    let mut slot = shared.request.lock().unwrap();
    loop {
        if shared.stop.load(Ordering::SeqCst) {
            return None;
        }
        if let Some(request) = slot.take() {
            return Some(request);
        }
        slot = shared.wake.wait(slot).unwrap();
    }
}

fn render_loop(shared: &Shared, ctx: &Context) {
    let mut plan: Option<(u64, RenderPlan)> = None;
    let mut pics: Option<(f64, Pictures)> = None;
    while let Some(request) = next_request(shared) {
        if plan.as_ref().is_none_or(|(rev, _)| *rev != request.rev) {
            plan = Some((request.rev, build_render_plan(&request.project)));
        }
        let plan = &plan.as_ref().unwrap().1;
        if pics.as_ref().is_none_or(|(fps, _)| *fps != plan.fps) {
            pics = Some((plan.fps, Pictures::new(plan.fps, true)));
        }
        let pictures = &mut pics.as_mut().unwrap().1;
        pictures.wait = request.wait;
        let Some(mut image) = compose_video(plan, request.frame, request.w, request.h, pictures) else {
            let mut slot = shared.request.lock().unwrap();
            if slot.is_none() {
                *slot = Some(request);
            }
            drop(slot);
            thread::sleep(RETRY);
            continue;
        };
        let texts = draw_texts(&mut image, &request.project, request.frame as f64 / plan.fps);
        *shared.done.lock().unwrap() = Some(Rendered { image, texts });
        ctx.request_repaint();
    }
}
