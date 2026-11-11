use ab_glyph::FontArc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use windows_sys::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumValueW, RegOpenKeyExW};

const FONTS_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
const FALLBACK: &str = "Segoe UI";

pub fn face_name(value_name: &str) -> String {
    let name = value_name.split(" (").next().unwrap_or(value_name);
    name.trim().to_lowercase()
}

pub fn face_key(family: &str, bold: bool, italic: bool) -> String {
    let mut key = family.trim().to_lowercase();
    if bold {
        key.push_str(" bold");
    }
    if italic {
        key.push_str(" italic");
    }
    key
}

pub fn pick_face<'a>(faces: &'a HashMap<String, PathBuf>, family: &str, bold: bool, italic: bool) -> Option<&'a PathBuf> {
    [(family, bold, italic), (family, bold, false), (family, false, italic), (family, false, false), (FALLBACK, bold, italic), (FALLBACK, false, false)]
        .iter()
        .find_map(|(f, b, i)| faces.get(&face_key(f, *b, *i)))
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn read_faces(root: HKEY, base: &PathBuf, faces: &mut HashMap<String, PathBuf>) {
    let mut key: HKEY = std::ptr::null_mut();
    if unsafe { RegOpenKeyExW(root, wide(FONTS_KEY).as_ptr(), 0, KEY_READ, &mut key) } != 0 {
        return;
    }
    for index in 0.. {
        let mut name = [0u16; 512];
        let mut name_len = name.len() as u32;
        let mut data = [0u16; 1024];
        let mut data_len = (data.len() * 2) as u32;
        let status = unsafe { RegEnumValueW(key, index, name.as_mut_ptr(), &mut name_len, std::ptr::null(), std::ptr::null_mut(), data.as_mut_ptr().cast(), &mut data_len) };
        if status != 0 {
            break;
        }
        let file = String::from_utf16_lossy(&data[..(data_len as usize / 2)]).trim_end_matches('\0').to_string();
        if !file.to_lowercase().ends_with(".ttf") && !file.to_lowercase().ends_with(".otf") {
            continue;
        }
        let path = if file.contains('\\') { PathBuf::from(&file) } else { base.join(&file) };
        faces.entry(face_name(&String::from_utf16_lossy(&name[..name_len as usize]))).or_insert(path);
    }
    unsafe { RegCloseKey(key) };
}

static FACES: LazyLock<HashMap<String, PathBuf>> = LazyLock::new(|| {
    let windows = PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| r"C:\Windows".into())).join("Fonts");
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join(r"Microsoft\Windows\Fonts");
    let mut faces = HashMap::new();
    read_faces(HKEY_LOCAL_MACHINE, &windows, &mut faces);
    read_faces(HKEY_CURRENT_USER, &local, &mut faces);
    faces
});

static LOADED: LazyLock<Mutex<HashMap<PathBuf, Option<FontArc>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn font(family: &str, bold: bool, italic: bool) -> Option<FontArc> {
    let path = pick_face(&FACES, family, bold, italic)?.clone();
    let mut loaded = LOADED.lock().ok()?;
    loaded.entry(path.clone()).or_insert_with(|| std::fs::read(&path).ok().and_then(|bytes| FontArc::try_from_vec(bytes).ok())).clone()
}
