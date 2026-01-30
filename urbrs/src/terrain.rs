use std::sync::Arc;

use crate::vulkan::{buffer::Buffer, context::Context};

pub struct Heightmap {
    size: i64,
    data: Vec<f32>,
}

impl Heightmap {
    pub fn new(size: i64) -> Self {
        let length = size.pow(2);

        let mut data = Vec::new();
        data.resize(length as usize, 0.0);

        Self { size, data }
    }

    // Sample the heightmap at a point. Works like a texture, [0,1] on both axes.
    pub fn sample(&self, pos: glam::Vec2) -> Option<f32> {
        // Nearest-neighbor, round the sampling vector.
        let scaled = pos * self.size as f32;
        if scaled.x < 0.0 || scaled.x > 1.0 || scaled.y < 0.0 || scaled.y > 1.0 {
            return None;
        }

        let x = scaled.x.round() as i64;
        let y = scaled.y.round() as i64;

        let idx = y * self.size + x;
        self.data.get(idx as usize).copied()
    }
}

struct TerrainVertex {
    position: glam::Vec3,
    normal: glam::Vec3,
}

pub struct Terrain {
    heightmap: Heightmap,

    chunk_size: i64,
    terrain_chunks: i64,

    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

// One chunk = 128x128 vertices.
const CHUNK_SIZE: i64 = 128;
// 16 chunks across... for now.
const TERRAIN_CHUNKS: i64 = 16;

impl Terrain {
    const HEIGHTMAP_RESOLUTION: i64 = 1024;

    pub fn new(
        context: Arc<Context>,
        chunk_size: i64,
        terrain_chunks: i64,
    ) -> anyhow::Result<Self> {
        let heightmap = Heightmap::new(Self::HEIGHTMAP_RESOLUTION);

        let num_chunks = terrain_chunks.pow(2);

        let vertices_per_chunk = chunk_size.pow(2);
        let total_vertices = vertices_per_chunk * num_chunks;

        let vertex_buffer_size = size_of::<TerrainVertex>() * total_vertices as usize;
        let vertex_buffer = Buffer::new(
            context.clone(),
            vertex_buffer_size,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;

        // Need to fill each "gap" between vertices with triangles
        // (n -  1) gaps and then two tris per gap.
        let tris_per_chunk = (chunk_size - 1).pow(2) * 2;
        let indices_per_chunk = tris_per_chunk * 3;
        let total_indices = indices_per_chunk * num_chunks;

        let index_buffer_size = size_of::<u32>() * total_indices as usize;
        let index_buffer = Buffer::new(
            context.clone(),
            index_buffer_size,
            ash::vk::BufferUsageFlags::INDEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;

        Ok(Self {
            heightmap,
            chunk_size,
            terrain_chunks,
            vertex_buffer,
            index_buffer,
        })
    }
}
