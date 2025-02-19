use std::{fmt::Display, path::Path, sync::Arc, time::Instant};

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
    swapchain: Arc<Swapchain>,

    command_pool: CommandPool,
    command_buffer: CommandBuffer,

    start: Instant,

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
            swapchain,
            command_pool,
            command_buffer,
            start: Instant::now(),
            render_fence,
            swap_acquired,
            render_complete,
            graphics_pipeline,
        })
    }

    pub fn render(&self) -> VkResult<()> {
        // Wait one sec for the fence to be available.
        self.render_fence.wait(1_000_000_000)?;
        self.render_fence.reset()?;

        self.command_buffer
            .begin(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;

        let swap_image = self.swapchain.acquire_image(&self.swap_acquired)?;

        util::swap_acquire_transition(self.device.clone(), &self.command_buffer, swap_image.image);

        let mut clear_value = ash::vk::ClearValue::default();

        let elapsed = self.start.elapsed().as_secs_f32().fract();

        clear_value.color.float32 = [1.0f32, elapsed, 1.0f32, 1.0f32];

        let color_attachment_info = ash::vk::RenderingAttachmentInfo::default()
            .image_view(swap_image.view)
            .image_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .clear_value(clear_value)
            .store_op(ash::vk::AttachmentStoreOp::STORE);

        let color_attachments = &[color_attachment_info];

        let rendering_info = ash::vk::RenderingInfo::default()
            .color_attachments(color_attachments)
            .layer_count(1)
            .render_area(self.swapchain.swap_area());

        let viewport = ash::vk::Viewport::default()
            .max_depth(1.0f32)
            .width(1920.0f32)
            .height(1080.0f32);

        let scissor = self.swapchain.swap_area();

        unsafe {
            self.device
                .handle()
                .cmd_begin_rendering(self.command_buffer.handle(), &rendering_info);

            self.device.handle().cmd_bind_pipeline(
                self.command_buffer.handle(),
                ash::vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.handle(),
            );

            let viewports = &[viewport];
            self.device
                .handle()
                .cmd_set_viewport(self.command_buffer.handle(), 0, viewports);

            let scissors = &[scissor];
            self.device
                .handle()
                .cmd_set_scissor(self.command_buffer.handle(), 0, scissors);

            self.device
                .handle()
                .cmd_draw(self.command_buffer.handle(), 3, 1, 0, 0);

            self.device
                .handle()
                .cmd_end_rendering(self.command_buffer.handle());
        }

        util::swap_present_transition(self.device.clone(), &self.command_buffer, swap_image.image);

        self.command_buffer.end()?;

        let wait_submits = &[self
            .swap_acquired
            .submit_info(ash::vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];

        let signal_submits = &[self
            .render_complete
            .submit_info(ash::vk::PipelineStageFlags2::ALL_COMMANDS)];

        let buffer_submits = &[self.command_buffer.submit_info()];

        let submit_info = ash::vk::SubmitInfo2::default()
            .signal_semaphore_infos(signal_submits)
            .wait_semaphore_infos(wait_submits)
            .command_buffer_infos(buffer_submits);

        let submits = &[submit_info];

        unsafe {
            self.device.handle().queue_submit2(
                self.device.graphics_queue().queue,
                submits,
                self.render_fence.handle(),
            )?
        };

        self.swapchain.present(
            swap_image.idx,
            self.device.present_queue(),
            &self.render_complete,
        )?;

        Ok(())
    }
}
