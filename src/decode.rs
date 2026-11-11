use crate::ffmpeg::{ffmpeg, spawn_piped};
use crate::raster::{Image, shrink};
use std::collections::HashMap;
use std::io::Read;
use std::process::Child;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::thread;

const READ_AHEAD: usize = 6;
const MAX_JUMP_SEC: f64 = 2.0;
const IDLE_TICKS: u64 = 3;
const MAX_STILL_SIDE: usize = 4096;

pub fn frame_index(start_sec: f64, rate: f64, sec: f64) -> i64 {
    ((sec - start_sec) * rate + 1e-6).floor() as i64
}

pub fn reusable(current: Option<i64>, target: i64, rate: f64) -> bool {
    target >= current.unwrap_or(0) && (target - current.unwrap_or(0)) as f64 <= MAX_JUMP_SEC * rate
}

struct Decoder {
    path: String,
    w: usize,
    h: usize,
    start_sec: f64,
    child: Child,
    frames: Receiver<Image>,
    index: Option<i64>,
    current: Option<Arc<Image>>,
    ended: bool,
    used_tick: u64,
}

impl Decoder {
    fn open(path: &str, sec: f64, w: usize, h: usize, rate: f64) -> Option<Decoder> {
        let args: Vec<String> =
            ["-v", "error", "-ss", &format!("{:.6}", sec.max(0.0)), "-i", path, "-an", "-sn", "-dn", "-vf", &format!("fps={rate},scale={w}:{h}:flags=area"), "-f", "rawvideo", "-pix_fmt", "rgba", "-"]
                .map(String::from)
                .into();
        let mut child = spawn_piped(&ffmpeg(), &args).ok()?;
        let mut stdout = child.stdout.take()?;
        let (send, frames) = sync_channel(READ_AHEAD);
        thread::spawn(move || {
            loop {
                let mut px = vec![0u8; w * h * 4];
                if stdout.read_exact(&mut px).is_err() || send.send(Image { w, h, px }).is_err() {
                    break;
                }
            }
        });
        Some(Decoder { path: path.into(), w, h, start_sec: sec.max(0.0), child, frames, index: None, current: None, ended: false, used_tick: 0 })
    }

    fn advance(&mut self, target: i64, wait: bool) {
        while !self.ended && self.index.is_none_or(|i| i < target) {
            let next = if wait { self.frames.recv().map_err(|_| TryRecvError::Disconnected) } else { self.frames.try_recv() };
            match next {
                Ok(frame) => {
                    self.current = Some(Arc::new(frame));
                    self.index = Some(self.index.map_or(0, |i| i + 1));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => self.ended = true,
            }
        }
    }

    fn picture(&self) -> Option<Arc<Image>> {
        match &self.current {
            Some(frame) => Some(frame.clone()),
            None if self.ended => Some(Arc::new(Image::new(0, 0))),
            None => None,
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Pictures {
    rate: f64,
    pub wait: bool,
    tick: u64,
    decoders: Vec<Decoder>,
    stills: HashMap<String, Arc<Image>>,
}

impl Pictures {
    pub fn new(rate: f64, wait: bool) -> Pictures {
        Pictures { rate, wait, tick: 0, decoders: Vec::new(), stills: HashMap::new() }
    }

    pub fn begin_frame(&mut self) {
        self.tick += 1;
        let tick = self.tick;
        self.decoders.retain(|d| tick - d.used_tick <= IDLE_TICKS);
    }

    pub fn video(&mut self, path: &str, sec: f64, w: usize, h: usize) -> Option<Arc<Image>> {
        let (rate, tick) = (self.rate, self.tick);
        let fits = |d: &Decoder| d.used_tick != tick && d.path == path && d.w == w && d.h == h && reusable(d.index, frame_index(d.start_sec, rate, sec), rate);
        let found = self.decoders.iter().position(fits);
        let i = match found {
            Some(i) => i,
            None => match Decoder::open(path, sec, w, h, rate) {
                Some(decoder) => {
                    self.decoders.push(decoder);
                    self.decoders.len() - 1
                }
                None => return Some(Arc::new(Image::new(0, 0))),
            },
        };
        let decoder = &mut self.decoders[i];
        decoder.used_tick = tick;
        decoder.advance(frame_index(decoder.start_sec, rate, sec), self.wait);
        decoder.picture()
    }

    pub fn still(&mut self, path: &str) -> Arc<Image> {
        self.stills.entry(path.to_string()).or_insert_with(|| Arc::new(load_still(path))).clone()
    }
}

pub fn load_still(path: &str) -> Image {
    let Ok(img) = image::open(path) else {
        return Image::new(0, 0);
    };
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let full = Image::from_straight(w, h, rgba.into_raw());
    shrink(&full, w.max(h).div_ceil(MAX_STILL_SIDE))
}
