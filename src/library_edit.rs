use crate::format::format_duration;
use crate::types::{Library, LibraryFolder, Preset};

const NEW_FOLDER: &str = "New folder";

pub fn add_item(library: &Library, item: Preset, folder_id: Option<String>) -> Library {
    let mut next = library.clone();
    next.items.push(Preset { folder_id, ..item });
    next
}

pub fn add_folder(library: &Library, id: String, parent_id: Option<String>) -> Library {
    let mut next = library.clone();
    next.folders.push(LibraryFolder { id, name: NEW_FOLDER.into(), parent_id, color: None });
    next
}

pub fn rename(library: &Library, id: &str, name: &str) -> Library {
    let name = name.trim();
    if name.is_empty() {
        return library.clone();
    }
    let mut next = library.clone();
    next.folders.iter_mut().filter(|f| f.id == id).for_each(|f| f.name = name.into());
    next.items.iter_mut().filter(|p| p.id == id).for_each(|p| p.name = name.into());
    next
}

pub fn set_color(library: &Library, id: &str, color: Option<String>) -> Library {
    let mut next = library.clone();
    next.folders.iter_mut().filter(|f| f.id == id).for_each(|f| f.color = color.clone());
    next.items.iter_mut().filter(|p| p.id == id).for_each(|p| p.color = color.clone());
    next
}

pub fn delete_item(library: &Library, id: &str) -> Library {
    Library { items: library.items.iter().filter(|p| p.id != id).cloned().collect(), ..library.clone() }
}

pub fn delete_folder(library: &Library, id: &str) -> Library {
    let parent = library.folders.iter().find(|f| f.id == id).and_then(|f| f.parent_id.clone());
    let mut next = library.clone();
    next.folders.retain(|f| f.id != id);
    next.folders.iter_mut().filter(|f| f.parent_id.as_deref() == Some(id)).for_each(|f| f.parent_id = parent.clone());
    next.items.iter_mut().filter(|p| p.folder_id.as_deref() == Some(id)).for_each(|p| p.folder_id = parent.clone());
    next
}

pub fn move_to(library: &Library, id: &str, folder_id: Option<&str>) -> Library {
    if folder_path(library, folder_id).iter().any(|f| f.id == id) {
        return library.clone();
    }
    let target = folder_id.map(String::from);
    let mut next = library.clone();
    next.folders.iter_mut().filter(|f| f.id == id).for_each(|f| f.parent_id = target.clone());
    next.items.iter_mut().filter(|p| p.id == id).for_each(|p| p.folder_id = target.clone());
    next
}

pub fn replace_item(library: &Library, id: &str, fresh: Preset) -> Library {
    let mut next = library.clone();
    for item in next.items.iter_mut().filter(|p| p.id == id) {
        *item = Preset { id: item.id.clone(), name: item.name.clone(), created_at: item.created_at, folder_id: item.folder_id.clone(), color: item.color.clone(), ..fresh.clone() };
    }
    next
}

pub fn folder_path(library: &Library, current: Option<&str>) -> Vec<LibraryFolder> {
    let mut path: Vec<LibraryFolder> = Vec::new();
    let mut at = current;
    while let Some(id) = at {
        let Some(folder) = library.folders.iter().find(|f| f.id == id) else { break };
        if path.len() > library.folders.len() {
            break;
        }
        path.insert(0, folder.clone());
        at = folder.parent_id.as_deref();
    }
    path
}

pub fn child_count(library: &Library, folder_id: &str) -> usize {
    library.folders.iter().filter(|f| f.parent_id.as_deref() == Some(folder_id)).count() + library.items.iter().filter(|p| p.folder_id.as_deref() == Some(folder_id)).count()
}

pub fn entry_name(library: &Library, id: &str) -> Option<String> {
    let folder = library.folders.iter().find(|f| f.id == id).map(|f| f.name.clone());
    folder.or_else(|| library.items.iter().find(|p| p.id == id).map(|p| p.name.clone()))
}

fn counted(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

pub fn item_meta(item: &Preset) -> String {
    let clips = item.tracks.iter().map(|t| t.clips.len()).sum();
    format!("{} · {} · {}", counted(item.tracks.len(), "layer"), counted(clips, "clip"), format_duration(item.duration_sec))
}

pub fn folder_meta(count: usize) -> String {
    counted(count, "item")
}
