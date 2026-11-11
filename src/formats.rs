use std::path::Path;

pub const VIDEO_EXTENSIONS: [&str; 14] = ["mp4", "mkv", "mov", "m4v", "webm", "avi", "flv", "ts", "mts", "m2ts", "wmv", "mpg", "mpeg", "3gp"];
pub const AUDIO_EXTENSIONS: [&str; 10] = ["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus", "wma", "aiff", "aif"];
pub const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "bmp"];
pub const STILL_SEC: f64 = 5.0;
pub const PROJECT_EXTENSION: &str = "tandem";
pub const DEFAULT_LIMIT_DB: f64 = -1.0;
pub const MIN_LIMIT_DB: f64 = -24.0;

pub fn media_extensions() -> Vec<&'static str> {
    VIDEO_EXTENSIONS.iter().chain(&AUDIO_EXTENSIONS).chain(&IMAGE_EXTENSIONS).copied().collect()
}

pub fn extension_of(path: &str) -> String {
    Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn is_openable(path: &str) -> bool {
    let ext = extension_of(path);
    ext == PROJECT_EXTENSION || media_extensions().contains(&ext.as_str())
}

pub fn is_project(path: &str) -> bool {
    extension_of(path) == PROJECT_EXTENSION
}

pub fn ffmpeg_rate(fps: f64) -> String {
    for base in [24.0, 30.0, 60.0] {
        if (fps - base * 1000.0 / 1001.0).abs() < 0.01 {
            return format!("{}/1001", base * 1000.0);
        }
    }
    format!("{fps}")
}

pub fn db_to_gain(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

pub fn limiter_filter(limit_db: Option<f64>) -> Option<String> {
    let limit = db_to_gain(limit_db?.clamp(MIN_LIMIT_DB, 0.0));
    Some(format!("alimiter=limit={limit:.5}:attack=5:release=80:level=0"))
}
