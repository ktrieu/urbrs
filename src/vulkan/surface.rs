use std::sync::Arc;

use ash::prelude::VkResult;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::instance::Instance;

pub struct Surface {
    instance: Arc<Instance>,
    surface_instance: ash::khr::surface::Instance,
    handle: ash::vk::SurfaceKHR,
}

impl Surface {
    // Safety: The Surface must be dropped before the Instance passed in.
    pub unsafe fn new(
        instance: Arc<Instance>,
        window: RawWindowHandle,
        display: RawDisplayHandle,
    ) -> VkResult<Self> {
        let surface =
            ash_window::create_surface(instance.entry(), instance.handle(), display, window, None)?;

        let surface_instance =
            ash::khr::surface::Instance::new(instance.entry(), instance.handle());

        Ok(Self {
            instance,
            surface_instance,
            handle: surface,
        })
    }

    pub fn handle(&self) -> &ash::vk::SurfaceKHR {
        &self.handle
    }

    pub fn surface_instance(&self) -> &ash::khr::surface::Instance {
        &self.surface_instance
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_instance.destroy_surface(self.handle, None);
        }
    }
}
