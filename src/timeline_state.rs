use crate::store::{Store, stream_key};
use crate::timeline_drag::{Drag, DragView};
use crate::types::{Peaks, Track};
use eframe::egui::{ColorImage, Context, TextureHandle, TextureId, TextureOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const LOADS_PER_FRAME: u32 = 8;

#[derive(Default)]
pub struct TimelineState {
    pub drag: Option<Drag>,
    pub view: Option<DragView>,
    pub hover_clip: Option<String>,
    pub menu_clip: Option<String>,
    pub asset_drag: Option<String>,
    pub library_drag: Option<String>,
    pub v_scroll: f32,
    pub followed: f64,
    pub loads_left: u32,
    textures: HashMap<PathBuf, Option<TextureHandle>>,
    norms: HashMap<String, (usize, f32)>,
    track_peaks: HashMap<String, ((u64, usize), Option<f32>)>,
}

fn load_texture(ctx: &Context, path: &Path) -> Option<TextureHandle> {
    let image = image::open(path).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(ctx.load_texture(path.to_string_lossy(), ColorImage::from_rgba_unmultiplied(size, image.as_raw()), TextureOptions::LINEAR))
}

pub fn loudest(peaks: &[f32]) -> f32 {
    let top = peaks.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    if top > 0.0 { top } else { 1.0 }
}

pub fn track_loudest<'a>(track: &Track, peaks_of: impl Fn(&str, usize) -> Option<&'a Peaks>) -> Option<f32> {
    let mut max = 0.0_f32;
    let mut known = false;
    for clip in &track.clips {
        let Some(peaks) = peaks_of(&clip.asset_id, clip.stream.unwrap_or(0)) else { continue };
        known = true;
        let per_sec = peaks.peaks_rate / 2.0;
        let end = ((peaks.peaks.len() / 2) as f64).min((clip.source_out_sec * per_sec).ceil()) as usize;
        let start = (clip.source_in_sec * per_sec).floor().max(0.0) as usize;
        for bucket in start..end {
            let loud = (-peaks.peaks[2 * bucket]).max(peaks.peaks[2 * bucket + 1]);
            max = max.max(clip.volume.unwrap_or(1.0) as f32 * loud);
        }
    }
    known.then_some(max * track.volume as f32)
}

impl TimelineState {
    pub fn begin_frame(&mut self) {
        self.loads_left = LOADS_PER_FRAME;
    }

    pub fn texture(&mut self, ctx: &Context, path: &Path) -> Option<TextureId> {
        if let Some(entry) = self.textures.get(path) {
            return entry.as_ref().map(|t| t.id());
        }
        if self.loads_left == 0 {
            ctx.request_repaint();
            return None;
        }
        self.loads_left -= 1;
        let handle = load_texture(ctx, path);
        let id = handle.as_ref().map(|t| t.id());
        self.textures.insert(path.to_path_buf(), handle);
        id
    }

    pub fn peak_norm(&mut self, key: &str, peaks: &Peaks) -> f32 {
        if let Some((len, norm)) = self.norms.get(key)
            && *len == peaks.peaks.len()
        {
            return *norm;
        }
        let norm = loudest(&peaks.peaks);
        self.norms.insert(key.to_string(), (peaks.peaks.len(), norm));
        norm
    }

    pub fn track_peak(&mut self, store: &Store, track: &Track) -> Option<f32> {
        let stamp = (store.rev, store.peaks.len());
        if let Some((seen, value)) = self.track_peaks.get(&track.id)
            && *seen == stamp
        {
            return *value;
        }
        let value = track_loudest(track, |asset, stream| store.peaks.get(&stream_key(asset, stream)));
        self.track_peaks.insert(track.id.clone(), (stamp, value));
        value
    }
}
