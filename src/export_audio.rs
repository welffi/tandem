use crate::ffmpeg::{Cancel, ffmpeg, run, run_streaming};
use crate::formats::limiter_filter;
use crate::render_plan::{AudioPiece, AudioTrackPlan, RenderPlan};
use crate::types::AudioMode;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

pub const AUDIO_RATE: f64 = 48000.0;
const FRAME_BYTES: usize = 4;
const EDGE_FADE_FRAMES: i64 = 64;

pub fn edge_gain(frame: i64, frames: i64, fade: i64, fade_in: f64, fade_out: f64) -> f64 {
    let edges = (frame as f64 / fade as f64).min((frames - 1 - frame) as f64 / fade as f64);
    let user_in = if fade_in > 0.0 { frame as f64 / fade_in } else { 1.0 };
    let user_out = if fade_out > 0.0 { (frames - frame) as f64 / fade_out } else { 1.0 };
    1f64.min(edges).min(user_in).min(user_out)
}

pub struct Shaper {
    pub frame: i64,
    frames: i64,
    skip: i64,
    fade: i64,
    fade_in: f64,
    fade_out: f64,
    gain: f64,
    carry: Vec<u8>,
}

impl Shaper {
    pub fn new(frames: i64, skip: i64, fade_in: f64, fade_out: f64, gain: f64) -> Shaper {
        let fade = (frames >> 1).clamp(1, EDGE_FADE_FRAMES);
        Shaper { frame: 0, frames, skip, fade, fade_in, fade_out, gain, carry: Vec::new() }
    }

    pub fn starting_at(self, frame: i64) -> Shaper {
        Shaper { frame, ..self }
    }

    pub fn push(&mut self, chunk: &[u8], out: &mut Vec<u8>) {
        self.carry.extend_from_slice(chunk);
        let usable = self.carry.len() - self.carry.len() % FRAME_BYTES;
        for sample in self.carry[..usable].chunks_exact(FRAME_BYTES) {
            if self.frame >= self.skip && self.frame < self.frames {
                let gain = self.gain * edge_gain(self.frame, self.frames, self.fade, self.fade_in, self.fade_out);
                for half in sample.chunks_exact(2) {
                    let value = i16::from_le_bytes([half[0], half[1]]) as f64 * gain;
                    out.extend_from_slice(&(value.round().clamp(-32768.0, 32767.0) as i16).to_le_bytes());
                }
            }
            self.frame += 1;
        }
        self.carry.drain(..usable);
    }
}

fn write_silence(out: &mut impl Write, frames: i64, rate: f64) -> std::io::Result<()> {
    let block = vec![0u8; FRAME_BYTES * rate as usize];
    let mut left = frames.max(0) as usize * FRAME_BYTES;
    while left > 0 {
        let n = left.min(block.len());
        out.write_all(&block[..n])?;
        left -= n;
    }
    Ok(())
}

fn source_span(piece: &AudioPiece) -> f64 {
    match &piece.speed {
        Some(speed) => speed.points.last().map_or(0.0, |p| p.1) - piece.in_sec + 0.5,
        None => piece.duration_sec,
    }
}

pub fn speed_commands(piece: &AudioPiece) -> Option<String> {
    let speed = piece.speed.as_ref()?;
    let lines: Vec<String> = speed
        .points
        .windows(2)
        .filter(|w| w[1].0 - w[0].0 > 1e-9)
        .map(|w| {
            let rate = format!("{:.5}", (w[1].1 - w[0].1) / (w[1].0 - w[0].0));
            let pitch = if speed.keep_pitch { "1".to_string() } else { rate.clone() };
            format!("{:.6} rubberband tempo {rate}, rubberband pitch {pitch};", (w[0].1 - piece.in_sec).max(0.0))
        })
        .collect();
    Some(lines.join("\n"))
}

fn piece_args(piece: &AudioPiece, seek_sec: f64, rate: f64, commands_file: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error", "-ss"].map(String::from).into();
    args.extend([format!("{:.6}", piece.in_sec + seek_sec), "-t".into(), format!("{:.6}", source_span(piece) - seek_sec), "-i".into(), piece.path.clone()]);
    args.extend(["-map".into(), format!("0:a:{}", piece.stream)]);
    if let Some(file) = commands_file {
        args.extend(["-af".into(), format!("asendcmd=f={file},rubberband=tempo=1")]);
    }
    args.extend(["-ac".into(), "2".into(), "-ar".into(), format!("{rate}"), "-f".into(), "s16le".into(), "pipe:1".into()]);
    args
}

pub struct Span<'a> {
    pub rate: f64,
    pub from_sec: f64,
    pub to_sec: f64,
    pub dir: &'a Path,
    pub cancel: &'a Cancel,
}

