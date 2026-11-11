use crate::framing::Rect;
use rayon::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Image {
    pub fn new(w: usize, h: usize) -> Image {
        Image { w, h, px: vec![0; w * h * 4] }
    }

    pub fn filled(w: usize, h: usize, rgba: [u8; 4]) -> Image {
        Image { w, h, px: rgba.repeat(w * h) }
    }

    pub fn from_straight(w: usize, h: usize, mut px: Vec<u8>) -> Image {
        px.par_chunks_mut(4).for_each(|p| {
            if p[3] < 255 {
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = mul(*c as u32, a);
                }
            }
        });
        Image { w, h, px }
    }

    pub fn clear(&mut self) {
        self.px.fill(0);
    }

    fn rows_mut(&mut self) -> rayon::slice::ChunksMut<'_, u8> {
        let stride = self.w * 4;
        self.px.par_chunks_mut(stride.max(1))
    }
}

pub fn mul(a: u32, b: u32) -> u8 {
    let t = a * b + 128;
    ((t + (t >> 8)) >> 8) as u8
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub xx: f64,
    pub xy: f64,
    pub x0: f64,
    pub yx: f64,
    pub yy: f64,
    pub y0: f64,
}

impl Affine {
    pub fn into_rect(src_w: f64, src_h: f64, rect: Rect, degrees: f64) -> Affine {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let (kx, ky) = (src_w / rect.w, src_h / rect.h);
        let (cx, cy) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        Affine { xx: kx * cos, xy: kx * sin, x0: kx * (rect.w / 2.0 - cos * cx - sin * cy), yx: -ky * sin, yy: ky * cos, y0: ky * (rect.h / 2.0 + sin * cx - cos * cy) }
    }

    pub fn flipped(self, src_w: f64, src_h: f64, flip_x: bool, flip_y: bool) -> Affine {
        let mut a = self;
        if flip_x {
            a = Affine { xx: -a.xx, xy: -a.xy, x0: src_w - a.x0, ..a };
        }
        if flip_y {
            a = Affine { yx: -a.yx, yy: -a.yy, y0: src_h - a.y0, ..a };
        }
        a
    }

    pub fn source_of(&self, x: f64, y: f64) -> (f64, f64) {
        (self.xx * x + self.xy * y + self.x0, self.yx * x + self.yy * y + self.y0)
    }
}

pub fn edge_coverage(u: f64, size: f64, step: f64) -> f64 {
    ((u.min(size - u)) / step.max(1e-9) + 0.5).clamp(0.0, 1.0)
}

fn texel(src: &Image, x: usize, y: usize) -> [f32; 4] {
    let i = (y * src.w + x) * 4;
    let p = &src.px[i..i + 4];
    [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]
}

fn bilinear(src: &Image, u: f64, v: f64) -> [f32; 4] {
    let fx = (u - 0.5).clamp(0.0, (src.w - 1) as f64);
    let fy = (v - 0.5).clamp(0.0, (src.h - 1) as f64);
    let (x0, y0) = (fx as usize, fy as usize);
    let (x1, y1) = ((x0 + 1).min(src.w - 1), (y0 + 1).min(src.h - 1));
    let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
    let (a, b, c, d) = (texel(src, x0, y0), texel(src, x1, y0), texel(src, x0, y1), texel(src, x1, y1));
    let mut out = [0.0; 4];
    for k in 0..4 {
        let top = a[k] + (b[k] - a[k]) * tx;
        let bottom = c[k] + (d[k] - c[k]) * tx;
        out[k] = top + (bottom - top) * ty;
    }
    out
}

fn blend_over(d: &mut [u8], s: [f32; 4], alpha: f32) {
    let sa = s[3] * alpha;
    if sa <= 0.0 {
        return;
    }
    let keep = 1.0 - sa / 255.0;
    for k in 0..3 {
        d[k] = (s[k] * alpha + d[k] as f32 * keep).round().clamp(0.0, 255.0) as u8;
    }
    d[3] = (sa + d[3] as f32 * keep).round().clamp(0.0, 255.0) as u8;
}

