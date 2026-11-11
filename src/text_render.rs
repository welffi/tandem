use crate::effects::{active_effects, grayscale_amount, hex_rgb, on_text};
use crate::fonts::font;
use crate::framing::Rect;
use crate::keyframes::effect_at;
use crate::looks::{apply_drop_shadows, grayscale, shadow_reach};
use crate::raster::{Image, Region, over_at, region_around};
use crate::speed::source_at;
use crate::timeline::{clip_at, fade_gain};
use crate::types::{Effect, Project, TextAlign, TextStyle, TrackKind};
use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use rayon::prelude::*;
use std::sync::{Arc, LazyLock, Mutex};

const LINE_HEIGHT: f64 = 1.2;
const BOX_ALPHA: f32 = 0.6;
const OUTLINE_ALPHA: f32 = 0.9;
const CACHED_LAYERS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

struct Laid {
    font: FontArc,
    scale: PxScale,
    px: f64,
    lines: Vec<(String, f64)>,
    block_w: f64,
    left: f64,
    top: f64,
    bounds: TextBounds,
}

fn line_width(font: &FontArc, scale: PxScale, line: &str) -> f64 {
    let scaled = font.as_scaled(scale);
    let mut width = 0.0;
    let mut last = None;
    for c in line.chars() {
        let id = scaled.glyph_id(c);
        if let Some(prev) = last {
            width += scaled.kern(prev, id);
        }
        width += scaled.h_advance(id);
        last = Some(id);
    }
    width as f64
}

pub fn line_left(align: TextAlign, left: f64, block_w: f64, line_w: f64) -> f64 {
    match align {
        TextAlign::Left => left,
        TextAlign::Right => left + block_w - line_w,
        TextAlign::Center => left + (block_w - line_w) / 2.0,
    }
}

pub fn block_bounds(style: &TextStyle, frame_w: f64, frame_h: f64, block_w: f64, lines: usize) -> (f64, f64, TextBounds) {
    let px = style.size / 100.0 * frame_h;
    let block_h = lines as f64 * px * LINE_HEIGHT;
    let left = style.x * frame_w - block_w / 2.0;
    let top = style.y * frame_h - block_h / 2.0;
    let (pad_x, pad_y) = (px * 0.35, px * 0.15);
    (left, top, TextBounds { x: left - pad_x, y: top - pad_y, w: block_w + pad_x * 2.0, h: block_h + pad_y * 2.0 })
}

fn layout(style: &TextStyle, frame_w: f64, frame_h: f64) -> Option<Laid> {
    let font = font(&style.font, style.bold, style.italic)?;
    let px = style.size / 100.0 * frame_h;
    let scale = PxScale::from((px * font.height_unscaled() as f64 / font.units_per_em().unwrap_or(1000.0) as f64) as f32);
    let lines: Vec<(String, f64)> = style.content.split('\n').map(|l| (l.to_string(), line_width(&font, scale, l))).collect();
    let block_w = lines.iter().map(|l| l.1).fold(0.0, f64::max);
    let (left, top, bounds) = block_bounds(style, frame_w, frame_h, block_w, lines.len());
    Some(Laid { font, scale, px, lines, block_w, left, top, bounds })
}

pub fn text_bounds(style: &TextStyle, frame_w: f64, frame_h: f64) -> Option<TextBounds> {
    layout(style, frame_w, frame_h).map(|l| l.bounds)
}

pub fn rounded_box_cover(x: f64, y: f64, b: &TextBounds, radius: f64) -> f64 {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    let r = radius.min(b.w / 2.0).min(b.h / 2.0);
    let qx = (x - cx).abs() - (b.w / 2.0 - r);
    let qy = (y - cy).abs() - (b.h / 2.0 - r);
    let outside = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
    (0.5 - outside).clamp(0.0, 1.0)
}

fn draw_box(frame: &mut Image, b: &TextBounds, radius: f64, alpha: f64) {
    let (w, h) = (frame.w, frame.h);
    let y0 = b.y.floor().max(0.0) as usize;
    let y1 = ((b.y + b.h).ceil().max(0.0) as usize).min(h);
    let x0 = b.x.floor().max(0.0) as usize;
    let x1 = ((b.x + b.w).ceil().max(0.0) as usize).min(w);
    frame.px.par_chunks_mut(w * 4).enumerate().skip(y0).take(y1.saturating_sub(y0)).for_each(|(y, row)| {
        for x in x0..x1 {
            let a = rounded_box_cover(x as f64 + 0.5, y as f64 + 0.5, b, radius) as f32 * BOX_ALPHA * alpha as f32;
            for c in &mut row[x * 4..x * 4 + 4] {
                *c = (*c as f32 * (1.0 - a)).round() as u8;
            }
            row[x * 4 + 3] = (row[x * 4 + 3] as f32 + 255.0 * a).min(255.0) as u8;
        }
    });
}

