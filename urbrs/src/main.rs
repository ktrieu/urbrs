use window::Window;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

mod renderer;
mod vulkan;
mod window;

struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            // We already initialized. Just move on.
            return;
        }

        self.window = Some(Window::new(event_loop).expect("window creation should succeed"))
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Err(err) = self.window.as_ref().unwrap().render() {
            eprintln!("rendering failed: {err:?}");
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Err(err) = self.window.as_ref().unwrap().exit() {
            eprintln!("exit failed: {err:?}")
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop creation should succeed");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App { window: None };
    let _ = event_loop.run_app(&mut app);
}
