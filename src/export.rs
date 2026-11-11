use crate::compose::compose_video;
use crate::decode::Pictures;
use crate::export_audio::{audio_tags, render_audio};
use crate::ffmpeg::{CANCELLED, Cancel, command, ffmpeg, run};
use crate::formats::ffmpeg_rate;
use crate::raster::Image;
use crate::render_plan::{RenderPlan, build_render_plan};
use crate::text_render::draw_texts;
use crate::types::{ExportPhase, ExportProgress, ExportSettings, Project, Quality};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

const VIDEO_SHARE: f64 = 0.97;
const FRAMES_AHEAD: usize = 3;

fn x264(quality: Quality) -> &'static [&'static str] {
    match quality {
        Quality::High => &["-c:v", "libx264", "-preset", "medium", "-crf", "18"],
        Quality::Medium => &["-c:v", "libx264", "-preset", "fast", "-crf", "21"],
        Quality::Draft => &["-c:v", "libx264", "-preset", "ultrafast", "-crf", "27"],
    }
}

fn nvenc(quality: Quality) -> &'static [&'static str] {
    match quality {
        Quality::High => &["-c:v", "h264_nvenc", "-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq", "20", "-b:v", "0"],
        Quality::Medium => &["-c:v", "h264_nvenc", "-preset", "p4", "-tune", "hq", "-rc", "vbr", "-cq", "23", "-b:v", "0"],
        Quality::Draft => &["-c:v", "h264_nvenc", "-preset", "p1", "-rc", "vbr", "-cq", "28", "-b:v", "0"],
    }
}

static NVENC: Mutex<Option<bool>> = Mutex::new(None);

fn has_nvenc() -> bool {
    let mut known = NVENC.lock().unwrap();
    *known.get_or_insert_with(|| {
        let probe = ["-v", "error", "-f", "lavfi", "-i", "color=s=256x256:d=0.1", "-c:v", "h264_nvenc", "-f", "null", "-"];
        run(&ffmpeg(), &probe.map(String::from), None).is_ok()
    })
}

fn video_args(plan: &RenderPlan, encoder: &[&str], out: &Path) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"].map(String::from).into();
    args.extend([format!("{}x{}", plan.width, plan.height), "-r".into(), ffmpeg_rate(plan.fps), "-i".into(), "-".into(), "-an".into()]);
    args.extend(encoder.iter().map(|s| s.to_string()));
    args.extend(["-pix_fmt".into(), "yuv420p".into(), out.to_string_lossy().into_owned()]);
    args
}

fn frame_image(project: &Project, plan: &RenderPlan, frame: i64, pics: &mut Pictures) -> Image {
    let (w, h) = (plan.width as usize, plan.height as usize);
    let mut img = compose_video(plan, frame, w, h, pics).unwrap_or_else(|| Image::filled(w, h, [0, 0, 0, 255]));
    draw_texts(&mut img, project, frame as f64 / plan.fps);
    img
}

fn encode_video(project: &Project, plan: &RenderPlan, encoder: &[&str], out: &Path, cancel: &Cancel, progress: &dyn Fn(f64)) -> Result<(), String> {
    let mut child =
        command(&ffmpeg()).args(video_args(plan, encoder, out)).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| format!("Couldn't start ffmpeg: {e}"))?;
    let mut stdin = child.stdin.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let errors = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let (send, frames) = sync_channel::<Image>(FRAMES_AHEAD);
    let writer = thread::spawn(move || {
        for img in frames {
            if stdin.write_all(&img.px).is_err() {
                return false;
            }
        }
        true
    });
    let mut pics = Pictures::new(plan.fps, true);
    for frame in 0..plan.total_frames {
        if cancel.is_cancelled() || send.send(frame_image(project, plan, frame, &mut pics)).is_err() {
            break;
        }
        progress((frame + 1) as f64 / plan.total_frames as f64);
    }
    drop(send);
    let wrote_all = writer.join().unwrap_or(false);
    if cancel.is_cancelled() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CANCELLED.into());
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() && wrote_all {
        return Ok(());
    }
    let text = errors.join().unwrap_or_default();
    Err(format!("The video encoder failed: {}", text.trim().lines().last().unwrap_or("it stopped early")))
}

