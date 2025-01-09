use std::fmt::Display;

use super::device::Device;
use super::instance::{Instance, InstanceCreateError};
use super::phys_device::PhysicalDevice;

pub struct Context {
    instance: Instance,
    device: Device,
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
    pub fn new() -> Result<Self, ContextCreateError> {
        let instance = Instance::new()?;

        let phys_device = PhysicalDevice::select_device(&instance.instance())?
            .ok_or(ContextCreateError::NoDevice)?;

        let device = Device::new(&instance.instance(), phys_device)?;

        Ok(Self { instance, device })
    }
}
