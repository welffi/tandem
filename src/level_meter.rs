use crate::theme::{METER_BG, METER_CLIP, METER_LED_OFF, METER_SCALE, METER_STOPS};
use eframe::egui::{Align2, Color32, FontId, Mesh, Painter, Rect, Response, Sense, Ui, pos2, vec2};
use std::time::{Duration, Instant};

pub const FLOOR_DB: f32 = -60.0;
const FALL_DB_PER_SEC: f32 = 26.0;
const HOLD: Duration = Duration::from_millis(1500);
const HOLD_FALL_DB_PER_SEC: f32 = 18.0;
const CLIP_LEVEL: f32 = 0.999;
const SCALE_MARKS: [i32; 10] = [0, -6, -12, -18, -24, -30, -36, -42, -48, -54];
const GUTTER: f32 = 22.0;
const LED: f32 = 7.0;

pub fn to_db(amplitude: f32) -> f32 {
    if amplitude > 0.0 { (20.0 * amplitude.log10()).max(FLOOR_DB) } else { FLOOR_DB }
}

pub fn position(db: f32) -> f32 {
    (db - FLOOR_DB) / -FLOOR_DB
}

pub struct Meter {
    pub level: [f32; 2],
    pub hold: [f32; 2],
    hold_at: [Option<Instant>; 2],
    pub clipped: [bool; 2],
    last: Option<Instant>,
}

impl Default for Meter {
    fn default() -> Meter {
        Meter { level: [FLOOR_DB; 2], hold: [FLOOR_DB; 2], hold_at: [None; 2], clipped: [false; 2], last: None }
    }
}

impl Meter {
    pub fn update(&mut self, amplitudes: [f32; 2], now: Instant) {
        let dt = self.last.map_or(0.0, |last| (now - last).as_secs_f32().min(0.25));
        self.last = Some(now);
        for (i, &amplitude) in amplitudes.iter().enumerate() {
            let db = to_db(amplitude);
            self.level[i] = db.max(self.level[i] - FALL_DB_PER_SEC * dt);
            if db >= self.hold[i] {
                self.hold[i] = db;
                self.hold_at[i] = Some(now);
            } else if self.hold_at[i].is_none_or(|at| now - at > HOLD) {
                self.hold[i] = self.level[i].max(self.hold[i] - HOLD_FALL_DB_PER_SEC * dt);
            }
            self.clipped[i] |= amplitude >= CLIP_LEVEL;
        }
    }

    pub fn is_resting(&self) -> bool {
        self.level.iter().chain(&self.hold).all(|&db| db <= FLOOR_DB)
    }
}

fn color_at(db: f32) -> Color32 {
    let next = METER_STOPS.iter().position(|(stop, _)| *stop >= db).unwrap_or(METER_STOPS.len() - 1).max(1);
    let ((a, ca), (b, cb)) = (METER_STOPS[next - 1], METER_STOPS[next]);
    ca.lerp_to_gamma(cb, ((db - a) / (b - a)).clamp(0.0, 1.0))
}

fn gradient(painter: &Painter, bar: Rect, vertical: bool, from: f32, to: f32, alpha: f32) {
    if to <= from {
        return;
    }
    let mut cuts: Vec<f32> = METER_STOPS.iter().map(|(db, _)| position(*db)).filter(|p| *p > from && *p < to).collect();
    cuts.insert(0, from);
    cuts.push(to);
    let at = |p: f32| {
        if vertical {
            (pos2(bar.left(), bar.bottom() - bar.height() * p), pos2(bar.right(), bar.bottom() - bar.height() * p))
        } else {
            (pos2(bar.left() + bar.width() * p, bar.top()), pos2(bar.left() + bar.width() * p, bar.bottom()))
        }
    };
    let mut mesh = Mesh::default();
    for pair in cuts.windows(2) {
        let (a0, a1) = at(pair[0]);
        let (b0, b1) = at(pair[1]);
        let ca = color_at(FLOOR_DB - FLOOR_DB * pair[0]).gamma_multiply(alpha);
        let cb = color_at(FLOOR_DB - FLOOR_DB * pair[1]).gamma_multiply(alpha);
        let base = mesh.vertices.len() as u32;
        for (p, c) in [(a0, ca), (a1, ca), (b1, cb), (b0, cb)] {
            mesh.colored_vertex(p, c);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(mesh);
}

pub fn paint(painter: &Painter, rect: Rect, meter: &Meter, vertical: bool, scale: bool) {
    let gap = if vertical { 2.0 } else { 1.0 };
    let gutter = if scale { GUTTER } else { 0.0 };
    let led = if scale { LED } else { 0.0 };
    let top = if scale { led + gap * 2.0 } else { 0.0 };
    let bars: Vec<Rect> = (0..2)
        .map(|i| {
            let i = i as f32;
            if vertical {
                let bw = (rect.width() - gutter - gap) / 2.0;
                Rect::from_min_size(pos2(rect.left() + gutter + i * (bw + gap), rect.top() + top), vec2(bw, rect.height() - top))
            } else {
                let bh = (rect.height() - gap) / 2.0;
                Rect::from_min_size(pos2(rect.left(), rect.top() + i * (bh + gap)), vec2(rect.width(), bh))
            }
        })
        .collect();
    for (i, bar) in bars.iter().enumerate() {
        painter.rect_filled(*bar, 0.0, METER_BG);
        gradient(painter, *bar, vertical, 0.0, 1.0, 0.16);
        gradient(painter, *bar, vertical, 0.0, position(meter.level[i]), 1.0);
        let hold = position(meter.hold[i]);
        if hold > 0.005 {
            let tick = 1.5 / if vertical { bar.height() } else { bar.width() };
            gradient(painter, *bar, vertical, (hold - tick).max(0.0), hold, 1.0);
        }
        if scale {
            let light = Rect::from_min_size(pos2(bar.left(), rect.top()), vec2(bar.width(), led));
            painter.rect_filled(light, 0.0, if meter.clipped[i] { METER_CLIP } else { METER_LED_OFF });
        }
    }
    if scale && vertical {
        let bar = bars[0];
        for db in SCALE_MARKS {
            let y = (bar.bottom() - bar.height() * position(db as f32)).max(bar.top() + 5.0);
            painter.text(pos2(rect.left() + gutter - 4.0, y), Align2::RIGHT_CENTER, db.to_string(), FontId::proportional(9.0), METER_SCALE);
        }
    }
}

pub fn show(ui: &mut Ui, rect: Rect, meter: &mut Meter, vertical: bool, scale: bool) -> Response {
    let response = ui.allocate_rect(rect, if scale { Sense::click() } else { Sense::hover() });
    if scale && response.clicked() {
        meter.clipped = [false; 2];
    }
    paint(ui.painter(), rect, meter, vertical, scale);
    if scale { response.on_hover_text("Audio levels in dB. Click to reset the clip lights.") } else { response }
}
