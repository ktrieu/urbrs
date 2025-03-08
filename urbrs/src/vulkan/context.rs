use std::fmt::Display;
use std::sync::{Arc, Mutex};

use ash::prelude::VkResult;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::device::Device;
use super::instance::{Instance, InstanceCreateError};
use super::phys_device::PhysicalDevice;
use super::surface::Surface;
use super::swapchain::Swapchain;

pub struct Context {
    _instance: Arc<Instance>,
    _surface: Arc<Surface>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
}

#[derive(Debug)]
pub enum ContextCreateError {
    InstanceError(InstanceCreateError),
    VkError(ash::vk::Result),
    AllocatorError(gpu_allocator::AllocationError),
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

impl From<gpu_allocator::AllocationError> for ContextCreateError {
    fn from(value: gpu_allocator::AllocationError) -> Self {
        ContextCreateError::AllocatorError(value)
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
            ContextCreateError::AllocatorError(allocation_error) => {
                write!(f, "gpu allocation error: {allocation_error}")
            }
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

        let alloc_create_desc = gpu_allocator::vulkan::AllocatorCreateDesc {
            instance: instance.handle().clone(),
            device: device.handle().clone(),
            physical_device: device.physical_device().handle(),
            debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
            // TODO: yeah this should be supported.
            buffer_device_address: true,
            allocation_sizes: gpu_allocator::AllocationSizes::default(),
        };

        let allocator = Arc::new(Mutex::new(gpu_allocator::vulkan::Allocator::new(
            &alloc_create_desc,
        )?));

        Ok(Self {
            _instance: instance,
            _surface: surface,
            device,
            swapchain,
            allocator,
        })
    }

    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn swapchain(&self) -> Arc<Swapchain> {
        self.swapchain.clone()
    }

    pub fn alloc_gpu_mem(
        &self,
        desc: &gpu_allocator::vulkan::AllocationCreateDesc,
    ) -> VkResult<gpu_allocator::vulkan::Allocation> {
        let mut allocator = self.allocator.lock().unwrap();

        // It's fine I'm just going to anyhow this soon anyway.
        Ok(allocator.allocate(desc).unwrap())
    }

    pub fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.device.handle().device_wait_idle() }?;

        Ok(())
    }
}