fn video_with_fallback(project: &Project, plan: &RenderPlan, quality: Quality, out: &Path, cancel: &Cancel, progress: &dyn Fn(f64)) -> Result<(), String> {
    if !has_nvenc() {
        return encode_video(project, plan, x264(quality), out, cancel, progress);
    }
    match encode_video(project, plan, nvenc(quality), out, cancel, progress) {
        Err(error) if error != CANCELLED => {
            *NVENC.lock().unwrap() = Some(false);
            encode_video(project, plan, x264(quality), out, cancel, progress)
        }
        result => result,
    }
}

fn mux_args(plan: &RenderPlan, settings: &ExportSettings, video: &Path, audio: Option<&Path>) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error", "-y", "-i"].map(String::from).into();
    args.push(video.to_string_lossy().into_owned());
    match audio {
        Some(audio) => {
            args.extend(["-i".into(), audio.to_string_lossy().into_owned(), "-map".into(), "0:v".into(), "-map".into(), "1:a".into()]);
            let names: Vec<String> = plan.audio.iter().map(|t| t.name.clone()).collect();
            args.extend(audio_tags(&names, settings.audio_mode));
        }
        None => args.extend(["-map".into(), "0:v".into()]),
    }
    args.extend(["-c".into(), "copy".into(), "-t".into(), format!("{:.6}", plan.duration_sec), "-movflags".into(), "+faststart".into()]);
    args.push(settings.output_path.to_string_lossy().into_owned());
    args
}

fn work_dir() -> Result<PathBuf, String> {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("tandem-export-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn first_failure(results: [Result<(), String>; 2]) -> Result<(), String> {
    let failures: Vec<String> = results.into_iter().filter_map(Result::err).collect();
    match failures.iter().find(|e| *e != CANCELLED).or(failures.first()) {
        Some(error) => Err(error.clone()),
        None => Ok(()),
    }
}

pub fn export_project(project: &Project, settings: &ExportSettings, cancel: &Cancel, on_progress: &(dyn Fn(ExportProgress) + Sync)) -> Result<(), String> {
    let plan = build_render_plan(project);
    if plan.total_frames == 0 {
        return Err("The timeline is empty".into());
    }
    let work = work_dir()?;
    let result = export_in(project, settings, &plan, &work, cancel, on_progress);
    let _ = std::fs::remove_dir_all(&work);
    result
}

fn export_in(project: &Project, settings: &ExportSettings, plan: &RenderPlan, work: &Path, cancel: &Cancel, on_progress: &(dyn Fn(ExportProgress) + Sync)) -> Result<(), String> {
    let inner = Cancel::default();
    let video_path = work.join("video.mp4");
    let progress = |f: f64| on_progress(ExportProgress { phase: ExportPhase::Video, fraction: f * VIDEO_SHARE });
    let (video, audio) = thread::scope(|scope| {
        let audio = scope.spawn(|| {
            let result = render_audio(plan, settings.audio_mode, project.limit_db, work, &inner);
            if result.is_err() {
                inner.cancel();
            }
            result
        });
        let watch = scope.spawn(|| {
            while !inner.is_cancelled() && !cancel.is_cancelled() {
                thread::sleep(Duration::from_millis(50));
            }
            inner.cancel();
        });
        let video = video_with_fallback(project, plan, settings.quality, &video_path, &inner, &progress);
        if video.is_err() {
            inner.cancel();
        }
        let audio = audio.join().unwrap_or_else(|_| Err("The audio render crashed".into()));
        inner.cancel();
        let _ = watch.join();
        (video, audio)
    });
    first_failure([video, audio.as_ref().map(|_| ()).map_err(String::clone)])?;
    if cancel.is_cancelled() {
        return Err(CANCELLED.into());
    }
    let audio_path = audio.unwrap_or_default();
    on_progress(ExportProgress { phase: ExportPhase::Mux, fraction: VIDEO_SHARE });
    run(&ffmpeg(), &mux_args(plan, settings, &video_path, audio_path.as_deref()), Some(cancel))?;
    on_progress(ExportProgress { phase: ExportPhase::Done, fraction: 1.0 });
    Ok(())
}
