use crate::cache::{cache_dir_for, replace};
use crate::ffmpeg::{self, args, ffmpeg, to_f32};
use crate::formats::{IMAGE_EXTENSIONS, extension_of};
use crate::probe::probe;
use crate::types::Peaks;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Thumbnails {
    pub interval_sec: f64,
    pub files: Vec<PathBuf>,
}

pub fn thumbnail_interval(duration_sec: f64) -> f64 {
    if duration_sec < 4.0 {
        return (duration_sec / 4.0).max(0.05);
    }
    (duration_sec / 300.0).max(1.0)
}

fn jpgs_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "jpg")).collect();
    files.sort();
    files
}

pub fn thumbnails(path: &str, duration_sec: f64) -> Result<Thumbnails, String> {
    let interval_sec = thumbnail_interval(duration_sec);
    let dir = cache_dir_for(path)?.join("thumbs");
    let done = dir.join("done.txt");
    if fs::read_to_string(&done).ok().and_then(|t| t.trim().parse::<f64>().ok()) == Some(interval_sec) {
        return Ok(Thumbnails { interval_sec, files: jpgs_in(&dir) });
    }
    let still = IMAGE_EXTENSIONS.contains(&extension_of(path).as_str());
    let filter = if still { "scale=-2:72".to_string() } else { format!("fps=1/{interval_sec},scale=-2:72") };
    let pattern = dir.join("t%05d.jpg").to_string_lossy().into_owned();
    let extract = |keyframes_only: bool| -> Result<Vec<PathBuf>, String> {
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut list = args(&["-v", "error"]);
        if keyframes_only {
            list.extend(args(&["-skip_frame", "nokey"]));
        }
        list.extend(args(&["-i", path, "-an", "-vf", &filter, "-q:v", "5", &pattern]));
        ffmpeg::run(&ffmpeg(), &list, None)?;
        Ok(jpgs_in(&dir))
    };
    let expected = (duration_sec / interval_sec).floor().max(1.0) as usize;
    let mut files = extract(true)?;
    if files.len() * 2 < expected {
        files = extract(false)?;
    }
    fs::write(&done, interval_sec.to_string()).map_err(|e| e.to_string())?;
    Ok(Thumbnails { interval_sec, files })
}

const PEAK_RATE: u32 = 8000;
const PEAK_BUCKET: usize = 32;

pub fn bucket_peaks(samples: &[f32]) -> Vec<f32> {
    samples.chunks(PEAK_BUCKET).flat_map(|bucket| [bucket.iter().cloned().fold(0.0, f32::min), bucket.iter().cloned().fold(0.0, f32::max)]).collect()
}

pub fn stream_peaks(path: &str, stream: usize) -> Result<Peaks, String> {
    let peaks_rate = PEAK_RATE as f64 / PEAK_BUCKET as f64 * 2.0;
    let cache = cache_dir_for(path)?.join(format!("peaks{stream}.v1.f32"));
    if let Ok(bytes) = fs::read(&cache) {
        return Ok(Peaks { peaks: to_f32(&bytes), peaks_rate });
    }
    let map = format!("0:a:{stream}");
    let raw = ffmpeg::run(&ffmpeg(), &args(&["-v", "error", "-i", path, "-map", &map, "-ac", "1", "-ar", &PEAK_RATE.to_string(), "-f", "f32le", "pipe:1"]), None)?;
    let peaks = bucket_peaks(&to_f32(&raw));
    let bytes: Vec<u8> = peaks.iter().flat_map(|v| v.to_le_bytes()).collect();
    let partial = cache.with_extension("partial");
    fs::write(&partial, bytes).map_err(|e| e.to_string())?;
    replace(&partial, &cache)?;
    Ok(Peaks { peaks, peaks_rate })
}

const REVERSE_BUFFER_BYTES: f64 = 512.0 * 1024.0 * 1024.0;

pub fn reverse_chunk_sec(width: u32, height: u32, fps: f64) -> f64 {
    let frame_bytes = width as f64 * height as f64 * 1.5;
    (REVERSE_BUFFER_BYTES / frame_bytes).floor().max(1.0) / fps
}

pub fn reversed_media(path: &str, in_sec: f64, out_sec: f64, with_video: bool) -> Result<PathBuf, String> {
    let info = probe(path)?;
    let video = with_video && info.has_video && !info.still;
    let name = format!("reverse-{in_sec:.3}-{out_sec:.3}{}", if video { "" } else { "-audio" });
    let dir = cache_dir_for(path)?;
    let ext = if video { "mp4" } else { "m4a" };
    let target = dir.join(format!("{name}.{ext}"));
    let done = dir.join(format!("{name}.done"));
    if done.is_file() {
        return Ok(target);
    }
    let work = dir.join(format!("{name}.parts"));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let mut inputs: Vec<String> = Vec::new();
    if video {
        let fps = info.fps.unwrap_or(30.0);
        let chunk_sec = reverse_chunk_sec(info.width.unwrap_or(1920), info.height.unwrap_or(1080), fps);
        let count = ((out_sec - in_sec) / chunk_sec - 1e-6).ceil() as usize;
        let mut parts = Vec::new();
        for i in (0..count).rev() {
            let start = in_sec + i as f64 * chunk_sec;
            let part = work.join(format!("part{}.mp4", parts.len()));
            let length = chunk_sec.min(out_sec - start);
            ffmpeg::run(
                &ffmpeg(),
                &args(&[
                    "-v",
                    "error",
                    "-y",
                    "-ss",
                    &format!("{start:.6}"),
                    "-t",
                    &format!("{length:.6}"),
                    "-i",
                    path,
                    "-map",
                    "0:v:0",
                    "-an",
                    "-sn",
                    "-dn",
                    "-vf",
                    "reverse",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "18",
                    "-pix_fmt",
                    "yuv420p",
                    "-r",
                    &fps.to_string(),
                    &part.to_string_lossy(),
                ]),
                None,
            )?;
            parts.push(part);
        }
        let list = work.join("parts.txt");
        let lines: Vec<String> = parts.iter().map(|p| format!("file '{}'", p.to_string_lossy().replace('\\', "/").replace('\'', "'\\''"))).collect();
        fs::write(&list, lines.join("\n")).map_err(|e| e.to_string())?;
        inputs.extend(args(&["-f", "concat", "-safe", "0", "-i", &list.to_string_lossy()]));
    }
    let audio = !info.audio_streams.is_empty();
    if audio {
        inputs.extend(args(&["-ss", &format!("{in_sec:.6}"), "-t", &format!("{:.6}", out_sec - in_sec), "-i", path]));
    }
    let partial = work.join(format!("out.{ext}"));
    let mut list = args(&["-v", "error", "-y"]);
    list.extend(inputs);
    if video {
        list.extend(args(&["-map", "0:v:0", "-c:v", "copy"]));
    }
    if audio {
        let map = format!("{}:a", if video { 1 } else { 0 });
        list.extend(args(&["-map", &map, "-af", "areverse", "-c:a", "aac", "-b:a", "256k"]));
    }
    list.extend(args(&["-sn", "-dn", "-movflags", "+faststart", &partial.to_string_lossy()]));
    ffmpeg::run(&ffmpeg(), &list, None)?;
    replace(&partial, &target)?;
    let _ = fs::remove_dir_all(&work);
    fs::write(&done, "").map_err(|e| e.to_string())?;
    Ok(target)
}
