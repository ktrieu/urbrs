use std::sync::Arc;

use super::device::Device;

pub struct Semaphore {
    device: Arc<Device>,
    handle: ash::vk::Semaphore,
}

impl Semaphore {
    pub fn new(device: Arc<Device>, flags: ash::vk::SemaphoreCreateFlags) -> anyhow::Result<Self> {
        let info = ash::vk::SemaphoreCreateInfo::default().flags(flags);

        let handle = unsafe { device.handle().create_semaphore(&info, None)? };

        Ok(Self { device, handle })
    }

    pub fn submit_info(
        &self,
        stages: ash::vk::PipelineStageFlags2,
    ) -> ash::vk::SemaphoreSubmitInfo {
        ash::vk::SemaphoreSubmitInfo::default()
            .semaphore(self.handle)
            .stage_mask(stages)
            .device_index(0)
            .value(1)
    }

    pub fn handle(&self) -> ash::vk::Semaphore {
        self.handle
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_semaphore(self.handle, None);
        }
    }
}

pub struct Fence {
    device: Arc<Device>,
    handle: ash::vk::Fence,
}

impl Fence {
    pub fn new(device: Arc<Device>, flags: ash::vk::FenceCreateFlags) -> anyhow::Result<Self> {
        let info = ash::vk::FenceCreateInfo::default().flags(flags);

        let handle = unsafe { device.handle().create_fence(&info, None)? };

        Ok(Self { device, handle })
    }

    pub fn wait(&self, timeout_ns: u64) -> anyhow::Result<()> {
        unsafe {
            self.device
                .handle()
                .wait_for_fences(&[self.handle], true, timeout_ns)?
        };

        Ok(())
    }

    pub fn reset(&self) -> anyhow::Result<()> {
        unsafe { self.device.handle().reset_fences(&[self.handle])? };

        Ok(())
    }

    pub fn handle(&self) -> ash::vk::Fence {
        self.handle
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe { self.device.handle().destroy_fence(self.handle, None) };
    }
}
