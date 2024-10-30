use nalgebra::Vector2 as Vec2;
#[derive(Clone, Copy)]
pub struct BezierCurve {
    pub start: Vec2<f64>,
    pub control1: Vec2<f64>,
    pub control2: Vec2<f64>,
    pub end: Vec2<f64>,
}