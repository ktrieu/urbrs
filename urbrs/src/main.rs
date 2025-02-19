use std::io::{self, BufReader};

use renderer::Renderer;
use vulkan::context::Context;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
};

mod renderer;
mod vulkan;

struct Window {
    handle: winit::window::Window,
    context: Context,
    renderer: Renderer,
}

struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            // We already initialized. Just move on.
            return;
        }

        let winit_window = event_loop
            .create_window(
                winit::window::WindowAttributes::default()
                    .with_title("urbrs")
                    .with_inner_size(PhysicalSize::new(1920, 1080)),
            )
            .expect("window creation should succeed");

        let raw_display_handle = event_loop
            .display_handle()
            .expect("display handle should be valid")
            .as_raw();

        let raw_window_handle = winit_window
            .window_handle()
            .expect("window handle should be valid")
            .as_raw();

        let context = Context::new(&winit_window, raw_display_handle, raw_window_handle)
            .expect("context creation should succeed");

        let renderer = Renderer::new(context.device(), context.swapchain())
            .expect("renderer creation should succeed");

        self.window = Some(Window {
            handle: winit_window,
            context,
            renderer,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let window = self.window.as_ref().expect("window should be initialized");

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.window.as_ref().unwrap();

        window.renderer.render().unwrap();
        window.handle.request_redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.window
            .as_ref()
            .expect("window should be initialized")
            .context
            .wait_idle()
            .expect("wait idle should succeed");
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App { window: None };
    let _ = event_loop.run_app(&mut app);
}
