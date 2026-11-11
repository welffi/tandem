use crate::formats::{AUDIO_EXTENSIONS, PROJECT_EXTENSION, extension_of, media_extensions};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::path::PathBuf;

pub fn confirm(text: &str) -> bool {
    MessageDialog::new().set_title("Tandem").set_description(text).set_level(MessageLevel::Info).set_buttons(MessageButtons::OkCancel).show() == MessageDialogResult::Ok
}

fn to_strings(paths: Option<Vec<PathBuf>>) -> Vec<String> {
    paths.unwrap_or_default().into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

pub fn pick_media() -> Vec<String> {
    to_strings(FileDialog::new().add_filter("Video, audio and images", &media_extensions()).add_filter("Tandem project", &[PROJECT_EXTENSION]).pick_files())
}

pub fn pick_audio() -> Vec<String> {
    to_strings(FileDialog::new().add_filter("Audio, or video to take its audio from", &media_extensions()).add_filter("Audio", &AUDIO_EXTENSIONS).pick_files())
}

pub fn pick_project() -> Option<PathBuf> {
    FileDialog::new().set_title("Open project").add_filter("Tandem project", &[PROJECT_EXTENSION]).pick_file()
}

pub fn save_project_as(name: &str) -> Option<PathBuf> {
    FileDialog::new().set_title("Save project").set_file_name(format!("{name}.{PROJECT_EXTENSION}")).add_filter("Tandem project", &[PROJECT_EXTENSION]).save_file()
}

pub fn locate(original_name: &str) -> Option<String> {
    let ext = extension_of(original_name);
    let extensions: Vec<String> = std::iter::once(ext).filter(|e| !e.is_empty()).chain(media_extensions().into_iter().map(String::from)).collect();
    let path = FileDialog::new().set_title(format!("Locate {original_name}")).add_filter(original_name, &extensions).pick_file()?;
    Some(path.to_string_lossy().into_owned())
}

pub fn save_video(default_name: &str) -> Option<PathBuf> {
    FileDialog::new().set_title("Export video").set_file_name(default_name).add_filter("MP4 video", &["mp4"]).save_file()
}

pub fn show_in_folder(path: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", path.display())).spawn();
}
