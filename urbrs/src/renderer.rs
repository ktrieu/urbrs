use std::{fmt::Display, path::Path, sync::Arc};

use ash::prelude::VkResult;

use crate::vulkan::{
    command::{CommandBuffer, CommandPool},
    device::Device,
    pipeline::{Pipeline, PipelineBuildError, PipelineBuilder},
    swapchain::Swapchain,
    sync::{Fence, Semaphore},
    util::{self, SpirvReadError},
};

pub struct Renderer {
    device: Arc<Device>,
    command_pool: CommandPool,
    command_buffer: CommandBuffer,

    render_fence: Fence,
    swap_acquired: Semaphore,
    render_complete: Semaphore,

    graphics_pipeline: Pipeline,
}

#[derive(Debug)]
pub enum RendererCreateError {
    PipelineError(PipelineBuildError),
    VkError(ash::vk::Result),
    SpirvError(SpirvReadError),
}

impl From<PipelineBuildError> for RendererCreateError {
    fn from(value: PipelineBuildError) -> Self {
        RendererCreateError::PipelineError(value)
    }
}

impl From<ash::vk::Result> for RendererCreateError {
    fn from(value: ash::vk::Result) -> Self {
        RendererCreateError::VkError(value)
    }
}

impl From<SpirvReadError> for RendererCreateError {
    fn from(value: SpirvReadError) -> Self {
        RendererCreateError::SpirvError(value)
    }
}

impl Display for RendererCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererCreateError::PipelineError(err) => write!(f, "pipeline build error: {}", err),
            RendererCreateError::VkError(err) => write!(f, "vulkan error: {}", err),
            RendererCreateError::SpirvError(err) => write!(f, "SPIR-V read error: {}", err),
        }
    }
}

impl Renderer {
    pub fn new(
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
    ) -> Result<Self, RendererCreateError> {
        let command_pool = CommandPool::new(
            device.clone(),
            device.graphics_queue(),
            ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )?;

        let command_buffer = CommandBuffer::new(device.clone(), &command_pool)?;
        let render_fence = Fence::new(device.clone(), ash::vk::FenceCreateFlags::SIGNALED)?;

        let swap_acquired = Semaphore::new(device.clone(), ash::vk::SemaphoreCreateFlags::empty())?;
        let render_complete =
            Semaphore::new(device.clone(), ash::vk::SemaphoreCreateFlags::empty())?;

        let vertex_shader_data = util::read_spirv(Path::new("./data/shader/a.spv.vert"))?;
        let fragment_shader_data = util::read_spirv(Path::new("./data/shader/a.spv.frag"))?;

        let graphics_pipeline = PipelineBuilder::new()
            .with_color_format(swapchain.surface_color_format())
            .with_depth_format(ash::vk::Format::UNDEFINED)
            .with_vertex_shader_data(&vertex_shader_data)
            .with_fragment_shader_data(&fragment_shader_data)
            .build(device.clone())?;

        Ok(Self {
            device,
            command_pool,
            command_buffer,
            render_fence,
            swap_acquired,
            render_complete,
            graphics_pipeline,
        })
    }
}
