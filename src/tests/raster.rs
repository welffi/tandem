use super::near;
use crate::compose::{decode_size, fade_gain_at};
use crate::decode::{frame_index, reusable};
use crate::edge_fill::{mirror_span, shrink_factor};
use crate::effects::Screen;
use crate::framing::{FrameLook, Rect};
use crate::looks::{despill, grayscale, key_alpha, with_drop_shadow};
use crate::raster::{Affine, Image, blur, blurred_span, box_radii, draw_into_rect, edge_coverage, erf, shrink};
use crate::render_plan::Fades;
use crate::types::{EdgeFill, Framing};

#[test]
fn affine_maps_rect_center_to_picture_center_and_turns_clockwise() {
    let rect = Rect { x: 100.0, y: 50.0, w: 200.0, h: 100.0 };
    let map = Affine::into_rect(400.0, 200.0, rect, 0.0);
    let (u, v) = map.source_of(200.0, 100.0);
    near(u, 200.0);
    near(v, 100.0);
    let (u, v) = map.source_of(100.0, 50.0);
    near(u, 0.0);
    near(v, 0.0);
    let turned = Affine::into_rect(100.0, 100.0, Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, 90.0);
    let (u, v) = turned.source_of(100.0, 0.0);
    near(u, 0.0);
    near(v, 0.0);
    let flipped = map.flipped(400.0, 200.0, true, false);
    near(flipped.source_of(100.0, 50.0).0, 400.0);
}

#[test]
fn edges_fade_over_one_output_pixel() {
    near(edge_coverage(50.0, 100.0, 1.0), 1.0);
    near(edge_coverage(0.0, 100.0, 1.0), 0.5);
    near(edge_coverage(-0.5, 100.0, 1.0), 0.0);
    near(edge_coverage(1.0, 100.0, 4.0), 0.75);
}

#[test]
fn drawing_fills_the_rect_and_leaves_the_rest() {
    let src = Image::filled(4, 4, [255, 0, 0, 255]);
    let mut dst = Image::filled(10, 10, [0, 0, 0, 255]);
    draw_into_rect(&mut dst, &src, Rect { x: 2.0, y: 2.0, w: 4.0, h: 4.0 }, 0.0, 1.0);
    let at = |x: usize, y: usize| dst.px[(y * 10 + x) * 4];
    assert_eq!((at(3, 3), at(2, 5), at(1, 1), at(6, 3)), (255, 255, 0, 0));
}

#[test]
fn box_blur_matches_gaussian_spread_and_keeps_flat_color() {
    let radii = box_radii(10.0);
    let variance: f64 = radii.iter().map(|r| ((2 * r + 1).pow(2) - 1) as f64 / 12.0).sum();
    assert!((variance.sqrt() - 10.0).abs() < 0.6, "{radii:?}");
    let mut flat = vec![77u8; 20 * 10];
    blur(&mut flat, 20, 10, 1, 3.0);
    assert!(flat.iter().all(|v| *v == 77));
    let mut dot = vec![0u8; 21 * 21];
    dot[10 * 21 + 10] = 255;
    blur(&mut dot, 21, 21, 1, 1.5);
    assert!(dot[10 * 21 + 10] < 255 && dot[10 * 21 + 12] > 0 && dot[10 * 21 + 10] >= dot[10 * 21 + 12]);
}

#[test]
fn soft_shadow_edges_follow_the_normal_curve() {
    near(erf(0.0), 0.0);
    near(erf(1.0), 0.8427);
    near(erf(-2.0), -0.9953);
    near(blurred_span(50.0, 0.0, 100.0, 5.0), 1.0);
    near(blurred_span(0.0, 0.0, 100.0, 5.0), 0.5);
    near(blurred_span(-50.0, 0.0, 100.0, 5.0), 0.0);
}

