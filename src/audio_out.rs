use crate::export_audio::{Span, stream_track};
use crate::ffmpeg::Cancel;
use crate::mix::{Limiter, Mix, Voice};
use crate::render_plan::{AudioPiece, AudioTrackPlan, build_render_plan};
use crate::types::Project;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const QUEUED_CHUNKS: usize = 8;
const NULL_TICK_MS: u64 = 10;
const FALLBACK_RATE: f64 = 48000.0;

struct ChannelWriter(SyncSender<Vec<u8>>);

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.send(buf.to_vec()).map_err(|_| std::io::ErrorKind::BrokenPipe)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

enum Device {
    Speakers(cpal::Stream),
    Null(Arc<AtomicBool>),
}

impl Drop for Device {
    fn drop(&mut self) {
        if let Device::Null(stop) = self {
            stop.store(true, Ordering::SeqCst);
        }
    }
}

fn speakers(mix: &Arc<Mutex<Mix>>) -> Option<(Device, f64)> {
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    if config.sample_format() != cpal::SampleFormat::F32 {
        return None;
    }
    let (rate, channels) = (config.sample_rate().0 as f64, config.channels() as usize);
    let shared = mix.clone();
    let stream = device.build_output_stream(&config.config(), move |out: &mut [f32], _: &cpal::OutputCallbackInfo| shared.lock().unwrap().fill(out, channels), |_| {}, None).ok()?;
    stream.play().ok()?;
    Some((Device::Speakers(stream), rate))
}

fn null_device(mix: &Arc<Mutex<Mix>>) -> Device {
    let stop = Arc::new(AtomicBool::new(false));
    let (shared, stopped) = (mix.clone(), stop.clone());
    thread::spawn(move || {
        let mut buffer = vec![0f32; (FALLBACK_RATE as u64 * NULL_TICK_MS / 1000) as usize * 2];
        while !stopped.load(Ordering::SeqCst) {
            shared.lock().unwrap().fill(&mut buffer, 2);
            thread::sleep(Duration::from_millis(NULL_TICK_MS));
        }
    });
    Device::Null(stop)
}

type PlanKey = Vec<(String, Vec<AudioPiece>)>;

fn plan_key(tracks: &[AudioTrackPlan]) -> PlanKey {
    tracks.iter().map(|t| (t.id.clone(), t.pieces.clone())).collect()
}

pub struct Player {
    mix: Arc<Mutex<Mix>>,
    _device: Device,
    rate: f64,
    dir: PathBuf,
    cancel: Cancel,
    key: PlanKey,
    from_sec: f64,
}

impl Player {
    pub fn new() -> Player {
        let mix = Arc::new(Mutex::new(Mix::new(FALLBACK_RATE)));
        let (device, rate) = speakers(&mix).unwrap_or_else(|| (null_device(&mix), FALLBACK_RATE));
        let dir = std::env::temp_dir().join(format!("tandem-play-{}", std::process::id()));
        Player { mix, _device: device, rate, dir, cancel: Cancel::default(), key: Vec::new(), from_sec: 0.0 }
    }

    pub fn time(&self) -> f64 {
        self.from_sec + self.mix.lock().unwrap().frames as f64 / self.rate
    }

    pub fn is_playing(&self) -> bool {
        self.mix.lock().unwrap().playing
    }

    pub fn play(&mut self, project: &Project, from_sec: f64, master: f32) {
        self.start(project, from_sec);
        let mut mix = self.mix.lock().unwrap();
        mix.master = master;
        mix.playing = true;
    }

    pub fn pause(&mut self) {
        let at = self.time();
        self.stop_voices();
        let mut mix = self.mix.lock().unwrap();
        mix.playing = false;
        mix.frames = 0;
        self.from_sec = at;
    }

    pub fn sync(&mut self, project: &Project, master: f32) {
        let plan = build_render_plan(project);
        if self.is_playing() && plan_key(&plan.audio) != self.key {
            let at = self.time();
            self.start(project, at);
        }
        let mut mix = self.mix.lock().unwrap();
        mix.master = master;
        mix.limiter = Limiter::new(project.limit_db, self.rate);
        for voice in &mut mix.voices {
            voice.gain = plan.audio.iter().find(|t| t.id == voice.id).map_or(0.0, |t| t.volume as f32);
        }
    }

    pub fn take_levels(&self, key: &str) -> [f32; 2] {
        self.mix.lock().unwrap().take_peak(key)
    }

    fn stop_voices(&mut self) {
        self.cancel.cancel();
        self.mix.lock().unwrap().voices.clear();
    }

    fn start(&mut self, project: &Project, from_sec: f64) {
        self.stop_voices();
        let plan = build_render_plan(project);
        let _ = std::fs::create_dir_all(&self.dir);
        self.cancel = Cancel::default();
        self.key = plan_key(&plan.audio);
        let voices = plan
            .audio
            .into_iter()
            .enumerate()
            .map(|(index, track)| {
                let (send, chunks) = sync_channel(QUEUED_CHUNKS);
                let (rate, dir, cancel, to_sec) = (self.rate, self.dir.clone(), self.cancel.clone(), plan.duration_sec);
                let voice = Voice::new(track.id.clone(), track.volume as f32, chunks);
                thread::spawn(move || {
                    let span = Span { rate, from_sec, to_sec, dir: &dir, cancel: &cancel };
                    let _ = stream_track(&track, index, 1.0, &span, &mut ChannelWriter(send));
                });
                voice
            })
            .collect();
        let mut mix = self.mix.lock().unwrap();
        mix.voices = voices;
        mix.limiter = Limiter::new(project.limit_db, self.rate);
        mix.frames = 0;
        self.from_sec = from_sec;
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop_voices();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
