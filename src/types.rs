use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Video,
    Audio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Video,
    Audio,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Framing {
    #[default]
    Fit,
    Fill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeFill {
    #[default]
    Black,
    Mirror,
    Blur,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimField {
    Amount,
    Size,
    Height,
    X,
    Y,
    Rotation,
    Softness,
    Spill,
    Distance,
    Blur,
    Angle,
}

pub type Keys = BTreeMap<AnimField, Vec<Keyframe>>;

pub type Bezier = [f64; 4];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Keyframe {
    pub id: String,
    pub source_sec: f64,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease: Option<Bezier>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurvePreset {
    pub id: String,
    pub name: String,
    pub bezier: Bezier,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub content: String,
    pub font: String,
    pub size: f64,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
    pub align: TextAlign,
    pub x: f64,
    pub y: f64,
    pub outline: bool,
    #[serde(rename = "box")]
    pub boxed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Keys>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStream {
    pub title: String,
    pub codec: String,
    pub channels: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseSource {
    pub asset_id: String,
    pub in_sec: f64,
    pub out_sec: f64,
    pub video: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub kind: MediaKind,
    pub duration_sec: f64,
    #[serde(default)]
    pub audio_streams: Vec<AudioStream>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub still: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub alpha: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse_of: Option<ReverseSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedKey {
    pub source_sec: f64,
    pub speed: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    Grayscale,
    Transform,
    ChromaKey,
    DropShadow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub id: String,
    pub kind: EffectKind,
    pub enabled: bool,
    pub amount: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub softness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blur: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Keys>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub asset_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub source_in_sec: f64,
    pub source_out_sec: f64,
    pub timeline_start_sec: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_color: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub dissolve: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextStyle>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub adjustment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freeze_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_keys: Option<Vec<SpeedKey>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub keep_pitch: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<Effect>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub kind: TrackKind,
    pub name: String,
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default = "one")]
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing: Option<Framing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_fill: Option<EdgeFill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_blur: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_db: Option<f64>,
    pub assets: Vec<MediaAsset>,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetTrack {
    pub kind: TrackKind,
    pub name: String,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default = "one")]
    pub volume: f64,
    pub clips: Vec<Clip>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub created_at: f64,
    pub duration_sec: f64,
    pub assets: Vec<MediaAsset>,
    pub tracks: Vec<PresetTrack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFolder {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    #[serde(default)]
    pub folders: Vec<LibraryFolder>,
    #[serde(default)]
    pub items: Vec<Preset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curves: Option<Vec<CurvePreset>>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Probe {
    pub duration_sec: f64,
    pub has_video: bool,
    pub audio_streams: Vec<AudioStream>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub still: bool,
    pub alpha: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    High,
    Medium,
    Draft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMode {
    Mix,
    Separate,
}

#[derive(Clone, Debug)]
pub struct ExportSettings {
    pub output_path: std::path::PathBuf,
    pub quality: Quality,
    pub audio_mode: AudioMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportPhase {
    Audio,
    Video,
    Mux,
    Done,
    Error(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExportProgress {
    pub phase: ExportPhase,
    pub fraction: f64,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn one() -> f64 {
    1.0
}

impl Clip {
    pub fn effects(&self) -> &[Effect] {
        self.effects.as_deref().unwrap_or(&[])
    }

    pub fn speed_keys(&self) -> &[SpeedKey] {
        self.speed_keys.as_deref().unwrap_or(&[])
    }
}

#[derive(Clone, Debug, Default)]
pub struct Peaks {
    pub peaks: Vec<f32>,
    pub peaks_rate: f64,
}
