// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use anyhow::{Ok, Result};
use std::num::NonZeroUsize;
use rand::Rng;
use nalgebra::Vector2 as Vec2;
use std::sync::Arc;
use vello::kurbo::{Affine, BezPath, Point, Stroke};
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;
mod bezier;

use vello::wgpu;

// Simple struct to hold the state of the renderer
pub struct ActiveRenderState<'s> {
    surface: RenderSurface<'s>,
    window: Arc<Window>,
}

enum RenderState<'s> {
    Active(ActiveRenderState<'s>),
    Suspended(Option<Arc<Window>>),
}

enum DrawFlag {
    Curve(Vec2<f64>, Vec2<f64>, Vec2<f64>, Vec2<f64>, Color)
}

struct SimpleVelloApp<'s> {
    context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    state: RenderState<'s>,
    scene: Scene,
    draw_flags: Vec<DrawFlag>,
    points: Vec<Vec2<f64>>,
    is_drawing: bool,
    current_id: u64
}

impl<'s> ApplicationHandler for SimpleVelloApp<'s> {
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
                            self.draw_flags.clear();
                            render_state.window.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.scene.reset();

                for draw_flag in &self.draw_flags {
                    match draw_flag {
                        DrawFlag::Curve(
                            point0,
                            point1,
                            point2,
                            point3,
                            color
                        ) => {
                                let (start, pt1, pt2, pt3) = (
                                    Point::new(point0.x, point0.y),
                                    Point::new(point1.x, point1.y),
                                    Point::new(point2.x, point2.y),
                                    Point::new(point3.x, point3.y)
                                );

                                let mut path = BezPath::new();

                                path.move_to(Point::new(start.x, start.y));
                                path.curve_to(pt1, pt2, pt3);

                                self.scene.stroke(
                                    &Stroke::new(2.0),
                                    Affine::IDENTITY,
                                        color,
                                    None,
                                    &path,
                                );
                            }
                        }
                }

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
                self.is_drawing = true;
                self.points.clear();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            } => {
                self.is_drawing = false;
                render_state.window.request_redraw();
                self.current_id+=1;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x, position.y);
                if self.is_drawing {
                    self.points.append(&mut vec![Vec2::new(point.x, point.y)]);
                    if self.points.len() >= 3 {
                        let bezier_points = bezier::vec_to_bezier_control_points(&self.points);

                        let draw_flag = if self.current_id < self.draw_flags.len() as u64 {
                            self.draw_flags.pop()
                        } else {
                            Option::None
                        };

                        let color = match draw_flag {
                            Some(draw) => {
                                match draw {
                                    DrawFlag::Curve(_, _, _, _, color) => color
                                }
                            },
                            None => {
                                let mut th_rand = rand::thread_rng();
                                Color::rgb8(
                                    th_rand.gen_range(5..255),
                                    th_rand.gen_range(5..255),
                                    th_rand.gen_range(5..255), 
                                )
                            }
                        };

                        self.draw_flags.push(
                            DrawFlag::Curve(
                                bezier_points[0], 
                                bezier_points[1], 
                                bezier_points[2], 
                                bezier_points[3],
                                color
                            )
                        );
                        
                        render_state.window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let mut app = SimpleVelloApp {
        context: RenderContext::new(),
        renderers: Vec::new(),
        state: RenderState::Suspended(None),
        scene: Scene::new(),
        points: Vec::new(),
        is_drawing: false,
        draw_flags: Vec::new(),
        current_id: 0
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
