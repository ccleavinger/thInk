use std::sync::Arc;

use vello::util::RenderSurface;
use winit::window::Window;

// Simple struct to hold the state of the renderer
pub struct ActiveRenderState<'s> {
    pub surface: RenderSurface<'s>,
    pub window: Arc<Window>,
}

pub enum RenderState<'s> {
    Active(ActiveRenderState<'s>),
    Suspended(Option<Arc<Window>>),
}