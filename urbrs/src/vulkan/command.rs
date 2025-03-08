use std::sync::Arc;

use super::device::{Device, DeviceQueue};

pub struct CommandPool {
    device: Arc<Device>,
    handle: ash::vk::CommandPool,
}

impl CommandPool {
    pub fn new(
        device: Arc<Device>,
        queue: &DeviceQueue,
        flags: ash::vk::CommandPoolCreateFlags,
    ) -> anyhow::Result<Self> {
        let info = ash::vk::CommandPoolCreateInfo::default()
            .flags(flags)
            .queue_family_index(queue.idx);

        let handle = unsafe { device.handle().create_command_pool(&info, None)? };

        Ok(Self { device, handle })
    }

    pub fn handle(&self) -> ash::vk::CommandPool {
        self.handle
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_command_pool(self.handle, None);
        }
    }
}

pub struct CommandBuffer {
    device: Arc<Device>,
    handle: ash::vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn new(device: Arc<Device>, pool: &CommandPool) -> anyhow::Result<Self> {
        let info = ash::vk::CommandBufferAllocateInfo::default()
            .command_pool(pool.handle())
            .command_buffer_count(1)
            .level(ash::vk::CommandBufferLevel::PRIMARY);

        let handle = unsafe { device.handle().allocate_command_buffers(&info)? }[0];

        Ok(Self { device, handle })
    }

    pub fn begin(&self, usage_flags: ash::vk::CommandBufferUsageFlags) -> anyhow::Result<()> {
        let info = ash::vk::CommandBufferBeginInfo::default().flags(usage_flags);

        unsafe {
            self.device
                .handle()
                .begin_command_buffer(self.handle, &info)?;
        }

        Ok(())
    }

    pub fn end(&self) -> anyhow::Result<()> {
        unsafe { self.device.handle().end_command_buffer(self.handle)? };

        Ok(())
    }

    pub fn submit_info(&self) -> ash::vk::CommandBufferSubmitInfo {
        ash::vk::CommandBufferSubmitInfo::default()
            .device_mask(0)
            .command_buffer(self.handle)
    }

    pub fn handle(&self) -> ash::vk::CommandBuffer {
        self.handle
    }
}
