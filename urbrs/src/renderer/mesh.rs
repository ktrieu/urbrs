use std::sync::Arc;

use common::{Model, Vertex};
use gpu_allocator::vulkan::AllocationCreateDesc;

use crate::vulkan::{buffer::Buffer, command::CommandBuffer, context::Context, device::Device};

pub struct Mesh {
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    num_vertices: usize,
    num_indices: usize,
}

impl Mesh {
    pub fn new_from_model(context: Arc<Context>, model: &Model) -> anyhow::Result<Self> {
        let num_vertices = model.vertices.len();
        let num_indices = model.indices.len();

        let vertex_buffer_size = size_of::<Vertex>() * num_vertices;
        let mut vertex_buffer = Buffer::new(
            context.clone(),
            vertex_buffer_size,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;
        let name = format!("{} vertex buffer", model.name);
        let vertex_alloc_desc = AllocationCreateDesc {
            name: name.as_str(),
            requirements: vertex_buffer.memory_requirements(),
            location: gpu_allocator::MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        };
        vertex_buffer.allocate(vertex_alloc_desc)?;
        let mut slab = vertex_buffer
            .allocation_mut()
            .map(|a| a.try_as_mapped_slab())
            .flatten()
            .expect("vertex buffer should be valid mapped slab");
        presser::copy_from_slice_to_offset(model.vertices.as_slice(), &mut slab, 0)?;

        let index_buffer_size = size_of::<u32>() * num_indices;
        let mut index_buffer = Buffer::new(
            context.clone(),
            index_buffer_size,
            ash::vk::BufferUsageFlags::INDEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;
        let name = format!("{} index buffer", model.name);
        let index_alloc_desc = AllocationCreateDesc {
            name: name.as_str(),
            requirements: index_buffer.memory_requirements(),
            location: gpu_allocator::MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        };
        index_buffer.allocate(index_alloc_desc)?;
        let mut slab = index_buffer
            .allocation_mut()
            .map(|a| a.try_as_mapped_slab())
            .flatten()
            .expect("index buffer should be valid mapped slab");

        presser::copy_from_slice_to_offset(model.indices.as_slice(), &mut slab, 0)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            num_vertices,
            num_indices,
        })
    }

    pub fn bind(&self, device: Arc<Device>, cmd_buffer: &CommandBuffer) {
        unsafe {
            device.handle().cmd_bind_vertex_buffers(
                cmd_buffer.handle(),
                0,
                &[self.vertex_buffer.handle()],
                &[0],
            )
        };

        unsafe {
            device.handle().cmd_bind_index_buffer(
                cmd_buffer.handle(),
                self.index_buffer.handle(),
                0,
                ash::vk::IndexType::UINT32,
            )
        };
    }

    pub fn num_indices(&self) -> usize {
        self.num_indices
    }
}
