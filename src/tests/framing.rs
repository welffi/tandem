use super::effect;
use crate::edits::empty_project;
use crate::effects::{IDENTITY, Transform, transform_of};
use crate::framing::{FrameLook, Rect, covers_frame, frame_look, rotated_bounds, video_rect};
use crate::types::{EdgeFill, Effect, EffectKind, Framing, Project};

fn look(framing: Framing, scale: f64) -> FrameLook {
    FrameLook { framing, scale, edge_fill: EdgeFill::Mirror, edge_blur: 0.0, shadow: 0.0, shadow_size: 40.0 }
}

fn transform(size: f64, x: f64, y: f64, height: Option<f64>) -> Effect {
    Effect { size: Some(size), height, x: Some(x), y: Some(y), ..effect(EffectKind::Transform) }
}

#[test]
fn fit_letterboxes_a_wide_video_in_a_tall_frame() {
    let rect = video_rect(1080.0, 1920.0, 1920.0, 1080.0, &look(Framing::Fit, 1.0), &IDENTITY);
    assert_eq!(rect, Rect { x: 0.0, y: 656.25, w: 1080.0, h: 607.5 });
    assert!(!covers_frame(&rect, 1080.0, 1920.0));
}

#[test]
fn fill_covers_and_scale_grows_around_the_center() {
    let rect = video_rect(1080.0, 1920.0, 1920.0, 1080.0, &look(Framing::Fill, 1.5), &IDENTITY);
    assert_eq!(rect.h, 2880.0);
    assert_eq!(rect.x + rect.w / 2.0, 540.0);
    assert!(covers_frame(&rect, 1080.0, 1920.0));
}

#[test]
fn scale_below_one_shrinks_a_matching_video() {
    let rect = video_rect(1920.0, 1080.0, 1920.0, 1080.0, &look(Framing::Fit, 0.5), &IDENTITY);
    assert_eq!(rect, Rect { x: 480.0, y: 270.0, w: 960.0, h: 540.0 });
}

#[test]
fn frame_look_fills_defaults_and_clamps() {
    let project = empty_project();
    assert_eq!(frame_look(&project), FrameLook { framing: Framing::Fit, scale: 1.0, edge_fill: EdgeFill::Black, edge_blur: 40.0, shadow: 0.0, shadow_size: 40.0 });
    assert_eq!(frame_look(&Project { video_scale: Some(50.0), ..project.clone() }).scale, 4.0);
    assert_eq!(frame_look(&Project { edge_blur: Some(-3.0), ..project }).edge_blur, 0.0);
}

#[test]
fn transforms_stack_size_and_offset_the_rect() {
    let tf = transform_of(&[transform(0.5, 0.1, 0.0, None), transform(2.0, 0.0, -0.25, Some(1.0)), effect(EffectKind::Grayscale)]);
    assert_eq!(tf, Transform { sx: 1.0, sy: 0.5, x: 0.1, y: -0.25, rot: 0.0 });
    assert_eq!(video_rect(1000.0, 1000.0, 1000.0, 1000.0, &look(Framing::Fit, 1.0), &tf), Rect { x: 100.0, y: 0.0, w: 1000.0, h: 500.0 });
}

#[test]
fn rotation_adds_up() {
    let fx = |rotation| Effect { rotation: Some(rotation), ..effect(EffectKind::Transform) };
    assert_eq!(transform_of(&[fx(30.0), fx(15.0)]).rot, 45.0);
}

#[test]
fn rotated_bounds_swap_at_90_and_grow_at_45() {
    let rect = Rect { x: 100.0, y: 200.0, w: 400.0, h: 200.0 };
    let quarter = rotated_bounds(rect, 90.0);
    assert_eq!((quarter.x.round(), quarter.y.round(), quarter.w.round(), quarter.h.round()), (200.0, 100.0, 200.0, 400.0));
    let diagonal = rotated_bounds(rect, 45.0);
    assert!((diagonal.w - 600.0 / std::f64::consts::SQRT_2).abs() < 1e-9);
    assert!((diagonal.x + diagonal.w / 2.0 - 300.0).abs() < 1e-9);
}
