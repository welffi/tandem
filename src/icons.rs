use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};
use std::f32::consts::PI;

struct Grid {
    rect: Rect,
    units: f32,
}

impl Grid {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        let s = self.rect.width().min(self.rect.height()) / self.units;
        let origin = self.rect.center() - eframe::egui::vec2(self.units, self.units) * s / 2.0;
        origin + eframe::egui::vec2(x, y) * s
    }

    fn unit(&self) -> f32 {
        self.rect.width().min(self.rect.height()) / self.units
    }

    fn line(&self, painter: &Painter, points: &[(f32, f32)], width: f32, color: Color32) {
        let points: Vec<Pos2> = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        painter.add(Shape::line(points, Stroke::new(width * self.unit(), color)));
    }

    fn fill(&self, painter: &Painter, points: &[(f32, f32)], color: Color32) {
        let points: Vec<Pos2> = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }

    fn arc(&self, painter: &Painter, center: (f32, f32), radius: f32, from: f32, to: f32, width: f32, color: Color32) {
        let points: Vec<Pos2> = (0..=12).map(|i| from + (to - from) * i as f32 / 12.0).map(|a| self.at(center.0 + radius * a.cos(), center.1 + radius * a.sin())).collect();
        painter.add(Shape::line(points, Stroke::new(width * self.unit(), color)));
    }
}

fn grid(rect: Rect, units: f32) -> Grid {
    Grid { rect, units }
}

pub fn play(painter: &Painter, rect: Rect, playing: bool, color: Color32) {
    let g = grid(rect, 16.0);
    if playing {
        g.fill(painter, &[(3.0, 2.0), (7.0, 2.0), (7.0, 14.0), (3.0, 14.0)], color);
        g.fill(painter, &[(9.0, 2.0), (13.0, 2.0), (13.0, 14.0), (9.0, 14.0)], color);
    } else {
        g.fill(painter, &[(3.0, 1.5), (14.0, 8.0), (3.0, 14.5)], color);
    }
}

pub fn speaker(painter: &Painter, rect: Rect, muted: bool, level: f64, color: Color32) {
    let g = grid(rect, 16.0);
    g.fill(painter, &[(2.0, 6.0), (5.0, 6.0), (9.0, 3.0), (9.0, 13.0), (5.0, 10.0), (2.0, 10.0)], color);
    if muted {
        g.line(painter, &[(11.0, 6.0), (15.0, 10.0)], 1.5, color);
        g.line(painter, &[(15.0, 6.0), (11.0, 10.0)], 1.5, color);
        return;
    }
    if level > 0.0 {
        g.arc(painter, (9.5, 8.0), 2.3, -PI / 2.5, PI / 2.5, 1.5, color);
    }
    if level > 0.5 {
        g.arc(painter, (8.5, 8.0), 4.6, -PI / 2.6, PI / 2.6, 1.5, color);
    }
}

pub fn fullscreen(painter: &Painter, rect: Rect, active: bool, color: Color32) {
    let g = grid(rect, 16.0);
    let corners: [[(f32, f32); 3]; 4] = if active {
        [[(6.0, 2.0), (6.0, 6.0), (2.0, 6.0)], [(10.0, 2.0), (10.0, 6.0), (14.0, 6.0)], [(6.0, 14.0), (6.0, 10.0), (2.0, 10.0)], [(10.0, 14.0), (10.0, 10.0), (14.0, 10.0)]]
    } else {
        [[(2.0, 6.0), (2.0, 2.0), (6.0, 2.0)], [(14.0, 6.0), (14.0, 2.0), (10.0, 2.0)], [(2.0, 10.0), (2.0, 14.0), (6.0, 14.0)], [(14.0, 10.0), (14.0, 14.0), (10.0, 14.0)]]
    };
    for corner in corners {
        g.line(painter, &corner, 1.6, color);
    }
}

pub fn music(painter: &Painter, rect: Rect, color: Color32) {
    let g = grid(rect, 24.0);
    g.line(painter, &[(9.0, 18.0), (9.0, 5.0), (21.0, 3.0), (21.0, 16.0)], 1.8, color);
    for (x, y) in [(6.0, 18.0), (18.0, 16.0)] {
        painter.circle_stroke(g.at(x, y), 3.0 * g.unit(), Stroke::new(1.8 * g.unit(), color));
    }
}

pub fn film(painter: &Painter, rect: Rect, color: Color32) {
    let g = grid(rect, 24.0);
    g.line(painter, &[(3.0, 5.0), (16.0, 5.0), (16.0, 19.0), (3.0, 19.0), (3.0, 5.0)], 1.8, color);
    g.line(painter, &[(16.0, 10.0), (21.0, 7.0), (21.0, 17.0), (16.0, 14.0)], 1.8, color);
}

pub fn folder(painter: &Painter, rect: Rect, color: Color32) {
    let g = grid(rect, 16.0);
    g.fill(painter, &[(1.5, 3.5), (6.0, 3.5), (7.5, 5.0), (1.5, 5.0)], color);
    g.fill(painter, &[(1.5, 5.0), (14.5, 5.0), (14.5, 13.0), (1.5, 13.0)], color);
}

pub fn centered(rect: Rect, size: f32) -> Rect {
    Rect::from_center_size(rect.center(), eframe::egui::vec2(size, size))
}

pub fn eye(painter: &Painter, rect: Rect, hidden: bool, color: Color32) {
    let g = grid(rect, 16.0);
    g.line(painter, &[(1.0, 8.0), (4.0, 5.0), (8.0, 4.0), (12.0, 5.0), (15.0, 8.0), (12.0, 11.0), (8.0, 12.0), (4.0, 11.0), (1.0, 8.0)], 1.3, color);
    painter.circle_filled(g.at(8.0, 8.0), 2.0 * g.unit(), color);
    if hidden {
        g.line(painter, &[(3.0, 14.0), (13.0, 2.0)], 1.5, color);
    }
}

pub fn cross(painter: &Painter, rect: Rect, color: Color32) {
    let g = grid(rect, 16.0);
    g.line(painter, &[(4.0, 4.0), (12.0, 12.0)], 1.5, color);
    g.line(painter, &[(12.0, 4.0), (4.0, 12.0)], 1.5, color);
}
