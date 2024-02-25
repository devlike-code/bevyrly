use bevy::math::Vec2;

#[derive(Default, Clone, Copy)]
pub struct Circle(pub Vec2, pub f32);

impl From<(Vec2, f32)> for Circle {
    fn from(value: (Vec2, f32)) -> Self {
        Circle(value.0, value.1)
    }
}

#[derive(Default, Clone, Copy)]
pub struct Line(pub Vec2, pub Vec2);

impl From<[Vec2; 2]> for Line {
    fn from(value: [Vec2; 2]) -> Self {
        Line(value[0], value[1])
    }
}

pub fn line_equation(line: Line) -> [f32; 3] {
    let Line(p, q) = line;
    let Vec2 { x: x1, y: y1 } = p;
    let Vec2 { x: x2, y: y2 } = q;
    if x1 == x2 {
        [x1, 0., 0.]
    } else {
        let a = y2 - y1;
        let b = x1 - x2;
        let c = y1 * (x2 - x1) - (y2 - y1) * x1;
        [a, b, c]
    }
}
