use crate::effects::{ChromaKey, DropShadow, Screen, chroma_key_of, drop_shadow_of, grayscale_amount};
use crate::raster::{Image, blur, mul, over};
use crate::types::{Effect, EffectKind};
use rayon::prelude::*;

pub fn grayscale(img: &mut Image, amount: f64) {
    if amount <= 0.0 {
        return;
    }
    let t = amount.min(1.0) as f32;
    img.px.par_chunks_mut(4).for_each(|p| {
        let luma = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
        for c in &mut p[..3] {
            *c = (*c as f32 + (luma - *c as f32) * t).round() as u8;
        }
    });
}

fn chroma(rgb: [f32; 3]) -> [f32; 2] {
    let [r, g, b] = rgb;
    [-0.16874 * r - 0.33126 * g + 0.5 * b, 0.5 * r - 0.41869 * g - 0.08131 * b]
}

fn straight(p: &[u8]) -> [f32; 3] {
    let a = p[3] as f32;
    if a <= 0.0 {
        return [0.0; 3];
    }
    [p[0] as f32 / a, p[1] as f32 / a, p[2] as f32 / a]
}

pub fn key_alpha(diff: f32, similarity: f32, blend: f32) -> f32 {
    if blend > 0.0001 {
        ((diff - similarity) / blend).clamp(0.0, 1.0)
    } else if diff >= similarity {
        1.0
    } else {
        0.0
    }
}

pub fn despill(rgb: [f32; 3], screen: Option<Screen>, spill: f32) -> [f32; 3] {
    let [r, g, b] = rgb;
    match screen {
        Some(Screen::Green) => [r, g - spill * (g - (r + b) * 0.5).max(0.0), b],
        Some(Screen::Blue) => [r, g, b - spill * (b - (r + g) * 0.5).max(0.0)],
        None => rgb,
    }
}

pub fn chroma_key(img: &mut Image, key: &ChromaKey) {
    let (w, h) = (img.w, img.h);
    if w == 0 || h == 0 {
        return;
    }
    let k = chroma(key.rgb.map(|c| c as f32));
    let diffs: Vec<f32> = img
        .px
        .par_chunks(4)
        .map(|p| {
            let [u, v] = chroma(straight(p));
            (((u - k[0]).powi(2) + (v - k[1]).powi(2)) / 2.0).sqrt()
        })
        .collect();
    let (similarity, blend, spill) = (key.similarity as f32, key.blend as f32, key.spill as f32);
    img.px.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let mut sum = 0.0;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let sx = (x as i64 + dx).clamp(0, w as i64 - 1) as usize;
                    let sy = (y as i64 + dy).clamp(0, h as i64 - 1) as usize;
                    sum += diffs[sy * w + sx];
                }
            }
            let p = &mut row[x * 4..x * 4 + 4];
            let alpha = key_alpha(sum / 9.0, similarity, blend) * p[3] as f32 / 255.0;
            let rgb = despill(straight(p), key.screen, spill);
            for c in 0..3 {
                p[c] = (rgb[c].clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
            }
            p[3] = (alpha * 255.0).round() as u8;
        }
    });
}

pub fn apply_colors(img: &mut Image, effects: &[Effect]) {
    grayscale(img, grayscale_amount(effects));
    if let Some(effect) = effects.iter().find(|e| e.kind == EffectKind::ChromaKey) {
        chroma_key(img, &chroma_key_of(effect));
    }
}

fn shifted_alpha(layer: &Image, dx: i64, dy: i64) -> Vec<u8> {
    let (w, h) = (layer.w as i64, layer.h as i64);
    let mut alpha = vec![0u8; layer.w * layer.h];
    alpha.par_chunks_mut(layer.w.max(1)).enumerate().for_each(|(y, row)| {
        let sy = y as i64 - dy;
        if sy < 0 || sy >= h {
            return;
        }
        for (x, a) in row.iter_mut().enumerate() {
            let sx = x as i64 - dx;
            if sx >= 0 && sx < w {
                *a = layer.px[((sy * w + sx) * 4 + 3) as usize];
            }
        }
    });
    alpha
}

pub fn with_drop_shadow(layer: &Image, shadow: &DropShadow) -> Image {
    let mut alpha = shifted_alpha(layer, shadow.dx.round() as i64, shadow.dy.round() as i64);
    blur(&mut alpha, layer.w, layer.h, 1, shadow.blur / 2.0);
    let opacity = (shadow.opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    let mut out = Image::new(layer.w, layer.h);
    out.px.par_chunks_mut(4).zip(alpha.par_iter()).for_each(|(p, a)| {
        let a = mul(*a as u32, opacity) as u32;
        p[0] = mul(shadow.rgb[0] as u32, a);
        p[1] = mul(shadow.rgb[1] as u32, a);
        p[2] = mul(shadow.rgb[2] as u32, a);
        p[3] = a as u8;
    });
    over(&mut out, layer, 1.0);
    out
}

pub fn apply_drop_shadows(layer: Image, effects: &[Effect], frame_h: f64) -> Image {
    effects.iter().filter(|e| e.kind == EffectKind::DropShadow).fold(layer, |img, e| with_drop_shadow(&img, &drop_shadow_of(e, frame_h)))
}

pub fn shadow_reach(effects: &[Effect], frame_h: f64) -> f64 {
    effects
        .iter()
        .filter(|e| e.kind == EffectKind::DropShadow)
        .map(|e| {
            let s = drop_shadow_of(e, frame_h);
            s.dx.abs().ceil() + s.dy.abs().ceil() + s.blur * 1.5 + 2.0
        })
        .sum()
}

pub fn has_drop_shadow(effects: &[Effect]) -> bool {
    effects.iter().any(|e| e.kind == EffectKind::DropShadow)
}
