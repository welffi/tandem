use crate::actions::project_name_of;
use crate::keys::nudge_volume;
use crate::player_controls::seek_fraction;
use crate::player_view::fit;
use eframe::egui::{Rect, pos2};
use std::path::Path;

#[test]
fn fit_letterboxes_wide_video() {
    let frame = fit(Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 1000.0)), 1920.0, 1080.0);
    assert_eq!(frame.width(), 1000.0);
    assert!((frame.height() - 562.5).abs() < 1e-3);
    assert!((frame.top() - 218.75).abs() < 1e-3);
}

#[test]
fn fit_pillarboxes_tall_video() {
    let frame = fit(Rect::from_min_max(pos2(0.0, 0.0), pos2(1600.0, 900.0)), 1080.0, 1920.0);
    assert_eq!(frame.height(), 900.0);
    assert!((frame.center().x - 800.0).abs() < 1e-3);
}

#[test]
fn seek_fraction_clamps_to_bar() {
    assert_eq!(seek_fraction(50.0, 100.0, 200.0), 0.0);
    assert_eq!(seek_fraction(200.0, 100.0, 200.0), 0.5);
    assert_eq!(seek_fraction(400.0, 100.0, 200.0), 1.0);
}

#[test]
fn volume_nudges_in_whole_percents() {
    assert_eq!(nudge_volume(0.97, 0.05), 1.0);
    assert_eq!(nudge_volume(0.03, -0.05), 0.0);
    assert_eq!(nudge_volume(0.5, 0.05), 0.55);
}

#[test]
fn project_name_drops_the_project_extension_only() {
    assert_eq!(project_name_of(Path::new("C:/a/My cut.TANDEM")), "My cut");
    assert_eq!(project_name_of(Path::new("C:/a/clip.mp4")), "clip.mp4");
}
