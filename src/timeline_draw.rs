use crate::edits::stream_track_name;
use crate::effects::active_effects;
use crate::keyframes::clip_keys;
use crate::media::Thumbnails;
use crate::speed::{has_speed, local_at, media_sec, sorted_keys, source_at};
use crate::store::key_ref;
use crate::theme::*;
use crate::timeline::{TrimSide, clip_fades};
use crate::timeline_math::{FADE_INSET_PX, KEY_R, LANE_PAD, Lane, clip_width, fade_handle_x, key_x};
use crate::timeline_state::TimelineState;
use crate::types::{Clip, MediaAsset, Peaks, Track, TrackKind};
use crate::widgets::truncated;
use eframe::egui::{Align2, Color32, Context, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};

pub struct Geo {
    pub stage: Rect,
    pub v_scroll: f32,
    pub scroll_sec: f64,
    pub px: f64,
}

impl Geo {
    pub fn x_at(&self, sec: f64) -> f32 {
        self.stage.left() + ((sec - self.scroll_sec) * self.px) as f32
    }

    pub fn time_at(&self, x: f32) -> f64 {
        self.scroll_sec + (x - self.stage.left()) as f64 / self.px
    }

    pub fn lane_top(&self, lane: &Lane) -> f32 {
        self.stage.top() + lane.y - self.v_scroll
    }

    pub fn clip_rect(&self, lane: &Lane, clip: &Clip) -> Rect {
        Rect::from_min_size(pos2(self.x_at(clip.timeline_start_sec), self.lane_top(lane) + LANE_PAD), vec2(clip_width(clip, self.px), lane.h - LANE_PAD * 2.0))
    }
}

pub struct ClipParts<'a> {
    pub asset: Option<&'a MediaAsset>,
    pub thumbs: Option<&'a Thumbnails>,
    pub peaks: Option<&'a Peaks>,
    pub peaks_key: String,
}

pub struct ClipLook {
    pub selected: bool,
    pub dim: bool,
    pub handles: bool,
    pub linked: bool,
}

fn speed_tag(clip: &Clip) -> String {
    if !has_speed(clip) {
        return String::new();
    }
    let mut speeds: Vec<f64> = clip.speed_keys().iter().map(|k| k.speed).collect();
    speeds.dedup();
    if speeds.len() == 1 { format!(" · {}×", speeds[0]) } else { " · ramp".into() }
}

fn clip_tags(clip: &Clip) -> String {
    let freeze = if clip.freeze_at.is_some() { " · freeze" } else { "" };
    let effects = if active_effects(clip, None).is_empty() { "" } else { " · fx" };
    format!("{freeze}{}{effects}", speed_tag(clip))
}

pub fn clip_label(clip: &Clip, asset: Option<&MediaAsset>) -> String {
    if let Some(text) = &clip.text {
        let first = text.content.lines().next().unwrap_or("");
        return if first.is_empty() { "Text".into() } else { first.into() };
    }
    if clip.adjustment {
        return format!("Adjustment{}", clip_tags(clip));
    }
    let Some(asset) = asset else { return "missing".into() };
    match clip.stream {
        Some(stream) if asset.audio_streams.len() >= 2 => format!("{} · {}{}", asset.name, stream_track_name(asset, stream), clip_tags(clip)),
        _ => format!("{}{}", asset.name, clip_tags(clip)),
    }
}

pub fn diamond(center: Pos2, radius: f32) -> Vec<Pos2> {
    vec![center + vec2(0.0, -radius), center + vec2(radius, 0.0), center + vec2(0.0, radius), center + vec2(-radius, 0.0)]
}