pub fn draw(dst: &mut Image, src: &Image, map: Affine, bounds: Rect, opacity: f64) {
    if src.w == 0 || src.h == 0 || opacity <= 0.0 {
        return;
    }
    let x_from = bounds.x.floor().max(0.0) as usize;
    let x_to = ((bounds.x + bounds.w).ceil().max(0.0) as usize).min(dst.w);
    let y_from = bounds.y.floor().max(0.0) as usize;
    let y_to = ((bounds.y + bounds.h).ceil().max(0.0) as usize).min(dst.h);
    if x_from >= x_to || y_from >= y_to {
        return;
    }
    let (sw, sh) = (src.w as f64, src.h as f64);
    let step_u = map.xx.hypot(map.xy);
    let step_v = map.yx.hypot(map.yy);
    let alpha = opacity.min(1.0) as f32;
    dst.rows_mut().enumerate().skip(y_from).take(y_to - y_from).for_each(|(y, row)| {
        for x in x_from..x_to {
            let (u, v) = map.source_of(x as f64 + 0.5, y as f64 + 0.5);
            let cover = edge_coverage(u, sw, step_u) * edge_coverage(v, sh, step_v);
            if cover <= 0.0 {
                continue;
            }
            blend_over(&mut row[x * 4..x * 4 + 4], bilinear(src, u, v), alpha * cover as f32);
        }
    });
}

pub fn draw_into_rect(dst: &mut Image, src: &Image, rect: Rect, degrees: f64, opacity: f64) {
    let map = Affine::into_rect(src.w as f64, src.h as f64, rect, degrees);
    let bounds = crate::framing::rotated_bounds(rect, degrees);
    draw(dst, src, map, Rect { x: bounds.x - 1.0, y: bounds.y - 1.0, w: bounds.w + 2.0, h: bounds.h + 2.0 }, opacity);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

pub fn region_around(r: Rect, grow: f64, frame_w: usize, frame_h: usize) -> Region {
    let x0 = (r.x - grow).floor().clamp(0.0, frame_w as f64) as usize;
    let y0 = (r.y - grow).floor().clamp(0.0, frame_h as f64) as usize;
    let x1 = (r.x + r.w + grow).ceil().clamp(0.0, frame_w as f64) as usize;
    let y1 = (r.y + r.h + grow).ceil().clamp(0.0, frame_h as f64) as usize;
    Region { x: x0, y: y0, w: x1.saturating_sub(x0), h: y1.saturating_sub(y0) }
}

pub fn over_at(dst: &mut Image, src: &Image, x: usize, y: usize, opacity: f64) {
    let alpha = opacity.clamp(0.0, 1.0) as f32;
    let w = src.w.min(dst.w.saturating_sub(x));
    if w == 0 || src.h == 0 {
        return;
    }
    dst.px.par_chunks_mut(dst.w * 4).skip(y).zip(src.px.par_chunks(src.w * 4)).for_each(|(d, s)| {
        for (dp, sp) in d[x * 4..(x + w) * 4].chunks_exact_mut(4).zip(s.chunks_exact(4)) {
            blend_over(dp, [sp[0] as f32, sp[1] as f32, sp[2] as f32, sp[3] as f32], alpha);
        }
    });
}

pub fn over(dst: &mut Image, src: &Image, opacity: f64) {
    over_at(dst, src, 0, 0, opacity);
}

pub fn shrink(src: &Image, factor: usize) -> Image {
    if factor <= 1 {
        return src.clone();
    }
    let (w, h) = ((src.w / factor).max(1), (src.h / factor).max(1));
    let mut out = Image::new(w, h);
    out.rows_mut().enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let mut sum = [0u32; 4];
            let mut n = 0;
            for sy in y * factor..((y + 1) * factor).min(src.h) {
                for sx in x * factor..((x + 1) * factor).min(src.w) {
                    let i = (sy * src.w + sx) * 4;
                    for k in 0..4 {
                        sum[k] += src.px[i + k] as u32;
                    }
                    n += 1;
                }
            }
            for k in 0..4 {
                row[x * 4 + k] = (sum[k] / n.max(1)) as u8;
            }
        }
    });
    out
}

