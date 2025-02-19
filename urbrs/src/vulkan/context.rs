use std::fmt::Display;
use std::io::{BufReader, Read, Seek};
use std::ops::{Div, Rem};
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use ash::prelude::VkResult;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::device::Device;
use super::instance::{Instance, InstanceCreateError};
use super::phys_device::PhysicalDevice;
use super::pipeline::PipelineBuilder;
use super::surface::Surface;
use super::swapchain::Swapchain;

pub struct Context {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
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
        window: &winit::window::Window,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, ContextCreateError> {
        let instance = Arc::new(Instance::new(display_handle)?);

        let surface = Arc::new(Surface::new(
            instance.clone(),
            window_handle,
            display_handle,
        )?);

        let phys_device = PhysicalDevice::select_device(&instance.handle(), &surface)?
            .ok_or(ContextCreateError::NoDevice)?;

        let device = Arc::new(Device::new(instance.clone(), phys_device)?);

        let swapchain = Arc::new(Swapchain::new(
            instance.clone(),
            device.clone(),
            surface.clone(),
            window,
        )?);

        Ok(Self {
            instance,
            surface,
            device,
            swapchain,
        })
    }

    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn swapchain(&self) -> Arc<Swapchain> {
        self.swapchain.clone()
    }

    pub fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.device.handle().device_wait_idle() }?;

        Ok(())
    }
}
