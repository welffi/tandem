use crate::ffmpeg::{self, args, ffmpeg, ffprobe};
use crate::formats::{IMAGE_EXTENSIONS, STILL_SEC, extension_of};
use crate::types::{AudioStream, Probe};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Deserialize, Default)]
struct Disposition {
    #[serde(default)]
    attached_pic: i32,
}

#[derive(Deserialize, Default)]
struct Tags {
    rotate: Option<String>,
    title: Option<String>,
    handler_name: Option<String>,
}

#[derive(Deserialize)]
struct SideData {
    rotation: Option<f64>,
}

#[derive(Deserialize)]
struct Stream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    pix_fmt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    channels: Option<u32>,
    duration: Option<String>,
    #[serde(default)]
    disposition: Disposition,
    #[serde(default)]
    tags: Tags,
    #[serde(default)]
    side_data_list: Vec<SideData>,
}

#[derive(Deserialize, Default)]
struct Format {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct Output {
    #[serde(default)]
    streams: Vec<Stream>,
    #[serde(default)]
    format: Format,
}

pub fn parse_rate(rate: Option<&str>) -> Option<f64> {
    let rate = rate?;
    let mut parts = rate.split('/').map(|p| p.parse::<f64>().unwrap_or(f64::NAN));
    let num = parts.next()?;
    let value = match parts.next() {
        Some(den) if den != 0.0 => num / den,
        _ => num,
    };
    (value.is_finite() && value > 0.0 && value < 1000.0).then_some(value)
}

fn rotation_of(stream: &Stream) -> f64 {
    let side = stream.side_data_list.iter().find_map(|d| d.rotation);
    let tag = stream.tags.rotate.as_deref().and_then(|r| r.parse().ok()).unwrap_or(0.0);
    side.unwrap_or(tag).abs() % 180.0
}

fn is_generic_handler(handler: &str) -> bool {
    let lower = handler.to_lowercase();
    ["sound handler", "soundhandler", "audio handler", "audiohandler", "core media audio", "isomedia", "gpac", "l-smash", "mainconcept"].iter().any(|g| lower.starts_with(g))
}

fn stream_title(stream: &Stream) -> String {
    if let Some(title) = stream.tags.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        return title.into();
    }
    match stream.tags.handler_name.as_deref().map(str::trim) {
        Some(handler) if !handler.is_empty() && !is_generic_handler(handler) => handler.into(),
        _ => String::new(),
    }
}

fn may_have_alpha(pix_fmt: &str) -> bool {
    ["rgba", "bgra", "argb", "abgr", "ya", "yuva", "gbrap", "pal8"].iter().any(|p| pix_fmt.starts_with(p))
}

pub struct Parsed {
    pub probe: Probe,
    pub check_alpha: bool,
}

pub fn parse_probe(json: &str, path: &str) -> Result<Parsed, String> {
    let data: Output = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let video = data.streams.iter().find(|s| s.codec_type.as_deref() == Some("video") && s.disposition.attached_pic == 0);
    let audio: Vec<&Stream> = data.streams.iter().filter(|s| s.codec_type.as_deref() == Some("audio")).collect();
    if video.is_none() && audio.is_empty() {
        return Err("No audio or video streams found".into());
    }
    let still = video.is_some() && IMAGE_EXTENSIONS.contains(&extension_of(path).as_str());
    let durations: Vec<f64> = if still {
        vec![STILL_SEC]
    } else {
        [data.format.duration.as_deref(), video.and_then(|v| v.duration.as_deref())]
            .into_iter()
            .chain(audio.iter().map(|a| a.duration.as_deref()))
            .filter_map(|d| d?.parse::<f64>().ok())
            .filter(|d| *d > 0.0)
            .collect()
    };
    if durations.is_empty() {
        return Err("Could not determine media duration".into());
    }
    let rotated = video.is_some_and(|v| rotation_of(v) == 90.0);
    let (width, height) = video.map_or((None, None), |v| if rotated { (v.height, v.width) } else { (v.width, v.height) });
    let probe = Probe {
        duration_sec: durations.iter().cloned().fold(f64::MIN, f64::max),
        has_video: video.is_some(),
        audio_streams: audio.iter().map(|a| AudioStream { title: stream_title(a), codec: a.codec_name.clone().unwrap_or_else(|| "unknown".into()), channels: a.channels.unwrap_or(2) }).collect(),
        width,
        height,
        fps: if still { None } else { video.and_then(|v| parse_rate(v.avg_frame_rate.as_deref()).or_else(|| parse_rate(v.r_frame_rate.as_deref()))) },
        video_codec: video.and_then(|v| v.codec_name.clone()),
        still,
        alpha: false,
    };
    let check_alpha = still && video.and_then(|v| v.pix_fmt.as_deref()).is_some_and(may_have_alpha);
    Ok(Parsed { probe, check_alpha })
}

fn has_transparency(path: &str) -> bool {
    let filter = "format=rgba,alphaextract,signalstats,metadata=mode=print:key=lavfi.signalstats.YMIN:file=-";
    let Ok(out) = ffmpeg::run(&ffmpeg(), &args(&["-v", "error", "-f", "image2", "-pattern_type", "none", "-i", path, "-vf", filter, "-f", "null", "-"]), None) else {
        return false;
    };
    let text = String::from_utf8_lossy(&out);
    text.split("YMIN=").nth(1).and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u32>().ok()).is_some_and(|min| min < 255)
}

static CACHE: LazyLock<Mutex<HashMap<String, Probe>>> = LazyLock::new(Default::default);

pub fn probe(path: &str) -> Result<Probe, String> {
    if let Some(found) = CACHE.lock().unwrap().get(path) {
        return Ok(found.clone());
    }
    let raw = ffmpeg::run(&ffprobe(), &args(&["-v", "error", "-print_format", "json", "-show_format", "-show_streams", path]), None)?;
    let Parsed { mut probe, check_alpha } = parse_probe(&String::from_utf8_lossy(&raw), path)?;
    probe.alpha = check_alpha && has_transparency(path);
    CACHE.lock().unwrap().insert(path.into(), probe.clone());
    Ok(probe)
}
