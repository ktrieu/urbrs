use std::{marker::PhantomData, sync::Arc};

use gpu_allocator::vulkan::AllocationCreateDesc;

use crate::vulkan::{buffer::Buffer, context::Context};

pub struct UniformBuffer<T> {
    buffer: Buffer,
    phantom: PhantomData<T>,
}

impl<T> UniformBuffer<T> {
    pub fn new(
        context: Arc<Context>,
        count: usize,
        sharing_mode: ash::vk::SharingMode,
        name: Option<&str>,
    ) -> anyhow::Result<Self> {
        let bytes = count * size_of::<T>();
        let mut buffer = Buffer::new(
            context.clone(),
            bytes,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode,
        )?;

        buffer.allocate(AllocationCreateDesc {
            name: name.unwrap_or("uniform buffer (unnamed)"),
            requirements: buffer.memory_requirements(),
            // Uniform buffers we'll just write to directly, no need for a staging buffer.
            location: gpu_allocator::MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        })?;

        Ok(Self {
            buffer,
            phantom: PhantomData,
        })
    }
}
