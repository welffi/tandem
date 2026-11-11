use crate::edits::{EDGE_EPS, find_clip, group_of, place_on_track, split_clips_at, update_clips, update_track};
use crate::ids::new_id;
use crate::keyframes::{map_clip_keys, map_keys};
use crate::speed::{local_at, source_at};
use crate::timeline::{MIN_CLIP_SEC, clip_duration, clip_end, next_clip_after, split_clip};
use crate::types::{Clip, Keyframe, Project, Track, TrackKind};

const EXTEND_FREEZE_SEC: f64 = 2.0;

fn remap_keys(clip: &Clip, map: impl Fn(&Keyframe) -> Keyframe) -> Clip {
    map_clip_keys(clip, &mut |e| map_keys(e, &mut |k| map(k)), &mut |t| map_keys(t, &mut |k| map(k)))
}

pub fn freeze_frame_at(project: &Project, clip_id: &str, at_sec: f64) -> Project {
    let Some((track, clip)) = find_clip(project, clip_id) else {
        return project.clone();
    };
    if track.kind != TrackKind::Video || clip.text.is_some() || clip.adjustment || clip.freeze_at.is_some() {
        return project.clone();
    }
    if project.assets.iter().find(|a| a.id == clip.asset_id).is_some_and(|a| a.still) {
        return project.clone();
    }
    let end = clip_end(clip);
    if at_sec >= end - MIN_CLIP_SEC {
        return if at_sec <= end + 1.0 / project.fps { hold_last_frame(project, track, clip) } else { project.clone() };
    }
    let split = split_clips_at(project, Some(&group_of(project, clip_id)), at_sec);
    let right = split.tracks.iter().find(|t| t.id == track.id).and_then(|t| t.clips.iter().find(|c| c.id != clip_id && (c.timeline_start_sec - at_sec).abs() < EDGE_EPS));
    let Some(right) = right.cloned() else {
        return project.clone();
    };
    let duration = clip_duration(&right);
    let held = Clip {
        freeze_at: Some(source_at(clip, at_sec - clip.timeline_start_sec)),
        resume_sec: Some(duration),
        source_in_sec: 0.0,
        source_out_sec: duration,
        speed_keys: None,
        keep_pitch: false,
        ..remap_keys(&right, |k| Keyframe { source_sec: local_at(&right, k.source_sec), ..k.clone() })
    };
    update_clips(&split, std::slice::from_ref(&right.id), |_| held.clone())
}

fn hold_last_frame(project: &Project, track: &Track, clip: &Clip) -> Project {
    let duration = clip_duration(clip);
    let end = clip.timeline_start_sec + duration;
    let next = next_clip_after(track, clip.timeline_start_sec);
    let length = EXTEND_FREEZE_SEC.min(next.map_or(f64::INFINITY, |n| n.timeline_start_sec - end));
    if length < MIN_CLIP_SEC {
        return project.clone();
    }
    let held = Clip {
        id: new_id(),
        fade_in_sec: None,
        fade_out_sec: None,
        dissolve: false,
        freeze_at: Some(clip.source_in_sec.max(source_at(clip, duration) - 1.0 / project.fps)),
        source_in_sec: 0.0,
        source_out_sec: length,
        timeline_start_sec: end,
        speed_keys: None,
        keep_pitch: false,
        ..remap_keys(clip, |k| Keyframe { source_sec: local_at(clip, k.source_sec) - duration, ..k.clone() })
    };
    place_on_track(project, &track.id, held)
}

pub fn resume_from_freeze(project: &Project, clip_id: &str, at_sec: f64) -> Project {
    let Some((track, clip)) = find_clip(project, clip_id) else {
        return project.clone();
    };
    let Some(from) = clip.freeze_at else {
        return project.clone();
    };
    let Some((held, rest)) = split_clip(clip, at_sec) else {
        return project.clone();
    };
    let plain = Clip { freeze_at: None, resume_sec: None, ..rest.clone() };
    let resume_sec = clip.resume_sec.unwrap_or_else(|| clip_duration(&rest));
    let delay = if clip.resume_sec.is_none() { 0.0 } else { at_sec - clip.timeline_start_sec };
    let old_end = clip.timeline_start_sec + resume_sec;
    let group = clip.group_id.clone();
    let mut shifted = project.clone();
    if delay > 0.0 {
        for c in shifted.tracks.iter_mut().flat_map(|t| &mut t.clips) {
            let later = c.id != clip_id && (c.timeline_start_sec >= old_end - EDGE_EPS || (group.is_some() && c.group_id == group));
            if later {
                c.timeline_start_sec += delay;
            }
        }
    }
    let asset_end = project.assets.iter().find(|a| a.id == rest.asset_id).map_or(f64::INFINITY, |a| a.duration_sec);
    let mut live =
        Clip { source_in_sec: from, source_out_sec: asset_end.min(from + resume_sec), ..remap_keys(&plain, |k| Keyframe { source_sec: from + k.source_sec - rest.source_in_sec, ..k.clone() }) };
    let plays_on = live.source_out_sec - from >= MIN_CLIP_SEC;
    let audio = if plays_on && clip.resume_sec.is_none() { audio_continuations(project, clip, &live, at_sec) } else { Vec::new() };
    if !audio.is_empty() {
        live.group_id = audio[0].1.group_id.clone();
    }
    let pieces = if plays_on { vec![held, live] } else { vec![held] };
    let resumed = update_track(&shifted, &track.id, |t| {
        let mut next = t.clone();
        next.clips = t.clips.iter().flat_map(|c| if c.id == clip_id { pieces.clone() } else { vec![c.clone()] }).collect();
        next
    });
    audio.into_iter().fold(resumed, |next, (track_id, piece)| place_on_track(&next, &track_id, piece))
}

fn audio_continuations(project: &Project, held: &Clip, live: &Clip, at_sec: f64) -> Vec<(String, Clip)> {
    let Some(group) = &held.group_id else { return Vec::new() };
    let linked = format!("{}-live", new_id());
    project
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .flat_map(|t| t.clips.iter().map(move |c| (t, c)))
        .filter(|(_, c)| c.group_id.as_ref() == Some(group) && c.asset_id == live.asset_id && (clip_end(c) - held.timeline_start_sec).abs() < EDGE_EPS)
        .map(|(t, c)| {
            let source_end = project.assets.iter().find(|a| a.id == c.asset_id).map_or(live.source_out_sec, |a| a.duration_sec);
            let piece = Clip {
                id: new_id(),
                group_id: Some(linked.clone()),
                timeline_start_sec: at_sec,
                source_in_sec: live.source_in_sec,
                source_out_sec: source_end.min(live.source_out_sec),
                fade_in_sec: None,
                fade_out_sec: None,
                speed_keys: None,
                ..c.clone()
            };
            (t.id.clone(), piece)
        })
        .collect()
}
