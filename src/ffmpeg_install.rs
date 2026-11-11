use crate::ffmpeg::{self, tools_dir};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

#[derive(Clone, Debug, PartialEq)]
pub enum InstallState {
    Downloading(Option<f32>),
    Unpacking,
    Done,
    Failed(String),
}

pub type Shared = Arc<Mutex<InstallState>>;

pub fn start(repaint: impl Fn() + Send + 'static) -> Shared {
    let state = Arc::new(Mutex::new(InstallState::Downloading(None)));
    let shared = state.clone();
    thread::spawn(move || {
        let result = install(&shared, &repaint);
        *shared.lock().unwrap() = match result {
            Ok(()) => InstallState::Done,
            Err(message) => InstallState::Failed(message),
        };
        repaint();
    });
    state
}

fn install(state: &Shared, repaint: &dyn Fn()) -> Result<(), String> {
    let work = tools_dir().with_file_name("ffmpeg-download");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let zip = work.join("ffmpeg.zip");
    download(&zip, state, repaint).map_err(|e| format!("Couldn't download ffmpeg: {e}"))?;
    *state.lock().unwrap() = InstallState::Unpacking;
    repaint();
    let status = ffmpeg::command(Path::new("tar")).arg("-xf").arg(&zip).arg("-C").arg(&work).status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Couldn't unpack ffmpeg.".into());
    }
    fs::create_dir_all(tools_dir()).map_err(|e| e.to_string())?;
    for name in ["ffmpeg.exe", "ffprobe.exe"] {
        let found = find_file(&work, name).ok_or_else(|| format!("{name} wasn't in the download."))?;
        fs::rename(&found, tools_dir().join(name)).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_dir_all(&work);
    Ok(())
}

fn download(target: &Path, state: &Shared, repaint: &dyn Fn()) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(900))).build().into();
    let mut response = agent.get(URL).header("User-Agent", "tandem").call().map_err(|e| e.to_string())?;
    let total = response.body().content_length().filter(|&t| t > 0);
    let mut reader = response.body_mut().as_reader();
    let mut file = File::create(target).map_err(|e| e.to_string())?;
    let mut buffer = vec![0; 1 << 16];
    let mut done = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            return Ok(());
        }
        file.write_all(&buffer[..read]).map_err(|e| e.to_string())?;
        done += read as u64;
        if let Some(total) = total {
            *state.lock().unwrap() = InstallState::Downloading(Some(done as f32 / total as f32));
            repaint();
        }
    }
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)) {
            return Some(path);
        }
    }
    None
}
