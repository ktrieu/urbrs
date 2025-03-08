use std::mem::offset_of;

// Make this customizable or something later.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub color: glam::Vec3,
}

pub struct VertexLayoutInfo {
    pub descs: Vec<ash::vk::VertexInputAttributeDescription>,
    pub bindings: Vec<ash::vk::VertexInputBindingDescription>,
}

impl Vertex {
    pub fn layout() -> VertexLayoutInfo {
        let bindings = vec![ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)];

        let descs = vec![
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, position) as u32),
            ash::vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32),
        ];

        VertexLayoutInfo { descs, bindings }
    }

    pub fn size() -> usize {
        size_of::<Self>()
    }
}