fn fill_mask(laid: &Laid, style: &TextStyle, r: Region) -> Vec<f32> {
    let (w, h) = (r.w, r.h);
    let mut mask = vec![0f32; w * h];
    let scaled = laid.font.as_scaled(laid.scale);
    let line_h = laid.px * LINE_HEIGHT;
    let middle = (scaled.ascent() + scaled.descent()) as f64 / 2.0;
    for (i, (line, line_w)) in laid.lines.iter().enumerate() {
        let baseline = laid.top + line_h * (i as f64 + 0.5) + middle;
        let mut x = line_left(style.align, laid.left, laid.block_w, *line_w) as f32;
        let mut last = None;
        for c in line.chars() {
            let id = scaled.glyph_id(c);
            if let Some(prev) = last {
                x += scaled.kern(prev, id);
            }
            let glyph = id.with_scale_and_position(laid.scale, point(x, baseline as f32));
            if let Some(outline) = laid.font.outline_glyph(glyph) {
                let px = outline.px_bounds();
                outline.draw(|gx, gy, cover| {
                    let (fx, fy) = (px.min.x as i64 + gx as i64 - r.x as i64, px.min.y as i64 + gy as i64 - r.y as i64);
                    if fx >= 0 && fy >= 0 && (fx as usize) < w && (fy as usize) < h {
                        let m = &mut mask[fy as usize * w + fx as usize];
                        *m = (*m + cover).min(1.0);
                    }
                });
            }
            x += scaled.h_advance(id);
            last = Some(id);
        }
    }
    mask
}

pub fn dilate(mask: &[f32], w: usize, h: usize, radius: f64) -> Vec<f32> {
    let reach = radius.ceil() as i64 + 1;
    let offsets: Vec<(i64, i64, f32)> = (-reach..=reach)
        .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let weight = (radius + 0.5 - (dx as f64).hypot(dy as f64)).clamp(0.0, 1.0) as f32;
            (weight > 0.0).then_some((dx, dy, weight))
        })
        .collect();
    let mut out = vec![0f32; w * h];
    out.par_chunks_mut(w.max(1)).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let mut best = 0f32;
            for (dx, dy, weight) in &offsets {
                let (sx, sy) = (x as i64 + dx, y as i64 + dy);
                if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                    best = best.max(mask[sy as usize * w + sx as usize] * weight);
                }
            }
            *o = best;
        }
    });
    out
}

fn outline_radius(px: f64) -> f64 {
    (px * 0.14).max(1.0) / 2.0
}

fn text_layer(laid: &Laid, style: &TextStyle, r: Region) -> Image {
    let fill = fill_mask(laid, style, r);
    let outline = style.outline.then(|| dilate(&fill, r.w, r.h, outline_radius(laid.px)));
    let rgb = hex_rgb(&style.color).unwrap_or([255, 255, 255]).map(|c| c as f32);
    let mut layer = Image::new(r.w, r.h);
    layer.px.par_chunks_mut(4).enumerate().for_each(|(i, p)| {
        let f = fill[i];
        let o = outline.as_ref().map_or(0.0, |m| m[i] * OUTLINE_ALPHA);
        let a = f + o * (1.0 - f);
        if a > 0.0 {
            for k in 0..3 {
                p[k] = (rgb[k] * f).round() as u8;
            }
            p[3] = (a * 255.0).round() as u8;
        }
    });
    layer
}

pub fn draw_text(frame: &mut Image, style: &TextStyle, alpha: f64, effects: &[Effect]) -> Option<TextBounds> {
    let laid = layout(style, frame.w as f64, frame.h as f64)?;
    if style.boxed {
        draw_box(frame, &laid.bounds, laid.px * 0.2, alpha);
    }
    let (layer, r) = finished_layer(&laid, style, effects, frame.w, frame.h);
    over_at(frame, &layer, r.x, r.y, alpha);
    Some(laid.bounds)
}

type LayerKey = (TextStyle, Vec<Effect>, usize, usize);

static LAYERS: LazyLock<Mutex<Vec<(LayerKey, Arc<Image>, Region)>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn finished_layer(laid: &Laid, style: &TextStyle, effects: &[Effect], frame_w: usize, frame_h: usize) -> (Arc<Image>, Region) {
    let key = (style.clone(), effects.to_vec(), frame_w, frame_h);
    if let Some((_, layer, r)) = LAYERS.lock().unwrap().iter().find(|(k, ..)| *k == key) {
        return (layer.clone(), *r);
    }
    let b = laid.bounds;
    let grow = laid.px + outline_radius(laid.px) + 2.0 + shadow_reach(effects, frame_h as f64);
    let r = region_around(Rect { x: b.x, y: b.y, w: b.w, h: b.h }, grow, frame_w, frame_h);
    let layer = if r.w > 0 && r.h > 0 {
        let mut layer = text_layer(laid, style, r);
        grayscale(&mut layer, grayscale_amount(effects));
        apply_drop_shadows(layer, effects, frame_h as f64)
    } else {
        Image::new(0, 0)
    };
    let layer = Arc::new(layer);
    let mut layers = LAYERS.lock().unwrap();
    if layers.len() >= CACHED_LAYERS {
        layers.remove(0);
    }
    layers.push((key, layer.clone(), r));
    (layer, r)
}

pub fn draw_texts(frame: &mut Image, project: &Project, time_sec: f64) -> Vec<(String, TextBounds)> {
    let mut drawn = Vec::new();
    for track in project.tracks.iter().rev().filter(|t| t.kind == TrackKind::Text && !t.hidden) {
        let Some(clip) = clip_at(track, time_sec) else {
            continue;
        };
        let Some(text) = &clip.text else { continue };
        let local = time_sec - clip.timeline_start_sec;
        let style = effect_at(text, source_at(clip, local));
        let effects: Vec<Effect> = active_effects(clip, Some(local)).into_iter().filter(|e| on_text(e.kind)).collect();
        if let Some(bounds) = draw_text(frame, &style, fade_gain(clip, time_sec), &effects) {
            drawn.push((clip.id.clone(), bounds));
        }
    }
    drawn
}