#[test]
fn shrinking_averages_blocks() {
    let mut img = Image::filled(4, 2, [0, 0, 0, 255]);
    img.px[0] = 200;
    img.px[4] = 100;
    let small = shrink(&img, 2);
    assert_eq!((small.w, small.h, small.px[0], small.px[3]), (2, 1, 75, 255));
}

#[test]
fn grayscale_moves_toward_luma() {
    let mut img = Image::filled(1, 1, [255, 0, 0, 255]);
    grayscale(&mut img, 1.0);
    assert_eq!(&img.px[..3], &[76, 76, 76]);
}

#[test]
fn chroma_key_alpha_and_spill() {
    assert_eq!(key_alpha(0.1, 0.12, 0.0), 0.0);
    assert_eq!(key_alpha(0.2, 0.12, 0.0), 1.0);
    assert!((key_alpha(0.15, 0.1, 0.1) - 0.5).abs() < 1e-6);
    assert_eq!(despill([0.2, 0.9, 0.4], Some(Screen::Green), 1.0), [0.2, 0.3, 0.4]);
    assert_eq!(despill([0.2, 0.1, 0.4], Some(Screen::Green), 1.0), [0.2, 0.1, 0.4]);
    assert_eq!(despill([0.2, 0.4, 0.9], None, 1.0), [0.2, 0.4, 0.9]);
}

#[test]
fn drop_shadow_sits_under_the_layer_at_its_offset() {
    let mut layer = Image::new(10, 10);
    layer.px[(2 * 10 + 2) * 4..(2 * 10 + 2) * 4 + 4].copy_from_slice(&[255, 255, 255, 255]);
    let shadow = crate::effects::DropShadow { dx: 3.0, dy: 1.0, blur: 0.0, rgb: [0, 0, 0], opacity: 1.0 };
    let out = with_drop_shadow(&layer, &shadow);
    assert_eq!(&out.px[(2 * 10 + 2) * 4..(2 * 10 + 2) * 4 + 4], &[255, 255, 255, 255]);
    assert_eq!(&out.px[(3 * 10 + 5) * 4..(3 * 10 + 5) * 4 + 4], &[0, 0, 0, 255]);
}

#[test]
fn fades_ramp_in_and_out_over_the_clip() {
    let fades = Fades { in_sec: 1.0, out_sec: 2.0 };
    near(fade_gain_at(fades, 0.5, 10.0), 0.5);
    near(fade_gain_at(fades, 5.0, 10.0), 1.0);
    near(fade_gain_at(fades, 9.0, 10.0), 0.5);
    near(fade_gain_at(Fades::default(), 0.0, 10.0), 1.0);
}

#[test]
fn decoders_are_reused_only_for_short_forward_steps() {
    assert_eq!(frame_index(10.0, 30.0, 10.0), 0);
    assert_eq!(frame_index(10.0, 30.0, 11.0), 30);
    assert_eq!(frame_index(10.0, 30.0, 9.99), -1);
    assert!(reusable(None, 0, 30.0));
    assert!(reusable(Some(5), 40, 30.0));
    assert!(!reusable(Some(5), 4, 30.0));
    assert!(!reusable(Some(5), 200, 30.0));
}

#[test]
fn edge_fill_helpers() {
    assert_eq!((shrink_factor(2.0), shrink_factor(17.0), shrink_factor(400.0)), (1, 4, 8));
    assert_eq!((mirror_span(-10.0, 100.0), mirror_span(0.0, 100.0), mirror_span(150.0, 100.0)), (0, 0, 2));
}

#[test]
fn decode_size_fits_the_frame_but_never_grows() {
    let look = FrameLook { framing: Framing::Fit, scale: 1.0, edge_fill: EdgeFill::Black, edge_blur: 40.0, shadow: 0.0, shadow_size: 40.0 };
    assert_eq!(decode_size(960, 540, 1920, 1080, &look), (960, 540));
    assert_eq!(decode_size(1920, 1080, 640, 480, &look), (640, 480));
}
