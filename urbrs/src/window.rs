use std::sync::Arc;

use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
};

use crate::{renderer::Renderer, vulkan::context::Context};

pub struct Window {
    handle: winit::window::Window,
    context: Arc<Context>,
    renderer: Renderer,
}

impl Window {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let winit_window = event_loop
            .create_window(
                winit::window::WindowAttributes::default()
                    .with_title("urbrs")
                    .with_inner_size(PhysicalSize::new(1920, 1080)),
            )
            .expect("window creation should succeed");

        let display_handle = event_loop
            .display_handle()
            .expect("display handle should be valid")
            .as_raw();

        let raw_window_handle = winit_window
            .window_handle()
            .expect("window handle should be valid")
            .as_raw();

        let context = Arc::new(
            Context::new(&winit_window, display_handle, raw_window_handle)
                .expect("context creation should succeed"),
        );

        let renderer = Renderer::new(context.clone(), context.swapchain())
            .expect("renderer creation should succeed");

        Self {
            handle: winit_window,
            context,
            renderer,
        }
    }

    pub fn render(&self) {
        self.renderer.render().unwrap();
        self.handle.request_redraw();
    }

    pub fn exit(&self) {
        self.context.wait_idle().expect("wait idle should succeed");
    }
}
