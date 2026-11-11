use crate::edits::{add_track, empty_project};
use crate::store::{Mode, Store};
use crate::types::{Project, TrackKind};

fn renamed(store: &Store, name: &str) -> Project {
    let mut next = store.project().clone();
    next.name = name.into();
    next
}

#[test]
fn edits_with_the_same_key_share_one_undo_step() {
    let mut store = Store::new();
    store.edit(renamed(&store, "a"), Some("rename"));
    store.edit(renamed(&store, "ab"), Some("rename"));
    store.edit(renamed(&store, "abc"), None);
    store.undo();
    assert_eq!(store.project().name, "ab");
    store.undo();
    assert_eq!(store.project().name, "Untitled");
    assert!(!store.can_undo());
    store.redo();
    assert_eq!(store.project().name, "ab");
    assert!(store.dirty);
}

#[test]
fn an_unchanged_edit_adds_no_history() {
    let mut store = Store::new();
    let same = store.project().clone();
    store.edit(same, None);
    assert!(!store.can_undo());
    assert!(!store.dirty);
}

#[test]
fn undo_keeps_the_live_mix() {
    let mut store = Store::new();
    store.load_project(empty_project(), None);
    let track = store.project().tracks.iter().find(|t| t.kind == TrackKind::Audio).unwrap().id.clone();
    store.edit(add_track(store.project(), TrackKind::Text, None), None);
    store.set_mix(&track, Some(true), Some(0.3));
    store.undo();
    let after = store.project().tracks.iter().find(|t| t.id == track).unwrap();
    assert!(after.muted);
    assert_eq!(after.volume, 0.3);
}

#[test]
fn mix_changes_mark_dirty_only_for_saved_editor_projects() {
    let mut store = Store::new();
    store.load_project(empty_project(), None);
    let track = store.project().tracks[0].id.clone();
    store.set_mix(&track, Some(true), None);
    assert!(!store.dirty);
    store.mode = Mode::Editor;
    store.mark_saved("x.tandem".into());
    store.set_mix(&track, Some(false), None);
    assert!(store.dirty);
}

#[test]
fn title_shows_name_and_unsaved_dot() {
    let mut store = Store::new();
    assert_eq!(store.title(), "Tandem");
    store.mode = Mode::Editor;
    store.edit(renamed(&store, "Trip"), None);
    assert_eq!(store.title(), "• Trip - Tandem");
}
