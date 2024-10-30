use bevy_ecs::component::Component;

use bezier::BezierCurve;
use vello::peniko::Color;
use super::bezier;

#[derive(Component, Clone)]
pub struct Spline { pub bez_spline: Vec<BezierCurve>, pub color: Color }