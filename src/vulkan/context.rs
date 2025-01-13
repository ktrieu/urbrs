use std::fmt::Display;
use std::mem::ManuallyDrop;
use std::sync::Arc;

use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::device::Device;
use super::instance::{Instance, InstanceCreateError};
use super::phys_device::PhysicalDevice;
use super::surface::Surface;

pub struct Context {
    // These are all ManuallyDrop because we need to control drop order.
    instance: ManuallyDrop<Arc<Instance>>,
    surface: ManuallyDrop<Surface>,
    device: ManuallyDrop<Device>,
}

#[derive(Debug)]
pub enum ContextCreateError {
    InstanceError(InstanceCreateError),
    VkError(ash::vk::Result),
    NoDevice,
}

impl From<InstanceCreateError> for ContextCreateError {
    fn from(value: InstanceCreateError) -> Self {
        ContextCreateError::InstanceError(value)
    }
}

impl From<ash::vk::Result> for ContextCreateError {
    fn from(value: ash::vk::Result) -> Self {
        ContextCreateError::VkError(value)
    }
}

impl Display for ContextCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextCreateError::InstanceError(instance_create_error) => {
                write!(f, "error creating instance: {instance_create_error}")
            }
            ContextCreateError::VkError(vk_error) => {
                write!(f, "vulkan error: {vk_error}")
            }
            ContextCreateError::NoDevice => write!(f, "no suitable device found"),
        }
    }
}

impl Context {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, ContextCreateError> {
        let instance = Arc::new(Instance::new(display_handle)?);

        // Safety: We ensure this is dropped in the right order in the Drop impl.
        let surface = unsafe { Surface::new(instance.clone(), window_handle, display_handle)? };

        let phys_device = PhysicalDevice::select_device(&instance.handle(), &surface)?
            .ok_or(ContextCreateError::NoDevice)?;

        // Safety: We ensure this is dropped in the right order in the Drop impl.
        let device = unsafe { Device::new(instance.clone(), phys_device)? };

        Ok(Self {
            instance: ManuallyDrop::new(instance),
            surface: ManuallyDrop::new(surface),
            device: ManuallyDrop::new(device),
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.surface);
            ManuallyDrop::drop(&mut self.instance);
        };
    }
}
