use super::{clip, effect, media, near, track};
use crate::edits::empty_project;
use crate::preview_edit::{dragged_text, moved_effect, scaled_effect, snap_center, text_grab, text_under, transform_grab, transform_target, with_effect, with_text_position};
use crate::preview_hit::{Spot, spot_at};
use crate::text_render::TextBounds;
use crate::types::{AnimField, Clip, EffectKind, Project, TextAlign, TextStyle, TrackKind};
use eframe::egui::{Rect, pos2};

fn text_style() -> TextStyle {
    TextStyle {
        content: "a".into(),
        font: "Arial".into(),
        size: 10.0,
        color: "#ffffff".into(),
        bold: false,
        italic: false,
        align: TextAlign::Center,
        x: 0.3,
        y: 0.6,
        outline: false,
        boxed: false,
        keys: None,
    }
}

fn video_project(clip: Clip) -> Project {
    let mut asset = media("a");
    asset.width = Some(1920);
    asset.height = Some(1080);
    let mut project = empty_project();
    project.width = 1920;
    project.height = 1080;
    project.assets = vec![asset];
    project.tracks = vec![track(TrackKind::Video, vec![clip])];
    project
}

fn text_project() -> Project {
    let mut c = clip("t1", 0.0, 0.0, 5.0);
    c.text = Some(text_style());
    let mut project = empty_project();
    project.tracks = vec![track(TrackKind::Text, vec![c])];
    project
}

#[test]
fn center_snaps_only_when_close_and_allowed() {
    near(snap_center(0.505, true), 0.5);
    near(snap_center(0.52, true), 0.52);
    near(snap_center(0.505, false), 0.505);
}

#[test]
fn the_topmost_text_under_the_pointer_wins() {
    let texts = vec![("low".to_string(), TextBounds { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }), ("high".to_string(), TextBounds { x: 50.0, y: 50.0, w: 100.0, h: 100.0 })];
    assert_eq!(text_under(&texts, (200, 200), (0.4, 0.4)), Some("high"));
    assert_eq!(text_under(&texts, (200, 200), (0.1, 0.1)), Some("low"));
    assert_eq!(text_under(&texts, (200, 200), (0.9, 0.9)), None);
    assert_eq!(text_under(&texts, (0, 0), (0.1, 0.1)), None);
}

#[test]
fn dragging_text_stays_in_the_frame_and_snaps_to_center() {
    let project = text_project();
    let grab = text_grab(&project, "t1", 1.0).unwrap();
    assert_eq!(grab.start, (0.3, 0.6));
    assert_eq!(dragged_text(&grab, (0.205, -0.1), true), (0.5, 0.5));
    assert_eq!(dragged_text(&grab, (0.205, -0.1), false), (0.505, 0.5));
    assert_eq!(dragged_text(&grab, (2.0, -2.0), true), (1.0, 0.0));
}

#[test]
fn moving_text_writes_both_axes() {
    let project = text_project();
    let grab = text_grab(&project, "t1", 1.0).unwrap();
    let moved = with_text_position(&project, &grab, (0.7, 0.2));
    let text = moved.tracks[0].clips[0].text.as_ref().unwrap();
    assert_eq!((text.x, text.y), (0.7, 0.2));
}

#[test]
fn the_transform_box_follows_a_selected_video_clip() {
    let project = video_project(clip("c", 0.0, 0.0, 5.0));
    assert!(transform_target(&project, &[], 1.0).is_none());
    assert!(transform_target(&project, &["c".to_string()], 6.0).is_none());
    let target = transform_target(&project, &["c".to_string()], 1.0).unwrap();
    near(target.rect.w, 1.0);
    near(target.rect.x, 0.0);
    let mut hidden = project.clone();
    hidden.tracks[0].hidden = true;
    assert!(transform_target(&hidden, &["c".to_string()], 1.0).is_none());
}

