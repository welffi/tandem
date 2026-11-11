use crate::edits::find_clip;
use crate::freeze::freeze_frame_at;
use crate::timeline::{MIN_CLIP_SEC, clip_end};
use crate::types::{Project, TrackKind};

#[derive(Clone, Debug, PartialEq)]
pub struct ClipMenu {
    pub split: bool,
    pub freeze: Option<bool>,
    pub end_freeze: Option<bool>,
    pub trim_silence: bool,
    pub reverse: bool,
}

pub fn clip_menu(project: &Project, selection: &[String], clip_id: &str, playhead: f64) -> Option<ClipMenu> {
    let (track, clip) = find_clip(project, clip_id)?;
    let split = project.tracks.iter().flat_map(|t| &t.clips).any(|c| selection.contains(&c.id) && playhead > c.timeline_start_sec && playhead < clip_end(c));
    let held = clip.freeze_at.is_some();
    let still = project.assets.iter().find(|a| a.id == clip.asset_id).is_some_and(|a| a.still);
    let freezable = track.kind == TrackKind::Video && clip.text.is_none() && !clip.adjustment && !held && !still;
    let under_playhead = playhead > clip.timeline_start_sec + MIN_CLIP_SEC && playhead < clip_end(clip) - MIN_CLIP_SEC;
    Some(ClipMenu {
        split,
        freeze: freezable.then(|| freeze_frame_at(project, clip_id, playhead) != *project),
        end_freeze: held.then_some(under_playhead),
        trim_silence: track.kind == TrackKind::Audio,
        reverse: track.kind != TrackKind::Text && !clip.adjustment && !held,
    })
}
