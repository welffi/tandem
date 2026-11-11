use crate::ffmpeg::data_dir;
use crate::types::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub struct OpenedProject {
    pub file_path: Option<PathBuf>,
    pub project: Project,
    pub missing_asset_ids: Vec<String>,
}

const NOT_A_PROJECT: &str = "This is not a Tandem project file, or it was made by a newer version";

pub fn parse_project(text: &str) -> Result<Project, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| NOT_A_PROJECT.to_string())?;
    let fits = value["version"] == 1 && value["tracks"].is_array() && value["assets"].is_array();
    if !fits {
        return Err(NOT_A_PROJECT.into());
    }
    serde_json::from_value(value).map_err(|e| format!("{NOT_A_PROJECT} ({e})"))
}

fn opened(project: Project, file_path: Option<PathBuf>) -> OpenedProject {
    let missing_asset_ids = project.assets.iter().filter(|a| !Path::new(&a.source_path).exists()).map(|a| a.id.clone()).collect();
    OpenedProject { file_path, project, missing_asset_ids }
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let partial = PathBuf::from(format!("{}.partial", path.display()));
    fs::write(&partial, text).map_err(|e| e.to_string())?;
    fs::rename(&partial, path).map_err(|e| e.to_string())
}

pub fn write_project(project: &Project, path: &Path) -> Result<(), String> {
    write_atomic(path, &serde_json::to_string_pretty(project).map_err(|e| e.to_string())?)
}

pub fn read_project(path: &Path) -> Result<OpenedProject, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(opened(parse_project(&text)?, Some(path.to_path_buf())))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recovery {
    file_path: Option<PathBuf>,
    project: serde_json::Value,
}

fn recovery_path() -> PathBuf {
    data_dir().join("recovery.json")
}

pub fn write_recovery(project: &Project, file_path: Option<&Path>) -> Result<(), String> {
    let recovery = Recovery { file_path: file_path.map(Path::to_path_buf), project: serde_json::to_value(project).map_err(|e| e.to_string())? };
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    write_atomic(&recovery_path(), &serde_json::to_string(&recovery).map_err(|e| e.to_string())?)
}

pub fn clear_recovery() {
    let _ = fs::remove_file(recovery_path());
}

pub fn take_recovery() -> Option<OpenedProject> {
    let text = fs::read_to_string(recovery_path()).ok()?;
    let parsed = serde_json::from_str::<Recovery>(&text).ok().and_then(|r| Some((parse_project(&r.project.to_string()).ok()?, r.file_path)));
    match parsed {
        Some((project, file_path)) => Some(opened(project, file_path.filter(|p| !p.as_os_str().is_empty()))),
        None => {
            clear_recovery();
            None
        }
    }
}
