use crate::edits::{add_track, clips_span, find_clip, remove_clips, ripple_delete, split_clips_at, update_clips};
use crate::freeze::{freeze_frame_at, resume_from_freeze};
use crate::insert::{default_text, insert_adjustment, insert_text};
use crate::keyframes::{map_clip_keys, remove_keys};
use crate::silence::trim_silence;
use crate::store::{Store, stream_key};
use crate::timeline_math::{fit_zoom, zoom_about};
use crate::types::TrackKind;
use std::collections::BTreeMap;

const TEXT_CLIP_SEC: f64 = 3.0;
const ADJUST_CLIP_SEC: f64 = 5.0;
pub const ZOOM_STEP: f64 = 1.5;

pub fn keys_by_clip(refs: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut by_clip: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for reference in refs {
        if let Some((clip, key)) = reference.split_once('/') {
            by_clip.entry(clip.to_string()).or_default().push(key.to_string());
        }
    }
    by_clip
}

impl Store {
    pub fn split_at_playhead(&mut self) {
        let selected = (!self.selection.is_empty()).then(|| self.selection.clone());
        let next = split_clips_at(self.project(), selected.as_deref(), self.playhead_sec);
        self.edit(next, None);
    }

    pub fn add_text_at_playhead(&mut self) {
        let (next, id) = insert_text(self.project(), self.playhead_sec, TEXT_CLIP_SEC, default_text());
        self.edit(next, None);
        self.set_selection(vec![id]);
    }

    pub fn add_adjustment_at_playhead(&mut self) {
        let (next, id) = insert_adjustment(self.project(), self.playhead_sec, ADJUST_CLIP_SEC);
        self.edit(next, None);
        self.set_selection(vec![id]);
    }

    pub fn add_track_of(&mut self, kind: TrackKind) {
        let next = add_track(self.project(), kind, None);
        self.edit(next, None);
    }

    pub fn add_freeze_frame(&mut self, clip_id: &str) {
        let next = freeze_frame_at(self.project(), clip_id, self.playhead_sec);
        self.edit(next, None);
    }

    pub fn end_freeze_frame(&mut self, clip_id: &str) {
        let next = resume_from_freeze(self.project(), clip_id, self.playhead_sec);
        self.edit(next, None);
    }

    pub fn trim_silence_selection(&mut self) {
        let ids: Vec<String> = self.project().tracks.iter().filter(|t| t.kind == TrackKind::Audio).flat_map(|t| &t.clips).filter(|c| self.selection.contains(&c.id)).map(|c| c.id.clone()).collect();
        if ids.is_empty() {
            return;
        }
        let peaks_for = |clip: &crate::types::Clip| self.peaks.get(&stream_key(&clip.asset_id, clip.stream.unwrap_or(0))).cloned();
        let (next, trimmed) = trim_silence(self.project(), &ids, peaks_for);
        if trimmed > 0 {
            self.edit(next, None);
            self.notify(format!("Trimmed the silence off {trimmed} clip{}.", if trimmed == 1 { "" } else { "s" }));
        } else if ids.iter().any(|id| find_clip(self.project(), id).is_some_and(|(_, c)| self.peaks.contains_key(&stream_key(&c.asset_id, c.stream.unwrap_or(0))))) {
            self.notify("No silence at the start or end to trim.");
        } else {
            self.notify("The waveform is still loading. Try again in a moment.");
        }
    }

    pub fn delete_selected_keys(&mut self) -> bool {
        if self.selected_keys.is_empty() {
            return false;
        }
        let by_clip = keys_by_clip(&self.selected_keys);
        let ids: Vec<String> = by_clip.keys().cloned().collect();
        let next = update_clips(self.project(), &ids, |c| {
            let keys = &by_clip[&c.id];
            map_clip_keys(c, &mut |e| remove_keys(e, keys), &mut |t| remove_keys(t, keys))
        });
        self.edit(next, None);
        self.selected_keys.clear();
        true
    }

    pub fn delete_selected_clips(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let next = remove_clips(self.project(), &self.selection);
        self.edit(next, None);
        self.set_selection(Vec::new());
    }

    pub fn ripple_delete_selection(&mut self) {
        let Some((start, end)) = clips_span(self.project(), &self.selection) else { return };
        let next = ripple_delete(self.project(), start, end);
        self.edit(next, None);
        self.set_selection(Vec::new());
        self.playhead_sec = start;
    }

    pub fn zoom_by(&mut self, factor: f64, stage_width: f64) {
        let anchor_x = (self.playhead_sec - self.scroll_sec) * self.px_per_sec;
        let x = if (0.0..=stage_width).contains(&anchor_x) { anchor_x } else { 0.0 };
        (self.px_per_sec, self.scroll_sec) = zoom_about(self.scroll_sec, self.px_per_sec, x, self.px_per_sec * factor);
    }

    pub fn zoom_to_fit(&mut self, stage_width: f64) {
        self.px_per_sec = fit_zoom(stage_width, self.duration());
        self.scroll_sec = 0.0;
    }
}

pub fn pick_keys(current: &[String], reference: &str, additive: bool) -> Vec<String> {
    let held = current.iter().any(|r| r == reference);
    match (additive, held) {
        (true, true) => current.iter().filter(|r| *r != reference).cloned().collect(),
        (true, false) => current.iter().cloned().chain([reference.to_string()]).collect(),
        (false, true) => current.to_vec(),
        (false, false) => vec![reference.to_string()],
    }
}
