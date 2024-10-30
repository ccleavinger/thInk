use bevy_ecs::component::Component;
use nalgebra::Vector2 as Vec2;

#[derive(Component, Clone)]
pub struct Points { pub points: Vec<Vec2<f64>> }