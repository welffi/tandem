use crate::library_edit::{add_folder, add_item, child_count, delete_folder, delete_item, folder_path, item_meta, move_to, rename, replace_item, set_color};
use crate::types::{Library, Preset, PresetTrack, TrackKind};

fn item(id: &str) -> Preset {
    Preset { id: id.into(), name: id.into(), created_at: 1.0, duration_sec: 65.0, assets: Vec::new(), tracks: Vec::new(), folder_id: None, color: None }
}

fn nested() -> Library {
    let library = add_folder(&Library::default(), "a".into(), None);
    let library = add_folder(&library, "b".into(), Some("a".into()));
    add_item(&library, item("x"), Some("b".into()))
}

#[test]
fn new_items_and_folders_land_in_the_open_folder() {
    let library = nested();
    assert_eq!(library.folders[1].parent_id.as_deref(), Some("a"));
    assert_eq!(library.items[0].folder_id.as_deref(), Some("b"));
}

#[test]
fn rename_trims_and_ignores_blank_names() {
    let library = rename(&nested(), "x", "  Intro  ");
    assert_eq!(library.items[0].name, "Intro");
    assert_eq!(rename(&library, "x", "   "), library);
    assert_eq!(rename(&library, "a", "Titles").folders[0].name, "Titles");
}

#[test]
fn deleting_a_folder_moves_its_contents_up_one_level() {
    let library = delete_folder(&nested(), "b");
    assert_eq!(library.items[0].folder_id.as_deref(), Some("a"));
    let library = delete_folder(&library, "a");
    assert_eq!(library.items[0].folder_id, None);
    assert!(library.folders.is_empty());
}

#[test]
fn a_folder_cannot_move_into_itself_or_its_own_child() {
    let library = nested();
    assert_eq!(move_to(&library, "a", Some("b")), library);
    assert_eq!(move_to(&library, "a", Some("a")), library);
    let moved = move_to(&library, "x", None);
    assert_eq!(moved.items[0].folder_id, None);
    assert_eq!(move_to(&library, "b", None).folders[1].parent_id, None);
}

#[test]
fn the_path_runs_from_the_top_and_is_empty_for_a_lost_folder() {
    let names: Vec<String> = folder_path(&nested(), Some("b")).into_iter().map(|f| f.name).collect();
    assert_eq!(names.len(), 2);
    assert!(folder_path(&nested(), Some("gone")).is_empty());
    assert!(folder_path(&nested(), None).is_empty());
}

#[test]
fn folders_count_their_folders_and_items() {
    assert_eq!(child_count(&nested(), "a"), 1);
    assert_eq!(child_count(&nested(), "b"), 1);
}

#[test]
fn replacing_an_item_keeps_its_name_place_and_colour() {
    let library = set_color(&nested(), "x", Some("#e05d5d".into()));
    let fresh = Preset { duration_sec: 9.0, name: "other".into(), ..item("fresh") };
    let next = replace_item(&library, "x", fresh);
    let kept = &next.items[0];
    assert_eq!((kept.id.as_str(), kept.name.as_str(), kept.duration_sec), ("x", "x", 9.0));
    assert_eq!((kept.folder_id.as_deref(), kept.color.as_deref()), (Some("b"), Some("#e05d5d")));
    assert!(delete_item(&next, "x").items.is_empty());
}

#[test]
fn item_meta_counts_layers_and_clips() {
    let track = PresetTrack { kind: TrackKind::Video, name: "V".into(), muted: false, hidden: false, volume: 1.0, clips: vec![super::clip("c", 0.0, 0.0, 1.0)] };
    let one = Preset { tracks: vec![track.clone()], ..item("m") };
    assert_eq!(item_meta(&one), "1 layer · 1 clip · 1:05");
    let two = Preset { tracks: vec![track.clone(), track], ..item("m") };
    assert!(item_meta(&two).starts_with("2 layers · 2 clips"));
}

#[test]
fn tints_parse_six_digit_hex_only() {
    use crate::theme::{TINTS, tint_color};
    use eframe::egui::Color32;
    assert_eq!(tint_color("#e05d5d"), Some(Color32::from_rgb(0xe0, 0x5d, 0x5d)));
    assert_eq!(tint_color("e05d5d"), None);
    assert_eq!(tint_color("#e05d"), None);
    assert_eq!(tint_color("#+05d5d"), None);
    assert!(TINTS.iter().all(|t| tint_color(t).is_some()));
}
