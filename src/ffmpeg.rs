use std::io::{self, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("TANDEM_DATA") {
        return PathBuf::from(dir);
    }
    PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("Tandem")
}

pub fn tools_dir() -> PathBuf {
    data_dir().join("ffmpeg")
}

pub fn ffmpeg() -> PathBuf {
    tools_dir().join("ffmpeg.exe")
}

pub fn ffprobe() -> PathBuf {
    tools_dir().join("ffprobe.exe")
}

pub fn installed() -> bool {
    ffmpeg().is_file() && ffprobe().is_file()
}

pub fn command(bin: &Path) -> Command {
    let mut command = Command::new(bin);
    command.creation_flags(CREATE_NO_WINDOW).stdin(Stdio::null());
    command
}

pub fn spawn_piped(bin: &Path, args: &[String]) -> io::Result<Child> {
    command(bin).args(args).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
}

#[derive(Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub const CANCELLED: &str = "Cancelled";

pub fn run_streaming(bin: &Path, args: &[String], dir: Option<&Path>, cancel: Option<&Cancel>, mut on_stdout: impl FnMut(&[u8])) -> Result<(), String> {
    let mut command = command(bin);
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let mut child = command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("Couldn't start {}: {e}", name_of(bin)))?;
    let mut stderr = child.stderr.take().unwrap();
    let errors = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let mut stdout = child.stdout.take().unwrap();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        if cancel.is_some_and(Cancel::is_cancelled) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CANCELLED.into());
        }
        match stdout.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => on_stdout(&buffer[..read]),
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if cancel.is_some_and(Cancel::is_cancelled) {
        return Err(CANCELLED.into());
    }
    if status.success() {
        return Ok(());
    }
    let text = errors.join().unwrap_or_default();
    let tail: Vec<&str> = text.trim().lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
    Err(format!("{} failed ({status}): {}", name_of(bin), tail.join(" ")))
}

pub fn run(bin: &Path, args: &[String], cancel: Option<&Cancel>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    run_streaming(bin, args, None, cancel, |chunk| out.extend_from_slice(chunk))?;
    Ok(out)
}

pub fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn name_of(bin: &Path) -> String {
    bin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
}
