use crate::ffmpeg::data_dir;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentProcessId};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};

const INBOX_POLL: Duration = Duration::from_millis(250);

static INSTANCE: AtomicUsize = AtomicUsize::new(0);

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn mutex_name() -> String {
    let scope: String = data_dir().to_string_lossy().chars().filter(char::is_ascii_alphanumeric).collect();
    format!("tandem_single_instance_{scope}")
}

pub fn claim_single_instance() -> bool {
    let name = wide(&mutex_name());
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        INSTANCE.store(handle as usize, Ordering::SeqCst);
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

pub fn release_single_instance() {
    let handle = INSTANCE.swap(0, Ordering::SeqCst);
    if handle != 0 {
        unsafe { CloseHandle(handle as HANDLE) };
    }
}

fn inbox_path() -> PathBuf {
    data_dir().join("inbox.txt")
}

pub fn send_to_running_copy(paths: &[String]) {
    if !paths.is_empty() {
        let _ = fs::create_dir_all(data_dir());
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(inbox_path()) {
            let _ = file.write_all(paths.iter().map(|p| format!("{p}\n")).collect::<String>().as_bytes());
        }
    }
    focus_running_copy();
}

pub fn watch_inbox(repaint: impl Fn() + Send + 'static) -> Receiver<Vec<String>> {
    let (send, recv) = channel();
    let _ = fs::remove_file(inbox_path());
    thread::spawn(move || {
        loop {
            thread::sleep(INBOX_POLL);
            let Ok(text) = fs::read_to_string(inbox_path()) else { continue };
            let _ = fs::remove_file(inbox_path());
            let paths: Vec<String> = text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
            if send.send(paths).is_err() {
                break;
            }
            repaint();
        }
    });
    recv
}

fn window_title(window: HWND) -> String {
    let mut buffer = [0u16; 512];
    let length = unsafe { GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..length])
}

struct Search {
    pid: u32,
    found: usize,
}

unsafe extern "system" fn visit(window: HWND, param: LPARAM) -> i32 {
    let search = unsafe { &mut *(param as *mut Search) };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    let title = window_title(window);
    if pid != search.pid && (title == "Tandem" || title.ends_with(" - Tandem")) {
        search.found = window as usize;
        return 0;
    }
    1
}

fn focus_running_copy() {
    let mut search = Search { pid: unsafe { GetCurrentProcessId() }, found: 0 };
    unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
    let window = search.found as HWND;
    if window.is_null() {
        return;
    }
    unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window);
    }
}
