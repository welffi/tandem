use crate::framing::{FrameLook, Rect, blur_sigma, has_shadow, shadow_drop, shadow_sigma};
use crate::raster::{Affine, Image, blur, draw, rect_shadow, shrink};
use crate::types::EdgeFill;

pub fn shrink_factor(sigma: f64) -> usize {
    ((sigma / 4.0).floor() as usize).clamp(1, 8)
}

pub fn mirror_span(start: f64, size: f64) -> i64 {
    if size <= 0.0 { 0 } else { (start / size).ceil().max(0.0) as i64 }
}

fn fitted(pic: &Image, target_w: f64) -> Image {
    shrink(pic, (pic.w as f64 / target_w.max(1.0)).floor().max(1.0) as usize)
}

fn draw_tile(canvas: &mut Image, pic: &Image, tile: Rect, flip_x: bool, flip_y: bool) {
    let map = Affine::into_rect(pic.w as f64, pic.h as f64, tile, 0.0).flipped(pic.w as f64, pic.h as f64, flip_x, flip_y);
    draw(canvas, pic, map, Rect { x: tile.x - 1.0, y: tile.y - 1.0, w: tile.w + 2.0, h: tile.h + 2.0 }, 1.0);
}

fn mirror(canvas: &mut Image, pic: &Image, video: Rect) {
    let pic = fitted(pic, video.w);
    let (nx, ny) = (mirror_span(video.x, video.w), mirror_span(video.y, video.h));
    let (cw, ch) = (canvas.w as f64, canvas.h as f64);
    for i in -nx..=nx {
        for j in -ny..=ny {
            let tile = Rect { x: video.x + i as f64 * video.w, y: video.y + j as f64 * video.h, ..video };
            if tile.x > cw || tile.y > ch || tile.x + tile.w < 0.0 || tile.y + tile.h < 0.0 {
                continue;
            }
            draw_tile(canvas, &pic, tile, i % 2 != 0, j % 2 != 0);
        }
    }
}

fn cover(canvas: &mut Image, pic: &Image) {
    let (cw, ch) = (canvas.w as f64, canvas.h as f64);
    let s = (cw / pic.w as f64).max(ch / pic.h as f64);
    let (w, h) = (pic.w as f64 * s, pic.h as f64 * s);
    let pic = fitted(pic, w);
    draw_tile(canvas, &pic, Rect { x: (cw - w) / 2.0, y: (ch - h) / 2.0, w, h }, false, false);
}

pub fn back_fill(w: usize, h: usize, pic: &Image, video: Rect, look: &FrameLook) -> Image {
    let mut frame = Image::filled(w, h, [0, 0, 0, 255]);
    if pic.w > 0 && pic.h > 0 {
        let sigma = blur_sigma(h as f64, look.edge_blur);
        let down = shrink_factor(sigma) as f64;
        let margin = if sigma > 0.0 { (sigma * 3.0 / down).ceil() } else { 0.0 };
        let (sw, sh) = ((w as f64 / down).ceil() + 2.0 * margin, (h as f64 / down).ceil() + 2.0 * margin);
        let mut small = Image::filled(sw as usize, sh as usize, [0, 0, 0, 255]);
        if look.edge_fill == EdgeFill::Mirror {
            let tile = Rect { x: video.x / down + margin, y: video.y / down + margin, w: video.w / down, h: video.h / down };
            mirror(&mut small, pic, tile);
        } else {
            cover(&mut small, pic);
        }
        blur(&mut small.px, small.w, small.h, 4, sigma / down);
        let map = Affine { xx: 1.0 / down, xy: 0.0, x0: margin, yx: 0.0, yy: 1.0 / down, y0: margin };
        draw(&mut frame, &small, map, Rect { x: 0.0, y: 0.0, w: w as f64, h: h as f64 }, 1.0);
    }
    if has_shadow(look) {
        let sigma = shadow_sigma(h as f64, look.shadow_size);
        rect_shadow(&mut frame, Rect { y: video.y + shadow_drop(sigma), ..video }, sigma, look.shadow / 100.0);
    }
    frame
}
