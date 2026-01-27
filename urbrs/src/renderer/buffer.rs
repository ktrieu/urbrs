use std::{marker::PhantomData, sync::Arc};

use gpu_allocator::vulkan::AllocationCreateDesc;

use crate::vulkan::{buffer::Buffer, context::Context};

pub struct UniformBuffer<T> {
    buffer: Buffer,

    count: usize,
    stride: usize,
    min_align: usize,

    phantom: PhantomData<T>,
}

fn get_stride<T>(struct_size: usize, min_align: usize) -> usize {
    // Get the actual buffer size for a uniform buffer of count elements of T.
    // Account for the minimum per-element alignment imposed by Vulkan
    // for uniform buffers.
    (struct_size + min_align - 1) & !(min_align - 1)
}

impl<T: Copy> UniformBuffer<T> {
    pub fn new(
        context: Arc<Context>,
        count: usize,
        sharing_mode: ash::vk::SharingMode,
        name: Option<&str>,
    ) -> anyhow::Result<Self> {
        let min_align = context
            .device()
            .physical_device()
            .limits()
            .min_uniform_buffer_offset_alignment;

        let min_align = min_align
            .try_into()
            .expect("uniform buffer minimum alignment should convert into u64");

        let stride = get_stride::<T>(size_of::<T>(), min_align);

        let size = stride * count;

        let mut buffer = Buffer::new(
            context.clone(),
            size,
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
            count,
            stride,
            min_align,
            phantom: PhantomData,
        })
    }

    pub fn write(&mut self, data: T, idx: usize) -> anyhow::Result<()> {
        assert!(idx < self.count);

        let allocation = self
            .buffer
            .allocation_mut()
            .expect("buffer must be allocated before write");

        let mut slab = allocation
            .try_as_mapped_slab()
            .expect("allocation must be valid slab");

        let offset = self.stride * idx;

        presser::copy_to_offset_with_align(&data, &mut slab, offset, self.min_align)?;

        Ok(())
    }

    pub fn descriptor_info(&self, idx: usize) -> ash::vk::DescriptorBufferInfo {
        ash::vk::DescriptorBufferInfo::default()
            .buffer(self.buffer.handle())
            .offset((idx * self.stride) as u64)
            .range(size_of::<T>() as u64)
    }
}
