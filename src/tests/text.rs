use super::near;
use crate::fonts::{face_name, pick_face};
use crate::text_render::{TextBounds, block_bounds, dilate, line_left, rounded_box_cover};
use crate::types::{TextAlign, TextStyle};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn registry_names_become_face_keys() {
    assert_eq!(face_name("Arial Bold Italic (TrueType)"), "arial bold italic");
    assert_eq!(face_name("Segoe UI (TrueType)"), "segoe ui");
}

#[test]
fn missing_styles_fall_back_to_plainer_faces() {
    let faces: HashMap<String, PathBuf> =
        [("impact", "impact.ttf"), ("segoe ui", "segoeui.ttf"), ("segoe ui bold", "segoeuib.ttf")].into_iter().map(|(k, v)| (k.to_string(), PathBuf::from(v))).collect();
    assert_eq!(pick_face(&faces, "Impact", true, true).unwrap(), &PathBuf::from("impact.ttf"));
    assert_eq!(pick_face(&faces, "Nope", true, false).unwrap(), &PathBuf::from("segoeuib.ttf"));
    assert_eq!(pick_face(&faces, "Nope", false, true).unwrap(), &PathBuf::from("segoeui.ttf"));
}

fn style(align: TextAlign) -> TextStyle {
    TextStyle { content: "a".into(), font: "Arial".into(), size: 10.0, color: "#ffffff".into(), bold: false, italic: false, align, x: 0.5, y: 0.5, outline: false, boxed: false, keys: None }
}

#[test]
fn text_blocks_center_on_their_point_with_padding() {
    let (left, top, b) = block_bounds(&style(TextAlign::Center), 1000.0, 500.0, 200.0, 2);
    near(left, 400.0);
    near(top, 250.0 - 60.0);
    near(b.x, 400.0 - 17.5);
    near(b.h, 120.0 + 15.0);
    near(line_left(TextAlign::Right, 400.0, 200.0, 50.0), 550.0);
    near(line_left(TextAlign::Center, 400.0, 200.0, 50.0), 475.0);
}

#[test]
fn rounded_box_is_solid_inside_and_clear_in_the_corner() {
    let b = TextBounds { x: 0.0, y: 0.0, w: 100.0, h: 40.0 };
    near(rounded_box_cover(50.0, 20.0, &b, 10.0), 1.0);
    near(rounded_box_cover(0.5, 0.5, &b, 10.0), 0.0);
    near(rounded_box_cover(50.0, 0.0, &b, 10.0), 0.5);
}

#[test]
fn outline_grows_the_fill_by_its_radius() {
    let mut mask = vec![0f32; 11 * 11];
    mask[5 * 11 + 5] = 1.0;
    let grown = dilate(&mask, 11, 11, 2.0);
    near(grown[5 * 11 + 6] as f64, 1.0);
    near(grown[5 * 11 + 7] as f64, 0.5);
    near(grown[5 * 11 + 8] as f64, 0.0);
    assert!(grown[0] == 0.0);
}
