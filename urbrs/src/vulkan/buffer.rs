use std::{ffi::c_void, ptr::NonNull, sync::Arc};

use super::{context::Context, mesh::Vertex};

pub struct Buffer {
    context: Arc<Context>,
    size: usize,
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
            size,
            allocation: None,
        })
    }

    pub fn handle(&self) -> ash::vk::Buffer {
        self.handle
    }

    // Quick and dirty - send the data direct to the GPU after allocating the memory.
    pub fn upload_direct(&mut self, data: &[Vertex]) -> anyhow::Result<()> {
        let data_size = data.len() * Vertex::size();

        if data_size != self.size {
            return Err(anyhow::anyhow!("data size did not match buffer size"));
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

        unsafe {
            self.context.device().handle().bind_buffer_memory(
                self.handle,
                allocation.memory(),
                allocation.offset(),
            )?;

            // Our buffer is host visible because we just asked for it.
            let ptr = allocation.mapped_ptr().unwrap();
            ptr.copy_from_nonoverlapping(
                // And our data pointer is not null because it comes from a valid slice.
                NonNull::new_unchecked(data.as_ptr() as *mut c_void),
                self.size as usize,
            );
        };

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
