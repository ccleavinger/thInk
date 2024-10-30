#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::{Ok, Result};
use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use components::shapes::points::Points;
use components::shapes::spline::Spline;
use systems::update_spline::sys_update_spline;
use std::num::NonZeroUsize;
use std::time::Instant;
use rand::Rng;
use nalgebra::Vector2 as Vec2;
use std::sync::Arc;
use vello::kurbo::{Affine, BezPath, Point, Stroke};
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use vello::wgpu;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;
use render::{ActiveRenderState, RenderState};

mod components;
mod systems;
mod math;
mod render;

struct ThinkApp<'s> {
    context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    state: RenderState<'s>,
    scene: Scene,
    points: Vec<Vec2<f64>>,
    is_drawing: bool,
    start_draw_time: Option<Instant>,
    last_draw_time: Instant,
    world: World // ecs for everything. ui, shapes, etc
}


impl<'s> ApplicationHandler for ThinkApp<'s> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let RenderState::Suspended(cached_window) = &mut self.state else {
            return;
        };

        let window = cached_window
            .take()
            .unwrap_or_else(|| create_winit_window(event_loop));

        let size = window.inner_size();
        let surface_future = self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        );
        let surface = pollster::block_on(surface_future).expect("Error creating surface");

        self.renderers
            .resize_with(self.context.devices.len(), || None);
        self.renderers[surface.dev_id]
            .get_or_insert_with(|| create_vello_renderer(&self.context, &surface));

        self.state = RenderState::Active(ActiveRenderState { window, surface });
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let RenderState::Active(state) = &self.state {
            self.state = RenderState::Suspended(Some(state.window.clone()));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let render_state = match &mut self.state {
            RenderState::Active(state) if state.window.id() == window_id => state,
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.context
                        .resize_surface(&mut render_state.surface, size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                match event.logical_key {
                    winit::keyboard::Key::Character(ch) => {
                        if ch == "r" || ch == "R" {
                            self.world = World::default();
                            render_state.window.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.scene.reset();

                let mut query = self.world.query::<&Spline>();
                query.iter(&self.world).for_each(|spline| {
                    let mut bez_path = BezPath::new();

                    for (i, curve) in spline.bez_spline.iter().enumerate() {
                        if i == 0 {
                            bez_path.move_to(Point::new(curve.start.x, curve.start.y));
                        }
                        bez_path.curve_to(
                            Point::new(curve.control1.x, curve.control1.y),
                            Point::new(curve.control2.x, curve.control2.y),
                            Point::new(curve.end.x, curve.end.y),
                        );
                    }
                    self.scene.stroke(
                        &Stroke::new(2.0), 
                        Affine::IDENTITY, 
                        &spline.color, 
                        None, 
                        &bez_path
                    );
                });

                

                let surface = &render_state.surface;
                let width = surface.config.width;
                let height = surface.config.height;
                let device_handle = &self.context.devices[surface.dev_id];
                let surface_texture = surface
                    .surface
                    .get_current_texture()
                    .expect("failed to get surface texture");

                self.renderers[surface.dev_id]
                    .as_mut()
                    .unwrap()
                    .render_to_surface(
                        &device_handle.device,
                        &device_handle.queue,
                        &self.scene,
                        &surface_texture,
                        &vello::RenderParams {
                            base_color: Color::BLACK,
                            width,
                            height,
                            antialiasing_method: AaConfig::Msaa16,
                        },
                    )
                    .expect("failed to render to surface");

                surface_texture.present();
                device_handle.device.poll(wgpu::Maintain::Poll);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                let mut rng = rand::thread_rng();
                self.is_drawing = true;
                self.points.clear();
                self.world.spawn(
                    (
                            Spline { 
                                bez_spline: Vec::new(), 
                                color: Color::rgb8(
                                    rng.gen_range(0..255), 
                                    rng.gen_range(0..255), 
                                    rng.gen_range(0..255)
                                ) 
                            }, 
                            Points { points: Vec::new() }
                        )
                );
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            } => {
                self.is_drawing = false;
                render_state.window.request_redraw();
                // remove the points component from the entity once the user is done drawing
                {
                    let mut query = self.world.query::<(Entity, &mut Points)>();
                    let (entity, _) = query.single_mut(&mut self.world);
                    self.world.entity_mut(entity).remove::<Points>();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.is_drawing {
                    let now = Instant::now();

                    if self.start_draw_time.is_none() {
                        self.start_draw_time = Some(now);
                    }

                    self.last_draw_time = now;
                    
                    let delta_time = self.last_draw_time.duration_since(self.start_draw_time.unwrap()).as_secs_f32();
                    
                    if delta_time > 0.05 {
                        let mut query = self.world.query::<(&mut Points, &mut Spline)>();

                        // push in external state
                        match query.single_mut(&mut self.world) {
                            (mut points, _) => {
                                points.points.push(Vec2::new(position.x as f64, position.y as f64));
                            }
                        }

                        // decoupled logic for updating the spline
                        // should scale easier than the atrocity I had before
                        self.world.run_system_once(sys_update_spline);

                        render_state.window.request_redraw();
                    }
                    
                    
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let mut app = ThinkApp {
        context: RenderContext::new(),
        renderers: Vec::new(),
        state: RenderState::Suspended(None),
        scene: Scene::new(),
        points: Vec::new(),
        is_drawing: false,
        start_draw_time: None,
        last_draw_time: Instant::now(),
        world: World::default(),
    };

    let event_loop = EventLoop::new()?;
    event_loop
        .run_app(&mut app)
        .expect("Error running event loop");
    Ok(())

}


/// Helper function that creates a Winit window and returns it (wrapped in an Arc for sharing between threads)
fn create_winit_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attr = Window::default_attributes()
        .with_inner_size(LogicalSize::new(1044, 800))
        .with_resizable(true)
        .with_title("Vello Shapes");
    Arc::new(event_loop.create_window(attr).unwrap())
}

/// Helper function that creates a vello `Renderer` for a given `RenderContext` and `RenderSurface`
fn create_vello_renderer(render_cx: &RenderContext, surface: &RenderSurface) -> Renderer {
    Renderer::new(
        &render_cx.devices[surface.dev_id].device,
        RendererOptions {
            surface_format: Some(surface.format),
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: NonZeroUsize::new(1),
        },
    )
        .expect("Couldn't create renderer")
}