#[test]
fn moving_a_clip_adds_a_transform_and_keeps_it_on_the_next_move() {
    let project = video_project(clip("c", 0.0, 0.0, 5.0));
    let grab = transform_grab(&project, "c", 1.0).unwrap();
    let moved = with_effect(&project, "c", &moved_effect(&grab, (0.2, -0.1), false));
    let effects = moved.tracks[0].clips[0].effects();
    assert_eq!(effects.len(), 1);
    near(effects[0].x.unwrap(), 0.2);
    near(effects[0].y.unwrap(), -0.1);
    let again = transform_grab(&moved, "c", 1.0).unwrap();
    near(again.total.x, 0.2);
    let moved_more = with_effect(&moved, "c", &moved_effect(&again, (0.1, 0.0), false));
    assert_eq!(moved_more.tracks[0].clips[0].effects().len(), 1);
    near(moved_more.tracks[0].clips[0].effects()[0].x.unwrap(), 0.3);
}

#[test]
fn moving_a_clip_snaps_its_center() {
    let project = video_project(clip("c", 0.0, 0.0, 5.0));
    let grab = transform_grab(&project, "c", 1.0).unwrap();
    near(moved_effect(&grab, (0.01, -0.01), true).x.unwrap(), 0.0);
    near(moved_effect(&grab, (0.01, -0.01), false).x.unwrap(), 0.01);
}

#[test]
fn scaling_is_limited_and_keeps_height_when_it_is_set() {
    let mut c = clip("c", 0.0, 0.0, 5.0);
    let mut fx = effect(EffectKind::Transform);
    fx.size = Some(1.0);
    fx.height = Some(2.0);
    c.effects = Some(vec![fx]);
    let grab = transform_grab(&video_project(c), "c", 1.0).unwrap();
    let doubled = scaled_effect(&grab, 2.0);
    near(doubled.size.unwrap(), 2.0);
    near(doubled.height.unwrap(), 4.0);
    near(scaled_effect(&grab, 100.0).size.unwrap(), 4.0);
    near(scaled_effect(&grab, 0.0001).size.unwrap(), 0.1);
    let plain = transform_grab(&video_project(clip("c", 0.0, 0.0, 5.0)), "c", 1.0).unwrap();
    assert!(scaled_effect(&plain, 2.0).height.is_none());
}

#[test]
fn moving_an_animated_clip_keys_the_current_time() {
    let mut c = clip("c", 0.0, 0.0, 5.0);
    let mut fx = effect(EffectKind::Transform);
    fx.x = Some(0.0);
    c.effects = Some(vec![fx]);
    let project = video_project(c);
    let first = transform_grab(&project, "c", 1.0).unwrap();
    let keyed = with_effect(&project, "c", &crate::keyframes::set_key(&first.original, AnimField::X, 1.0, 0.0));
    let grab = transform_grab(&keyed, "c", 2.0).unwrap();
    let moved = with_effect(&keyed, "c", &moved_effect(&grab, (0.1, 0.0), false));
    let keys = moved.tracks[0].clips[0].effects()[0].keys.as_ref().unwrap().get(&AnimField::X).unwrap();
    assert_eq!(keys.len(), 2);
}

#[test]
fn box_hits_follow_the_rotation() {
    let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(300.0, 200.0));
    assert_eq!(spot_at(rect, 0.0, pos2(200.0, 150.0)), Some(Spot::Inside));
    assert_eq!(spot_at(rect, 0.0, pos2(102.0, 98.0)), Some(Spot::Corner));
    assert_eq!(spot_at(rect, 0.0, pos2(400.0, 400.0)), None);
    assert_eq!(spot_at(rect, 90.0, pos2(200.0, 230.0)), Some(Spot::Inside));
    assert_eq!(spot_at(rect, 90.0, pos2(290.0, 150.0)), None);
    assert_eq!(spot_at(rect, 90.0, pos2(250.0, 50.0)), Some(Spot::Corner));
}
