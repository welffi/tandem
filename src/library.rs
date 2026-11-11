use crate::cache::file_key;
use crate::ffmpeg::data_dir;
use crate::types::{Library, MediaAsset, Preset};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn root() -> PathBuf {
    data_dir().join("library")
}

fn media_dir() -> PathBuf {
    root().join("media")
}

fn library_path() -> PathBuf {
    root().join("library.json")
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(_) => {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis());
            let _ = fs::rename(path, format!("{}.broken-{stamp}", path.display()));
            None
        }
    }
}

fn read_library() -> Library {
    if let Some(saved) = read_json::<Library>(&library_path()) {
        return saved;
    }
    let items = read_json::<Vec<Preset>>(&data_dir().join("presets.json")).unwrap_or_default();
    Library { folders: Vec::new(), items, curves: None }
}

fn keep_copy(asset: &MediaAsset) -> MediaAsset {
    let source = Path::new(&asset.source_path);
    if source.starts_with(media_dir()) {
        return asset.clone();
    }
    let Ok(key) = file_key(&asset.source_path) else {
        return asset.clone();
    };
    let ext = source.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default();
    let dest = media_dir().join(key + &ext);
    if !dest.is_file() {
        let partial = PathBuf::from(format!("{}.partial", dest.display()));
        let copied = fs::create_dir_all(media_dir()).and_then(|()| fs::copy(source, &partial)).and_then(|_| fs::rename(&partial, &dest));
        if copied.is_err() {
            return asset.clone();
        }
    }
    MediaAsset { source_path: dest.to_string_lossy().into_owned(), original_path: Some(asset.original_path.clone().unwrap_or_else(|| asset.source_path.clone())), ..asset.clone() }
}

pub fn save_library(library: &Library) -> Result<Library, String> {
    let kept = Library { items: library.items.iter().map(|item| Preset { assets: item.assets.iter().map(keep_copy).collect(), ..item.clone() }).collect(), ..library.clone() };
    fs::create_dir_all(root()).map_err(|e| e.to_string())?;
    let partial = library_path().with_extension("json.partial");
    fs::write(&partial, serde_json::to_string_pretty(&kept).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    fs::rename(&partial, library_path()).map_err(|e| e.to_string())?;
    Ok(kept)
}

pub fn load_library() -> Result<Library, String> {
    save_library(&read_library())
}
