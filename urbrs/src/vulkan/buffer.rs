use std::sync::Arc;

use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};

use super::context::Context;

pub struct Buffer {
    context: Arc<Context>,
    _size: usize,
    handle: ash::vk::Buffer,

    allocation: Option<gpu_allocator::vulkan::Allocation>,
}

impl Buffer {
    pub fn new(
        context: Arc<Context>,
        size: usize,
        usage: ash::vk::BufferUsageFlags,
        sharing_mode: ash::vk::SharingMode,
    ) -> anyhow::Result<Self> {
        let info = ash::vk::BufferCreateInfo::default()
            .size(size as u64)
            .sharing_mode(sharing_mode)
            .usage(usage);

        let handle = unsafe { context.device().handle().create_buffer(&info, None)? };

        Ok(Self {
            context,
            handle,
            _size: size,
            allocation: None,
        })
    }

    pub fn handle(&self) -> ash::vk::Buffer {
        self.handle
    }

    pub fn memory_requirements(&self) -> ash::vk::MemoryRequirements {
        unsafe {
            self.context
                .device()
                .handle()
                .get_buffer_memory_requirements(self.handle)
        }
    }

    pub fn allocation_mut(&mut self) -> Option<&mut Allocation> {
        self.allocation.as_mut()
    }

    pub fn allocate(&mut self, desc: AllocationCreateDesc) -> anyhow::Result<()> {
        let allocation = self.context.alloc_gpu_mem(&desc)?;

        unsafe {
            self.context.device().handle().bind_buffer_memory(
                self.handle,
                allocation.memory(),
                allocation.offset(),
            )?;
        }

        self.allocation = Some(allocation);

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            if let Some(allocation) = self.allocation.take() {
                self.context.free_gpu_mem(allocation).unwrap();
            }

            self.context
                .device()
                .handle()
                .destroy_buffer(self.handle, None)
        };
    }
}
