use std::sync::{Arc, Mutex};

use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::device::Device;
use super::instance::Instance;
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

impl Context {
    pub fn new(
        window: &winit::window::Window,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> anyhow::Result<Self> {
        let instance = Arc::new(Instance::new(display_handle)?);

        let surface = Arc::new(Surface::new(
            instance.clone(),
            window_handle,
            display_handle,
        )?);

        let phys_device = PhysicalDevice::select_device(&instance.handle(), &surface)?
            .ok_or(anyhow::anyhow!("no valid physical device found"))?;

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
    ) -> anyhow::Result<gpu_allocator::vulkan::Allocation> {
        let mut allocator = self.allocator.lock().unwrap();

        // It's fine I'm just going to anyhow this soon anyway.
        Ok(allocator.allocate(desc)?)
    }

    pub fn free_gpu_mem(
        &self,
        allocation: gpu_allocator::vulkan::Allocation,
    ) -> anyhow::Result<()> {
        let mut allocator = self.allocator.lock().unwrap();

        allocator.free(allocation)?;

        Ok(())
    }

    pub fn wait_idle(&self) -> anyhow::Result<()> {
        unsafe { self.device.handle().device_wait_idle() }?;

        Ok(())
    }
}
