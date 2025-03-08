use std::{alloc::alloc, ffi::c_void, sync::Arc};

use ash::prelude::VkResult;

use super::{context::Context, device::Device, mesh::Vertex};

pub struct Buffer {
    context: Arc<Context>,
    size: u64,
    handle: ash::vk::Buffer,

    allocation: Option<gpu_allocator::vulkan::Allocation>,
}

pub enum BufferUploadError {
    IncorrectSize,
    VkError(ash::vk::Result),
}

impl From<ash::vk::Result> for BufferUploadError {
    fn from(value: ash::vk::Result) -> Self {
        BufferUploadError::VkError(value)
    }
}

pub struct BufferUpload {}

impl Buffer {
    pub fn new(
        context: Arc<Context>,
        size: u64,
        usage: ash::vk::BufferUsageFlags,
        sharing_mode: ash::vk::SharingMode,
    ) -> VkResult<Self> {
        let info = ash::vk::BufferCreateInfo::default()
            .size(size)
            .sharing_mode(sharing_mode)
            .usage(usage);

        let handle = unsafe { context.device().handle().create_buffer(&info, None)? };

        Ok(Self {
            context,
            handle,
            size,
            allocation: None,
        })
    }

    // Quick and dirty - send the data direct to the GPU after allocating the memory.
    pub fn upload_direct(&mut self, data: &[Vertex]) -> Result<(), BufferUploadError> {
        let data_size = data.len() * Vertex::size();

        if data_size as u64 != self.size {
            return Err(BufferUploadError::IncorrectSize);
        }

        let requirements = unsafe {
            self.context
                .device()
                .handle()
                .get_buffer_memory_requirements(self.handle)
        };

        let desc = gpu_allocator::vulkan::AllocationCreateDesc {
            name: "placeholder",
            requirements,
            location: gpu_allocator::MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedBuffer(
                self.handle,
            ),
        };

        // don't worry about it.
        let allocation = self.context.alloc_gpu_mem(&desc)?;
        self.allocation = Some(self.context.alloc_gpu_mem(&desc)?);

        unsafe {
            let ptr = self.context.device().handle().map_memory(
                allocation.memory(),
                0,
                self.size,
                ash::vk::MemoryMapFlags::default(),
            )?;

            ptr.copy_from_nonoverlapping(data.as_ptr() as *mut c_void, self.size as usize);

            self.context
                .device()
                .handle()
                .unmap_memory(allocation.memory());
        };

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.context
                .device()
                .handle()
                .destroy_buffer(self.handle, None)
        };
    }
}