fn draw_thumbs(painter: &Painter, state: &mut TimelineState, ctx: &Context, rect: Rect, clip: &Clip, parts: &ClipParts, px: f64, range: (f32, f32)) {
    let Some(thumbs) = parts.thumbs.filter(|t| !t.files.is_empty()) else { return };
    let aspect = parts.asset.and_then(|a| Some(a.width? as f32 / a.height?.max(1) as f32)).unwrap_or(16.0 / 9.0);
    let tile = (rect.height() * aspect).max(24.0);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let mut i = (range.0 / tile).floor() as i32;
    while (i as f32) * tile < range.1 {
        let tx = i as f32 * tile;
        let source = media_sec(clip, ((tx + tile / 2.0) as f64 / px).max(0.0));
        let index = ((source / thumbs.interval_sec).floor().max(0.0) as usize).min(thumbs.files.len() - 1);
        if let Some(texture) = state.texture(ctx, &thumbs.files[index]) {
            painter.image(texture, Rect::from_min_size(pos2(rect.left() + tx, rect.top()), vec2(tile, rect.height())), uv, Color32::WHITE);
        }
        i += 1;
    }
}

fn draw_wave(painter: &Painter, state: &mut TimelineState, rect: Rect, clip: &Clip, parts: &ClipParts, px: f64, range: (f32, f32)) {
    let Some(peaks) = parts.peaks else { return };
    let top = 15.0;
    let band = rect.height() - top - 2.0;
    let mid = rect.top() + top + band / 2.0;
    let norm = state.peak_norm(&parts.peaks_key, peaks);
    let per_sec = peaks.peaks_rate / 2.0;
    let buckets = peaks.peaks.len() / 2;
    let mut mesh = Mesh::default();
    for column in (range.0.floor() as i32)..(range.1.ceil() as i32) {
        let from = source_at(clip, (column as f64 / px).max(0.0));
        let to = source_at(clip, ((column + 1) as f64 / px).max(0.0));
        let b0 = ((from * per_sec).floor().max(0.0) as usize).min(buckets);
        let b1 = ((to * per_sec).floor() as usize).max(b0 + 1).min(buckets);
        let (mut low, mut high) = (0.0_f32, 0.0_f32);
        for b in b0..b1 {
            low = low.min(peaks.peaks[2 * b]);
            high = high.max(peaks.peaks[2 * b + 1]);
        }
        let y0 = mid - high / norm * band / 2.0;
        let y1 = mid - low / norm * band / 2.0;
        let x = rect.left() + column as f32;
        mesh.add_colored_rect(Rect::from_min_max(pos2(x, y0), pos2(x + 1.0, y1.max(y0 + 1.0))), WAVE);
    }
    painter.add(mesh);
}

fn draw_fades(painter: &Painter, rect: Rect, clip: &Clip, px: f64, handles: bool) {
    let (fade_in, fade_out) = clip_fades(clip);
    let (w, h) = (rect.width(), rect.height());
    let in_x = fade_in as f32 * px as f32;
    let out_x = w - fade_out as f32 * px as f32;
    let at = |x: f32, y: f32| rect.min + vec2(x, y);
    let line = Stroke::new(1.0, FADE_LINE);
    if in_x > 0.0 {
        painter.add(Shape::convex_polygon(vec![at(0.0, 0.0), at(in_x, 0.0), at(0.0, h)], FADE_SHADE, Stroke::NONE));
        painter.line_segment([at(0.0, h), at(in_x, 0.0)], line);
    }
    if out_x < w {
        painter.add(Shape::convex_polygon(vec![at(w, 0.0), at(out_x, 0.0), at(w, h)], FADE_SHADE, Stroke::NONE));
        painter.line_segment([at(w, h), at(out_x, 0.0)], line);
    }
    if !handles || w < 30.0 {
        return;
    }
    for side in [TrimSide::Start, TrimSide::End] {
        painter.circle(at(fade_handle_x(clip, side, w, px), FADE_INSET_PX), 3.5, Color32::WHITE, Stroke::new(1.0, FADE_HANDLE_LINE));
    }
}

