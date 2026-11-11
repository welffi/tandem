use crate::app::App;
use crate::inspector_clip;
use crate::inspector_project;
use crate::theme::*;
use crate::types::Clip;
use crate::widgets::{Pill, pill};
use eframe::egui::{Align, Layout, Rect, ScrollArea, Stroke, Ui, UiBuilder, pos2, vec2};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Tab {
    #[default]
    Project,
    Clip,
}

#[derive(Default)]
pub struct InspectorState {
    tab: Tab,
    seen: Vec<String>,
}

const HEADER_H: f32 = 36.0;

fn selected_clips(app: &App) -> Vec<Clip> {
    app.store.project().tracks.iter().flat_map(|t| &t.clips).filter(|c| app.store.selection.contains(&c.id)).cloned().collect()
}

pub fn clip_title(clips: &[Clip]) -> String {
    match clips {
        _ if clips.iter().any(|c| c.text.is_some()) => "Text".into(),
        [one] if one.adjustment => "Adjustment".into(),
        [_] | [] => "Clip".into(),
        many => format!("{} clips", many.len()),
    }
}

fn follow_selection(app: &mut App) {
    if app.inspector.seen != app.store.selection {
        app.inspector.seen = app.store.selection.clone();
        app.inspector.tab = if app.store.selection.is_empty() { Tab::Project } else { Tab::Clip };
    }
}

fn header(ui: &mut Ui, app: &mut App, rect: Rect, clips: &[Clip]) {
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, BORDER));
    let mut bar = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0))).layout(Layout::left_to_right(Align::Center)));
    let has_clips = !clips.is_empty();
    for (tab, label) in [(Tab::Project, "Project".to_string()), (Tab::Clip, clip_title(clips))] {
        let enabled = tab == Tab::Project || has_clips;
        let on = enabled && app.inspector.tab == tab;
        if bar.add_enabled_ui(enabled, |ui| pill(ui, &label, if on { Pill::On } else { Pill::Plain })).inner.clicked() {
            app.inspector.tab = tab;
        }
    }
}

pub fn show(ui: &mut Ui, app: &mut App, rect: Rect) {
    follow_selection(app);
    ui.painter().rect_filled(rect, 0.0, PANEL);
    ui.painter().vline(rect.left() + 0.5, rect.y_range(), Stroke::new(1.0, BORDER));
    let clips = selected_clips(app);
    header(ui, app, Rect::from_min_size(rect.min, vec2(rect.width(), HEADER_H)), &clips);
    let body = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + HEADER_H + 4.0), pos2(rect.right() - 12.0, rect.bottom() - 4.0));
    let mut inner = ui.new_child(UiBuilder::new().max_rect(body));
    ScrollArea::vertical().id_salt("inspector").auto_shrink([false, false]).show(&mut inner, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        if app.inspector.tab == Tab::Clip && !clips.is_empty() {
            inspector_clip::show(ui, app, &clips);
        } else {
            inspector_project::show(ui, app);
        }
    });
}
