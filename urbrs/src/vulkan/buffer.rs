use std::{ffi::c_void, ptr::NonNull, sync::Arc};

use super::context::Context;

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

    // Make one allocation for the entire buffer. Not very clever - but we're just testing stuff right now.
    pub fn allocate_full(&mut self) -> anyhow::Result<()> {
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

    // Quick and dirty - send the data direct to the GPU after allocating the memory.
    pub fn update_mapped_data<T>(&mut self, data: &[T]) -> anyhow::Result<()> {
        let data_size = data.len() * size_of::<T>();

        if data_size != self.size {
            return Err(anyhow::anyhow!("data size did not match buffer size"));
        }

        let allocation = self
            .allocation
            .as_ref()
            .ok_or(anyhow::anyhow!("cannot update data for unallocated buffer"))?;

        unsafe {
            // Our buffer is host visible because we just asked for it.
            let ptr = allocation.mapped_ptr().unwrap();
            ptr.copy_from_nonoverlapping(
                // And our data pointer is not null because it comes from a valid slice.
                NonNull::new_unchecked(data.as_ptr() as *mut c_void),
                self.size as usize,
            );
        };

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
