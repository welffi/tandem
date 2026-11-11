use crate::effects::{active_effects, is_hex_color, is_keyed};
use crate::framing::{FrameLook, frame_look};
use crate::keyframes::is_animated;
use crate::speed::{has_speed, local_at, media_sec, source_at, warp_points};
use crate::timeline::{clip_duration, clip_end, clip_fades, project_duration, sort_clips};
use crate::types::{Clip, Effect, Project, Track, TrackKind};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fades {
    pub in_sec: f64,
    pub out_sec: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoSource {
    pub path: String,
    pub in_sec: f64,
    pub offset_sec: f64,
    pub clip_sec: f64,
    pub width: u32,
    pub height: u32,
    pub effects: Vec<Effect>,
    pub speed_clip: Option<Clip>,
    pub anim_clip: Option<Clip>,
    pub still: bool,
    pub held: bool,
    pub fade_color: Option<String>,
    pub fades: Fades,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StillOverlay {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub effects: Vec<Effect>,
    pub offset_frames: i64,
    pub clip_frames: i64,
    pub anim_clip: Option<Clip>,
    pub fades: Fades,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdjustLayer {
    pub effects: Vec<Effect>,
    pub offset_frames: i64,
    pub clip_frames: i64,
    pub anim_clip: Option<Clip>,
    pub fades: Fades,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Overlay {
    Still(StillOverlay),
    Keyed(VideoSource),
    Blend(VideoSource),
    Adjust(AdjustLayer),
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoSegment {
    pub start_frame: i64,
    pub frames: i64,
    pub source: Option<VideoSource>,
    pub overlays: Vec<Overlay>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedWarp {
    pub points: Vec<(f64, f64)>,
    pub keep_pitch: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioPiece {
    pub path: String,
    pub stream: usize,
    pub in_sec: f64,
    pub duration_sec: f64,
    pub timeline_start_sec: f64,
    pub speed: Option<SpeedWarp>,
    pub volume: Option<f64>,
    pub fades: Fades,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextOverlayPlan {
    pub clip_id: String,
    pub start_sec: f64,
    pub duration_sec: f64,
    pub fades: Fades,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioTrackPlan {
    pub id: String,
    pub name: String,
    pub volume: f64,
    pub pieces: Vec<AudioPiece>,
}

#[derive(Clone, Debug)]
pub struct RenderPlan {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub look: FrameLook,
    pub total_frames: i64,
    pub duration_sec: f64,
    pub video: Vec<VideoSegment>,
    pub audio: Vec<AudioTrackPlan>,
    pub text: Vec<TextOverlayPlan>,
}

fn anim_of(clip: &Clip) -> Option<Clip> {
    is_animated(&active_effects(clip, None)).then(|| clip.clone())
}

fn fades_of(clip: &Clip) -> Fades {
    let (in_sec, out_sec) = clip_fades(clip);
    Fades { in_sec, out_sec }
}

struct FramedClip<'a> {
    clip: &'a Clip,
    path: String,
    still: bool,
    alpha: bool,
    width: u32,
    height: u32,
    last_in_sec: f64,
    start_frame: i64,
    end_frame: i64,
}

impl FramedClip<'_> {
    fn keyed(&self) -> bool {
        is_keyed(&active_effects(self.clip, None))
    }

    fn fade_frames(&self, fps: f64) -> (i64, i64) {
        let (fade_in, fade_out) = clip_fades(self.clip);
        (self.start_frame + (fade_in * fps).round() as i64, self.end_frame - (fade_out * fps).round() as i64)
    }

    fn blends(&self, frame: i64, fps: f64) -> bool {
        if !self.clip.dissolve || self.alpha || self.keyed() {
            return false;
        }
        let (in_end, out_start) = self.fade_frames(fps);
        frame < in_end || frame >= out_start
    }

    fn source(&self, f0: i64, fps: f64) -> VideoSource {
        let offset_sec = (f0 - self.start_frame) as f64 / fps;
        VideoSource {
            path: self.path.clone(),
            in_sec: self.last_in_sec.min(media_sec(self.clip, offset_sec).max(0.0)),
            offset_sec,
            clip_sec: (self.end_frame - self.start_frame) as f64 / fps,
            width: self.width,
            height: self.height,
            effects: active_effects(self.clip, None),
            speed_clip: (!self.still && has_speed(self.clip)).then(|| self.clip.clone()),
            anim_clip: anim_of(self.clip),
            still: self.still,
            held: self.clip.freeze_at.is_some(),
            fade_color: self.clip.fade_color.clone().filter(|c| is_hex_color(c)),
            fades: fades_of(self.clip),
        }
    }

    fn overlay(&self, f0: i64, fps: f64) -> Overlay {
        let offset_frames = f0 - self.start_frame;
        let clip_frames = self.end_frame - self.start_frame;
        if self.clip.adjustment {
            Overlay::Adjust(AdjustLayer { effects: active_effects(self.clip, None), offset_frames, clip_frames, anim_clip: anim_of(self.clip), fades: fades_of(self.clip) })
        } else if self.blends(f0, fps) {
            Overlay::Blend(self.source(f0, fps))
        } else if self.still {
            Overlay::Still(StillOverlay {
                path: self.path.clone(),
                width: self.width,
                height: self.height,
                effects: active_effects(self.clip, None),
                offset_frames,
                clip_frames,
                anim_clip: anim_of(self.clip),
                fades: fades_of(self.clip),
            })
        } else {
            Overlay::Keyed(self.source(f0, fps))
        }
    }
}

fn dissolve_track(track: &Track, fps: f64) -> Vec<Track> {
    let mut clips = track.clips.clone();
    sort_clips(&mut clips);
    let mut tails = Vec::new();
    for i in 0..clips.len().saturating_sub(1) {
        let a = clips[i].clone();
        let b = clips[i + 1].clone();
        let cut = clip_end(&a);
        if (b.timeline_start_sec - cut).abs() > 0.5 / fps {
            continue;
        }
        let out = if a.dissolve { clip_fades(&a).1.min(-local_at(&b, 0.0)) } else { 0.0 };
        let into = if b.dissolve { clip_fades(&b).0 } else { 0.0 };
        if out + into <= 0.0 {
            continue;
        }
        let duration = clip_duration(&a);
        let tail_in = source_at(&a, duration - out);
        tails.push(Clip {
            id: format!("{}~tail", a.id),
            source_in_sec: tail_in,
            source_out_sec: source_at(&a, duration + into),
            timeline_start_sec: cut - out,
            fade_in_sec: None,
            fade_out_sec: Some(out + into),
            dissolve: true,
            ..a.clone()
        });
        clips[i] = Clip { source_out_sec: tail_in, fade_out_sec: if a.dissolve { None } else { a.fade_out_sec }, ..a };
        clips[i + 1] = Clip { source_in_sec: source_at(&b, -out), timeline_start_sec: b.timeline_start_sec - out, fade_in_sec: if b.dissolve { None } else { b.fade_in_sec }, ..b };
    }
    if tails.is_empty() {
        return vec![track.clone()];
    }
    vec![Track { id: format!("{}~dissolve", track.id), clips: tails, ..track.clone() }, Track { clips, ..track.clone() }]
}

pub fn with_dissolves(project: &Project) -> Project {
    let tracks = project.tracks.iter().flat_map(|t| if t.kind == TrackKind::Video { dissolve_track(t, project.fps) } else { vec![t.clone()] }).collect();
    Project { tracks, ..project.clone() }
}

fn video_layers<'a>(project: &'a Project) -> Vec<Vec<FramedClip<'a>>> {
    let fps = project.fps;
    let assets: HashMap<&str, _> = project.assets.iter().map(|a| (a.id.as_str(), a)).collect();
    project
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Video && !t.hidden)
        .map(|t| {
            t.clips
                .iter()
                .map(|clip| {
                    let asset = assets.get(clip.asset_id.as_str());
                    FramedClip {
                        clip,
                        path: asset.map(|a| a.source_path.clone()).unwrap_or_default(),
                        still: asset.is_some_and(|a| a.still),
                        alpha: asset.is_some_and(|a| a.alpha),
                        width: asset.and_then(|a| a.width).filter(|w| *w > 0).unwrap_or(project.width),
                        height: asset.and_then(|a| a.height).filter(|h| *h > 0).unwrap_or(project.height),
                        last_in_sec: (asset.map_or(0.0, |a| a.duration_sec) - 1.0 / fps).max(0.0),
                        start_frame: (clip.timeline_start_sec * fps).round() as i64,
                        end_frame: (clip_end(clip) * fps).round() as i64,
                    }
                })
                .filter(|c| (!c.path.is_empty() || (c.clip.adjustment && !active_effects(c.clip, None).is_empty())) && c.end_frame > c.start_frame)
                .collect()
        })
        .collect()
}

fn video_segments(layers: &[Vec<FramedClip>], total_frames: i64, fps: f64) -> Vec<VideoSegment> {
    let mut cuts: BTreeSet<i64> = BTreeSet::from([0, total_frames]);
    for c in layers.iter().flatten() {
        cuts.insert(c.start_frame);
        cuts.insert(c.end_frame);
        if c.clip.dissolve {
            let (a, b) = c.fade_frames(fps);
            cuts.insert(a);
            cuts.insert(b);
        }
    }
    let boundaries: Vec<i64> = cuts.into_iter().filter(|f| *f >= 0 && *f <= total_frames).collect();

    let mut video: Vec<VideoSegment> = Vec::new();
    let mut last_key: Option<Vec<String>> = None;
    for pair in boundaries.windows(2) {
        let (f0, frames) = (pair[0], pair[1] - pair[0]);
        let mut top: Option<&FramedClip> = None;
        let mut above: Vec<&FramedClip> = Vec::new();
        for layer in layers {
            let Some(hit) = layer.iter().find(|c| f0 >= c.start_frame && f0 < c.end_frame) else {
                continue;
            };
            if hit.clip.adjustment || hit.alpha || hit.still || hit.keyed() || hit.blends(f0, fps) {
                above.insert(0, hit);
            } else {
                top = Some(hit);
                break;
            }
        }
        if top.is_none() && above.first().is_some_and(|c| c.still && !c.alpha && !c.keyed() && !c.blends(f0, fps)) {
            top = Some(above.remove(0));
        }
        let key: Vec<String> = std::iter::once(top).chain(above.iter().map(|c| Some(*c))).map(|c| c.map(|c| c.clip.id.clone()).unwrap_or_default()).collect();
        if last_key.as_ref() == Some(&key)
            && let Some(prev) = video.last_mut()
        {
            prev.frames += frames;
            continue;
        }
        video.push(VideoSegment { start_frame: f0, frames, source: top.map(|c| c.source(f0, fps)), overlays: above.iter().map(|c| c.overlay(f0, fps)).collect() });
        last_key = Some(key);
    }
    video
}

fn audio_tracks(project: &Project) -> Vec<AudioTrackPlan> {
    let assets: HashMap<&str, _> = project.assets.iter().map(|a| (a.id.as_str(), a)).collect();
    let mut audio = Vec::new();
    for track in project.tracks.iter().filter(|t| t.kind == TrackKind::Audio && !t.muted && t.volume > 0.0) {
        let mut clips = track.clips.clone();
        sort_clips(&mut clips);
        let pieces: Vec<AudioPiece> = clips
            .iter()
            .filter_map(|clip| {
                let asset = assets.get(clip.asset_id.as_str())?;
                let stream = clip.stream.unwrap_or(0);
                if stream >= asset.audio_streams.len() {
                    return None;
                }
                let duration = clip_duration(clip);
                Some(AudioPiece {
                    path: asset.source_path.clone(),
                    stream,
                    in_sec: clip.source_in_sec,
                    duration_sec: duration,
                    timeline_start_sec: clip.timeline_start_sec,
                    speed: has_speed(clip).then(|| SpeedWarp { points: warp_points(clip, 0.0, duration), keep_pitch: clip.keep_pitch }),
                    volume: clip.volume,
                    fades: fades_of(clip),
                })
            })
            .collect();
        if !pieces.is_empty() {
            audio.push(AudioTrackPlan { id: track.id.clone(), name: track.name.clone(), volume: track.volume, pieces });
        }
    }
    audio
}

fn text_overlays(project: &Project, end_sec: f64) -> Vec<TextOverlayPlan> {
    project
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Text && !t.hidden)
        .rev()
        .flat_map(|t| &t.clips)
        .filter(|c| c.text.as_ref().is_some_and(|s| !s.content.trim().is_empty()) && c.timeline_start_sec < end_sec)
        .map(|c| TextOverlayPlan { clip_id: c.id.clone(), start_sec: c.timeline_start_sec, duration_sec: clip_duration(c), fades: fades_of(c) })
        .collect()
}

pub fn build_render_plan(input: &Project) -> RenderPlan {
    let project = with_dissolves(input);
    let fps = project.fps;
    let total_frames = (project_duration(&project) * fps).round() as i64;
    let duration_sec = total_frames as f64 / fps;
    RenderPlan {
        width: project.width,
        height: project.height,
        fps,
        look: frame_look(&project),
        total_frames,
        duration_sec,
        video: video_segments(&video_layers(&project), total_frames, fps),
        audio: audio_tracks(&project),
        text: text_overlays(&project, duration_sec),
    }
}