pub fn box_radii(sigma: f64) -> [usize; 3] {
    let ideal = (12.0 * sigma * sigma / 3.0 + 1.0).sqrt();
    let mut lower = ideal.floor() as i64;
    if lower % 2 == 0 {
        lower -= 1;
    }
    let lower = lower.max(1);
    let upper = lower + 2;
    let lf = lower as f64;
    let m = ((12.0 * sigma * sigma - 3.0 * lf * lf - 12.0 * lf - 9.0) / (-4.0 * lf - 4.0)).round() as i64;
    let size = |i: i64| if i < m { lower } else { upper };
    [0, 1, 2].map(|i| ((size(i) - 1) / 2) as usize)
}

fn box_rows(data: &mut [u8], w: usize, channels: usize, radius: usize) {
    if radius == 0 || w == 0 {
        return;
    }
    let span = (2 * radius + 1) as u32;
    data.par_chunks_mut(w * channels).for_each(|row| {
        let line = row.to_vec();
        let at = |x: i64, k: usize| line[(x.clamp(0, w as i64 - 1) as usize) * channels + k] as u32;
        for k in 0..channels {
            let mut sum: u32 = (-(radius as i64)..=radius as i64).map(|x| at(x, k)).sum();
            for x in 0..w {
                row[x * channels + k] = ((sum + span / 2) / span) as u8;
                sum += at(x as i64 + radius as i64 + 1, k);
                sum -= at(x as i64 - radius as i64, k);
            }
        }
    });
}

fn transpose(data: &[u8], w: usize, h: usize, channels: usize) -> Vec<u8> {
    let mut out = vec![0; data.len()];
    out.par_chunks_mut(h * channels).enumerate().for_each(|(x, col)| {
        for y in 0..h {
            let from = (y * w + x) * channels;
            col[y * channels..y * channels + channels].copy_from_slice(&data[from..from + channels]);
        }
    });
    out
}

pub fn blur(data: &mut Vec<u8>, w: usize, h: usize, channels: usize, sigma: f64) {
    if sigma < 0.3 || w == 0 || h == 0 {
        return;
    }
    let radii = box_radii(sigma);
    for r in radii {
        box_rows(data, w, channels, r);
    }
    let mut turned = transpose(data, w, h, channels);
    for r in radii {
        box_rows(&mut turned, h, channels, r);
    }
    *data = transpose(&turned, h, w, channels);
}

pub fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    let y = 1.0 - poly * (-x * x).exp();
    if x < 0.0 { -y } else { y }
}

pub fn blurred_span(p: f64, from: f64, to: f64, sigma: f64) -> f64 {
    let k = 1.0 / (sigma * std::f64::consts::SQRT_2);
    0.5 * (erf((p - from) * k) - erf((p - to) * k))
}

pub fn rect_shadow(dst: &mut Image, rect: Rect, sigma: f64, opacity: f64) {
    let xs: Vec<f64> = (0..dst.w).map(|x| blurred_span(x as f64 + 0.5, rect.x, rect.x + rect.w, sigma)).collect();
    let ys: Vec<f64> = (0..dst.h).map(|y| blurred_span(y as f64 + 0.5, rect.y, rect.y + rect.h, sigma)).collect();
    dst.rows_mut().enumerate().for_each(|(y, row)| {
        let fy = ys[y] * opacity;
        if fy < 1e-4 {
            return;
        }
        for (x, fx) in xs.iter().enumerate() {
            blend_over(&mut row[x * 4..x * 4 + 4], [0.0, 0.0, 0.0, 255.0], (fx * fy) as f32);
        }
    });
}

pub fn fade_toward(img: &mut Image, rgb: [u8; 3], amount: f64) {
    let t = amount.clamp(0.0, 1.0) as f32;
    if t <= 0.0 {
        return;
    }
    img.px.par_chunks_mut(4).for_each(|p| {
        for k in 0..3 {
            p[k] = (p[k] as f32 + (rgb[k] as f32 - p[k] as f32) * t).round() as u8;
        }
    });
}
