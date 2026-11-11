use crate::ffmpeg::data_dir;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn file_key(path: &str) -> Result<String, String> {
    let info = fs::metadata(path).map_err(|e| format!("{path}: {e}"))?;
    let modified = info.modified().ok().and_then(|m| m.duration_since(UNIX_EPOCH).ok()).map_or(0.0, |d| d.as_secs_f64() * 1000.0);
    let digest = Sha1::digest(format!("{path}|{}|{modified}", info.len()).as_bytes());
    Ok(digest.iter().take(10).map(|b| format!("{b:02x}")).collect())
}

pub fn cache_dir_for(path: &str) -> Result<PathBuf, String> {
    let dir = data_dir().join("media-cache").join(file_key(path)?);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn replace(partial: &Path, target: &Path) -> Result<(), String> {
    fs::rename(partial, target).map_err(|e| e.to_string())
}
