use crate::keyframes::clip_keys;
use crate::timeline::TrimSide;
use crate::timeline_draw::Geo;
use crate::timeline_math::{Lane, edge_hit, fade_hit, key_hit, lane_at};
use crate::types::Project;
use eframe::egui::Pos2;

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Empty,
    Body(String),
    Edge(String, TrimSide),
    Fade(String, TrimSide),
    Key(String, String),
}

impl Hit {
    pub fn clip_id(&self) -> Option<&str> {
        match self {
            Hit::Empty => None,
            Hit::Body(id) | Hit::Edge(id, _) | Hit::Fade(id, _) | Hit::Key(id, _) => Some(id),
        }
    }
}

pub fn lane_under(lanes: &[Lane], geo: &Geo, y: f32) -> Option<Lane> {
    lane_at(lanes, y - geo.stage.top() + geo.v_scroll)
}

pub fn hit_test(project: &Project, lanes: &[Lane], geo: &Geo, selected: &[String], at: Pos2) -> Hit {
    let Some(lane) = lane_under(lanes, geo, at.y) else { return Hit::Empty };
    let Some(track) = project.tracks.get(lane.index) else { return Hit::Empty };
    let Some(clip) = track.clips.iter().rev().find(|c| geo.clip_rect(&lane, c).contains(at)) else { return Hit::Empty };
    let rect = geo.clip_rect(&lane, clip);
    let local = at - rect.min;
    let id = clip.id.clone();
    if selected.contains(&id)
        && let Some(key) = key_hit(clip, &clip_keys(clip), rect.height(), geo.px, local.x, local.y)
    {
        return Hit::Key(id, key.id);
    }
    if let Some(side) = fade_hit(clip, rect.width(), geo.px, local.x, local.y) {
        return Hit::Fade(id, side);
    }
    match edge_hit(rect.width(), local.x) {
        Some(side) => Hit::Edge(id, side),
        None => Hit::Body(id),
    }
}
