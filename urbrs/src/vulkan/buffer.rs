use std::sync::Arc;

use ash::prelude::VkResult;

use super::device::Device;

pub struct Buffer {
    device: Arc<Device>,
    handle: ash::vk::Buffer,
}

impl Buffer {
    pub fn new(
        device: Arc<Device>,
        size: u64,
        usage: ash::vk::BufferUsageFlags,
        sharing_mode: ash::vk::SharingMode,
    ) -> VkResult<Self> {
        let info = ash::vk::BufferCreateInfo::default()
            .size(size)
            .sharing_mode(sharing_mode)
            .usage(usage);

        let handle = unsafe { device.handle().create_buffer(&info, None)? };

        Ok(Self { device, handle })
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { self.device.handle().destroy_buffer(self.handle, None) };
    }
}
