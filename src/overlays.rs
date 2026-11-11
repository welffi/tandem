use crate::app::App;
use crate::ffmpeg_install::InstallState;
use crate::store::{Mode, Store};
use crate::theme::{ACCENT, ACCENT_2, BG, BORDER, DANGER, DIM, DROP_WASH, PANEL_2, TEXT, semibold};
use crate::widgets::{Look, button};
use eframe::egui::{Align, Align2, Area, CornerRadius, FontId, Frame, Id, LayerId, Layout, Margin, Order, Pos2, ProgressBar, Rect, Sense, Shape, Stroke, Ui, UiBuilder, vec2};

const TOAST_WIDTH: f32 = 420.0;
const TOAST_FOR: std::time::Duration = std::time::Duration::from_secs(6);

pub fn install_screen(ui: &mut Ui, app: &mut App, state: &InstallState) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BG);
    let block = Rect::from_center_size(rect.center(), vec2(360.0, 140.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(block).layout(Layout::top_down(Align::Center)));
    child.spacing_mut().item_spacing.y = 12.0;
    child.heading("Getting ffmpeg ready");
    match state {
        InstallState::Downloading(progress) => {
            child.colored_label(DIM, "Tandem uses ffmpeg to play and export video. It downloads once.");
            let bar = ProgressBar::new(progress.unwrap_or(0.0)).desired_width(360.0).fill(ACCENT);
            child.add(if progress.is_some() { bar.show_percentage() } else { bar.animate(true) });
        }
        InstallState::Unpacking => {
            child.colored_label(DIM, "Unpacking…");
            child.add(ProgressBar::new(1.0).desired_width(360.0).fill(ACCENT).animate(true));
        }
        InstallState::Failed(message) => {
            child.colored_label(DANGER, message);
            if button(&mut child, "Retry", Look::Primary, true).clicked() {
                let ctx = child.ctx().clone();
                app.retry_install(&ctx);
            }
        }
        InstallState::Done => {}
    }
}

pub fn drop_hint(ui: &Ui, mode: Mode) {
    if ui.ctx().input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let rect = ui.ctx().content_rect();
    let painter = ui.ctx().layer_painter(LayerId::new(Order::Foreground, Id::unique("drop-hint")));
    painter.rect_filled(rect, 0.0, DROP_WASH);
    let edge = rect.shrink(1.5);
    let corners: Vec<Pos2> = vec![edge.left_top(), edge.right_top(), edge.right_bottom(), edge.left_bottom(), edge.left_top()];
    painter.extend(Shape::dashed_line(&corners, Stroke::new(3.0, ACCENT), 9.0, 6.0));
    let text = if mode == Mode::Player { "Drop to play" } else { "Drop to import" };
    painter.text(rect.center(), Align2::CENTER_CENTER, text, FontId::new(22.0, semibold()), TEXT);
}

pub fn toast(ui: &mut Ui, store: &mut Store) {
    let Some(text) = store.live_toast().map(String::from) else { return };
    let ctx = ui.ctx().clone();
    if let Some(toast) = &store.toast {
        ctx.request_repaint_after(TOAST_FOR.saturating_sub(toast.at.elapsed()));
    }
    let frame = Frame::NONE.fill(PANEL_2).stroke(Stroke::new(1.0, BORDER)).corner_radius(CornerRadius::same(6)).inner_margin(Margin { left: 17, right: 14, top: 10, bottom: 10 });
    let shown = Area::new(Id::unique("toast")).order(Order::Foreground).anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -16.0)).show(&ctx, |ui| {
        let response = frame.show(ui, |ui| {
            ui.set_max_width(TOAST_WIDTH - 31.0);
            ui.label(text);
        });
        let rect = response.response.rect;
        ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(3.0, rect.height())), CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 }, ACCENT_2);
        response.response.interact(Sense::click())
    });
    if shown.inner.clicked() {
        store.toast = None;
    }
}
