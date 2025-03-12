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
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let winit_window = event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title("urbrs")
                .with_inner_size(PhysicalSize::new(1920, 1080)),
        )?;

        let display_handle = event_loop.display_handle()?.as_raw();

        let raw_window_handle = winit_window.window_handle()?.as_raw();

        let context = Arc::new(Context::new(
            &winit_window,
            display_handle,
            raw_window_handle,
        )?);

        let renderer = Renderer::new(
            context.clone(),
            context.swapchain(),
            winit_window.inner_size(),
        )?;

        Ok(Self {
            handle: winit_window,
            context,
            renderer,
        })
    }

    pub fn render(&self) -> anyhow::Result<()> {
        self.renderer.render()?;
        self.handle.request_redraw();

        Ok(())
    }

    pub fn exit(&self) -> anyhow::Result<()> {
        self.context.wait_idle()?;

        Ok(())
    }
}
