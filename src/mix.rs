use crate::formats::{MIN_LIMIT_DB, db_to_gain};
use std::sync::mpsc::{Receiver, TryRecvError};

pub const MASTER: &str = "master";
const RATIO: f32 = 20.0;
const ATTACK_SEC: f64 = 0.002;
const RELEASE_SEC: f64 = 0.08;

pub struct Limiter {
    threshold: f32,
    env: f32,
    attack: f32,
    release: f32,
}

impl Limiter {
    pub fn new(limit_db: Option<f64>, rate: f64) -> Limiter {
        let threshold = limit_db.map_or(f32::INFINITY, |db| db_to_gain(db.clamp(MIN_LIMIT_DB, 0.0)) as f32);
        let coef = |sec: f64| (-1.0 / (sec * rate)).exp() as f32;
        Limiter { threshold, env: 0.0, attack: coef(ATTACK_SEC), release: coef(RELEASE_SEC) }
    }

    pub fn gain(&mut self, peak: f32) -> f32 {
        let coef = if peak > self.env { self.attack } else { self.release };
        self.env = peak + (self.env - peak) * coef;
        if self.env <= self.threshold {
            return 1.0;
        }
        self.threshold * (self.env / self.threshold).powf(1.0 / RATIO) / self.env
    }
}

pub struct Voice {
    pub id: String,
    pub gain: f32,
    pub peak: [f32; 2],
    chunks: Receiver<Vec<u8>>,
    pending: Vec<f32>,
    pos: usize,
    ended: bool,
}

impl Voice {
    pub fn new(id: String, gain: f32, chunks: Receiver<Vec<u8>>) -> Voice {
        Voice { id, gain, peak: [0.0; 2], chunks, pending: Vec::new(), pos: 0, ended: false }
    }

    fn ready(&mut self) -> bool {
        while !self.ended && self.pos + 1 >= self.pending.len() {
            match self.chunks.try_recv() {
                Ok(chunk) => {
                    self.pending = chunk.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).collect();
                    self.pos = 0;
                }
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => self.ended = true,
            }
        }
        true
    }

    fn next(&mut self) -> (f32, f32) {
        if self.pos + 1 >= self.pending.len() {
            return (0.0, 0.0);
        }
        let (l, r) = (self.pending[self.pos] * self.gain, self.pending[self.pos + 1] * self.gain);
        self.pos += 2;
        note_peak(&mut self.peak, l, r);
        (l, r)
    }
}

fn note_peak(peak: &mut [f32; 2], l: f32, r: f32) {
    peak[0] = peak[0].max(l.abs());
    peak[1] = peak[1].max(r.abs());
}

pub struct Mix {
    pub voices: Vec<Voice>,
    pub master: f32,
    pub limiter: Limiter,
    pub playing: bool,
    pub frames: u64,
    pub peak: [f32; 2],
}

impl Mix {
    pub fn new(rate: f64) -> Mix {
        Mix { voices: Vec::new(), master: 1.0, limiter: Limiter::new(None, rate), playing: false, frames: 0, peak: [0.0; 2] }
    }

    pub fn fill(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        if !self.playing || channels == 0 {
            return;
        }
        for frame in out.chunks_mut(channels) {
            if !self.voices.iter_mut().all(Voice::ready) {
                return;
            }
            let (mut l, mut r) = (0.0, 0.0);
            for voice in &mut self.voices {
                let (a, b) = voice.next();
                l += a;
                r += b;
            }
            let g = self.limiter.gain((l * self.master).abs().max((r * self.master).abs())) * self.master;
            let (l, r) = ((l * g).clamp(-1.0, 1.0), (r * g).clamp(-1.0, 1.0));
            note_peak(&mut self.peak, l, r);
            if channels == 1 {
                frame[0] = (l + r) / 2.0;
            } else {
                frame[0] = l;
                frame[1] = r;
            }
            self.frames += 1;
        }
    }

    pub fn take_peak(&mut self, key: &str) -> [f32; 2] {
        let peak = if key == MASTER { Some(&mut self.peak) } else { self.voices.iter_mut().find(|v| v.id == key).map(|v| &mut v.peak) };
        peak.map(std::mem::take).unwrap_or_default()
    }
}
