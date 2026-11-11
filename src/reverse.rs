use crate::edits::{mirror_clip, update_clips};
use crate::types::{MediaAsset, Project, ReverseSource, TrackKind};

#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    pub asset_id: String,
    pub in_sec: f64,
    pub out_sec: f64,
    pub video: bool,
    pub clip_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    pub clip_ids: Vec<String>,
    pub asset_id: String,
    pub pivot_sec: f64,
}

#[derive(Default, Debug, PartialEq)]
pub struct ReversePlan {
    pub ranges: Vec<Range>,
    pub swaps: Vec<Swap>,
}

pub fn plan_reverse(project: &Project, selection: &[String]) -> ReversePlan {
    let mut plan = ReversePlan::default();
    for track in &project.tracks {
        for clip in track.clips.iter().filter(|c| selection.contains(&c.id) && c.text.is_none() && c.freeze_at.is_none()) {
            let Some(asset) = project.assets.iter().find(|a| a.id == clip.asset_id).filter(|a| !a.still) else { continue };
            if let Some(source) = &asset.reverse_of {
                if project.assets.iter().any(|a| a.id == source.asset_id) {
                    plan.swaps.push(Swap { clip_ids: vec![clip.id.clone()], asset_id: source.asset_id.clone(), pivot_sec: source.out_sec });
                }
                continue;
            }
            let video = track.kind == TrackKind::Video;
            match plan.ranges.iter_mut().find(|r| r.asset_id == asset.id && r.in_sec == clip.source_in_sec && r.out_sec == clip.source_out_sec) {
                Some(range) => {
                    range.video |= video;
                    range.clip_ids.push(clip.id.clone());
                }
                None => plan.ranges.push(Range { asset_id: asset.id.clone(), in_sec: clip.source_in_sec, out_sec: clip.source_out_sec, video, clip_ids: vec![clip.id.clone()] }),
            }
        }
    }
    plan
}

pub fn reversed_asset(mut made: MediaAsset, source: &MediaAsset, range: &Range) -> MediaAsset {
    made.name = format!("{} (reversed)", source.name);
    made.reverse_of = Some(ReverseSource { asset_id: source.id.clone(), in_sec: range.in_sec, out_sec: range.out_sec, video: range.video });
    made
}

pub fn apply_swaps(project: &Project, added: Vec<MediaAsset>, swaps: &[Swap]) -> Project {
    let mut next = project.clone();
    for asset in added {
        if !next.assets.iter().any(|a| a.id == asset.id) {
            next.assets.push(asset);
        }
    }
    swaps.iter().fold(next, |next, swap| update_clips(&next, &swap.clip_ids, |c| mirror_clip(c, &swap.asset_id, swap.pivot_sec)))
}