fn draw_effect_keys(painter: &Painter, rect: Rect, clip: &Clip, px: f64, selected_keys: &[String]) {
    for key in clip_keys(clip) {
        let fill = if selected_keys.contains(&key_ref(&clip.id, &key.id)) { Color32::WHITE } else { KEY_FILL };
        let center = pos2(rect.left() + key_x(clip, &key, px), rect.center().y);
        painter.add(Shape::convex_polygon(diamond(center, KEY_R), fill, Stroke::new(1.0, KEY_LINE)));
    }
}

fn draw_speed_keys(painter: &Painter, rect: Rect, clip: &Clip, px: f64) {
    for key in sorted_keys(clip) {
        let center = pos2(rect.left() + (local_at(clip, key.source_sec) * px) as f32, rect.bottom() - 6.0);
        painter.add(Shape::convex_polygon(diamond(center, 4.0), SPEED_KEY, Stroke::NONE));
    }
}

fn fill_of(track: &Track, clip: &Clip) -> Color32 {
    if clip.adjustment {
        return CLIP_ADJUST;
    }
    match track.kind {
        TrackKind::Text => CLIP_TEXT,
        TrackKind::Video => CLIP_VIDEO,
        TrackKind::Audio => CLIP_AUDIO,
    }
}

pub struct ClipJob<'a> {
    pub geo: &'a Geo,
    pub lane: &'a Lane,
    pub track: &'a Track,
    pub clip: &'a Clip,
    pub parts: ClipParts<'a>,
    pub look: ClipLook,
    pub selected_keys: &'a [String],
}

pub fn draw_clip(painter: &Painter, state: &mut TimelineState, ctx: &Context, job: ClipJob) {
    let ClipJob { geo, lane, track, clip, parts, look, selected_keys } = job;
    let rect = geo.clip_rect(lane, clip);
    if rect.right() < geo.stage.left() || rect.left() > geo.stage.right() {
        return;
    }
    let mut painter = painter.with_clip_rect(rect.expand(1.0).intersect(geo.stage));
    if look.dim {
        painter.set_opacity(0.45);
    }
    let range = ((geo.stage.left() - rect.left()).max(0.0), (geo.stage.right() - rect.left()).min(rect.width()));
    painter.rect_filled(rect, 3.0, fill_of(track, clip));
    if track.kind != TrackKind::Text && !clip.adjustment {
        if track.kind == TrackKind::Video {
            draw_thumbs(&painter, state, ctx, rect, clip, &parts, geo.px, range);
        } else {
            draw_wave(&painter, state, rect, clip, &parts, geo.px, range);
        }
    }
    if !clip.speed_keys().is_empty() {
        draw_speed_keys(&painter, rect, clip, geo.px);
    }
    if look.selected {
        draw_effect_keys(&painter, rect, clip, geo.px, selected_keys);
    }
    draw_fades(&painter, rect, clip, geo.px, look.handles);
    let label_x = range.0 + 4.0;
    let label_rect = Rect::from_min_size(pos2(rect.left() + label_x, rect.top() + 3.0), vec2((rect.width() - label_x - 4.0).max(0.0), 14.0));
    let label = clip_label(clip, parts.asset);
    truncated(&painter, label_rect.translate(vec2(1.0, 1.0)), Align2::LEFT_TOP, &label, FontId::proportional(11.0), CLIP_SHADOW);
    truncated(&painter, label_rect, Align2::LEFT_TOP, &label, FontId::proportional(11.0), CLIP_LABEL);
    let outline = if look.selected { Stroke::new(2.0, Color32::WHITE) } else { Stroke::new(1.0, CLIP_OUTLINE) };
    painter.rect_stroke(rect, 3.0, outline, StrokeKind::Inside);
    if look.linked {
        painter.rect_filled(Rect::from_min_size(pos2(rect.left() + 3.0, rect.bottom() - 3.0), vec2((rect.width() - 6.0).max(0.0), 2.0)), 0.0, CLIP_LINK);
    }
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(3.0, rect.height())), 0.0, CLIP_EDGE);
    painter.rect_filled(Rect::from_min_size(pos2(rect.right() - 3.0, rect.top()), vec2(3.0, rect.height())), 0.0, CLIP_EDGE);
}
