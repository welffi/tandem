use eframe::egui::{Pos2, Rect, vec2};

const HANDLE_REACH: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spot {
    Corner,
    Inside,
}

pub fn rotate(point: Pos2, center: Pos2, degrees: f32) -> Pos2 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let d = point - center;
    center + vec2(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
}

pub fn corners(rect: Rect, degrees: f32) -> [Pos2; 4] {
    [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()].map(|corner| rotate(corner, rect.center(), degrees))
}

pub fn spot_at(rect: Rect, degrees: f32, point: Pos2) -> Option<Spot> {
    if corners(rect, degrees).iter().any(|corner| corner.distance(point) <= HANDLE_REACH) {
        return Some(Spot::Corner);
    }
    rect.contains(rotate(point, rect.center(), -degrees)).then_some(Spot::Inside)
}