pub fn stream_track(track: &AudioTrackPlan, key: usize, gain: f64, span: &Span, out: &mut impl Write) -> Result<(), String> {
    let rate = span.rate;
    let total = ((span.to_sec - span.from_sec) * rate).ceil() as i64;
    let io = |e: std::io::Error| e.to_string();
    let mut written = 0i64;
    for (index, piece) in track.pieces.iter().enumerate() {
        let start = ((piece.timeline_start_sec - span.from_sec) * rate).round() as i64;
        if start >= total {
            break;
        }
        if start > written {
            write_silence(out, start - written, rate).map_err(io)?;
            written = start;
        }
        let frames = ((piece.duration_sec * rate).round() as i64).min(total - start);
        let skip = written - start;
        if frames <= skip {
            continue;
        }
        let commands_file = speed_commands(piece).map(|text| {
            let name = format!("speed{key}-{index}.txt");
            std::fs::write(span.dir.join(&name), text).map(|_| name)
        });
        let commands_file = commands_file.transpose().map_err(io)?;
        let seek = if commands_file.is_none() { skip } else { 0 };
        let mut shaper = Shaper::new(frames, skip, piece.fades.in_sec * rate, piece.fades.out_sec * rate, gain * piece.volume.unwrap_or(1.0)).starting_at(seek);
        let mut shaped = Vec::new();
        let mut failed = None;
        let args = piece_args(piece, seek as f64 / rate, rate, commands_file.as_deref());
        run_streaming(&ffmpeg(), &args, Some(span.dir), Some(span.cancel), |chunk| {
            shaped.clear();
            shaper.push(chunk, &mut shaped);
            if failed.is_none() && !shaped.is_empty() {
                failed = out.write_all(&shaped).err();
            }
        })?;
        if let Some(error) = failed {
            return Err(error.to_string());
        }
        written = start + skip.max(shaper.frame.min(frames));
        if shaper.frame < frames {
            write_silence(out, frames - shaper.frame.max(skip), rate).map_err(io)?;
            written = start + frames;
        }
    }
    write_silence(out, total - written, rate).map_err(io)?;
    out.flush().map_err(io)
}

pub fn audio_mapping(tracks: usize, mode: AudioMode, limiter: Option<&str>) -> Vec<String> {
    let inputs: Vec<String> = (0..tracks).map(|i| format!("[{i}:a]")).collect();
    if mode == AudioMode::Mix || tracks == 1 {
        let encode = ["-c:a", "aac", "-b:a", "320k"].map(String::from);
        if tracks == 1 && limiter.is_none() {
            return ["-map".to_string(), "0:a".into()].into_iter().chain(encode).collect();
        }
        let mix = (tracks > 1).then(|| format!("amix=inputs={tracks}:normalize=0:duration=longest"));
        let chain: Vec<String> = mix.into_iter().chain(limiter.map(String::from)).collect();
        let graph = format!("{}{}[mix]", inputs.concat(), chain.join(","));
        return ["-filter_complex".to_string(), graph, "-map".into(), "[mix]".into()].into_iter().chain(encode).collect();
    }
    let mut args = Vec::new();
    if let Some(limiter) = limiter {
        let graph: Vec<String> = (0..tracks).map(|i| format!("[{i}:a]{limiter}[a{i}]")).collect();
        args.extend(["-filter_complex".to_string(), graph.join(";")]);
    }
    for i in 0..tracks {
        args.extend(["-map".to_string(), if limiter.is_some() { format!("[a{i}]") } else { format!("{i}:a") }]);
    }
    args.extend(["-c:a", "aac", "-b:a", "256k"].map(String::from));
    args
}

pub fn audio_tags(names: &[String], mode: AudioMode) -> Vec<String> {
    let names = if mode == AudioMode::Mix && names.len() > 1 { vec!["Mix".to_string()] } else { names.to_vec() };
    names
        .iter()
        .enumerate()
        .flat_map(|(i, name)| {
            [
                format!("-metadata:s:a:{i}"),
                format!("title={name}"),
                format!("-metadata:s:a:{i}"),
                format!("handler_name={name}"),
                format!("-disposition:a:{i}"),
                if i == 0 { "default".into() } else { "0".into() },
            ]
        })
        .collect()
}

pub fn render_audio(plan: &RenderPlan, mode: AudioMode, limit_db: Option<f64>, work: &Path, cancel: &Cancel) -> Result<Option<PathBuf>, String> {
    if plan.audio.is_empty() {
        return Ok(None);
    }
    let mut args: Vec<String> = ["-v", "error", "-y"].map(String::from).into();
    for (index, track) in plan.audio.iter().enumerate() {
        let path = work.join(format!("track{index}.pcm"));
        let span = Span { rate: AUDIO_RATE, from_sec: 0.0, to_sec: plan.duration_sec, dir: work, cancel };
        let mut out = BufWriter::new(File::create(&path).map_err(|e| e.to_string())?);
        stream_track(track, index, track.volume, &span, &mut out)?;
        args.extend(["-f", "s16le", "-ar", "48000", "-ac", "2", "-i"].map(String::from));
        args.push(path.to_string_lossy().into_owned());
    }
    args.extend(audio_mapping(plan.audio.len(), mode, limiter_filter(limit_db).as_deref()));
    let output = work.join("audio.mka");
    args.push(output.to_string_lossy().into_owned());
    run(&ffmpeg(), &args, Some(cancel))?;
    Ok(Some(output))
}
