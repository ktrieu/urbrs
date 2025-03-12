use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context as anyhow_context;
use bytemuck::bytes_of;

use crate::vulkan::{
    buffer::Buffer,
    command::{CommandBuffer, CommandPool},
    context::Context,
    device::Device,
    mesh::Vertex,
    pipeline::{Pipeline, PipelineBuilder},
    swapchain::Swapchain,
    sync::{Fence, Semaphore},
    util::{self},
};

const VERTEX_DATA: [Vertex; 8] = [
    Vertex::new_pos(0.0, 0.0, 0.0),
    Vertex::new_pos(1.0, 0.0, 0.0),
    Vertex::new_pos(0.0, -1.0, 0.0),
    Vertex::new_pos(1.0, -1.0, 0.0),
    Vertex::new_pos(0.0, 0.0, 1.0),
    Vertex::new_pos(1.0, 0.0, 1.0),
    Vertex::new_pos(0.0, -1.0, 1.0),
    Vertex::new_pos(1.0, -1.0, 1.0),
];

const INDEX_DATA: [u16; 36] = [
    0, 2, 1, 1, 2, 3, // front
    5, 6, 4, 5, 7, 6, // back
    2, 6, 3, 3, 6, 7, // top
    0, 1, 4, 1, 5, 4, // bottom
    0, 4, 2, 4, 6, 2, // left
    1, 3, 5, 5, 3, 7, // right
];

pub struct Renderer {
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,

    _command_pool: CommandPool,
    command_buffer: CommandBuffer,

    start: Instant,

    render_fence: Fence,
    swap_acquired: Semaphore,
    render_complete: Semaphore,

    graphics_pipeline: Pipeline,

    window_size: winit::dpi::PhysicalSize<u32>,

    // Test...
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Renderer {
    pub fn new(
        context: Arc<Context>,
        swapchain: Arc<Swapchain>,
        window_size: winit::dpi::PhysicalSize<u32>,
    ) -> anyhow::Result<Self> {
        let device = context.device();

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

        let vertex_shader_data = util::read_spirv(Path::new("./data/shader/a.spv.vert"))
            .with_context(|| "failed to read vertex shader a.spv.vert")?;
        let fragment_shader_data = util::read_spirv(Path::new("./data/shader/a.spv.frag"))
            .with_context(|| "failed to read vertex shader a.spv.frag")?;

        let graphics_pipeline = PipelineBuilder::new()
            .with_color_format(swapchain.surface_color_format())
            .with_depth_format(ash::vk::Format::UNDEFINED)
            .with_vertex_shader_data(&vertex_shader_data)
            .with_fragment_shader_data(&fragment_shader_data)
            .with_vertex_layout_info(Vertex::layout())
            .with_push_constants::<glam::Mat4>()
            .build(device.clone())?;

        // test code to upload the buffer...
        let size = Vertex::size() * VERTEX_DATA.len();
        let mut vertex_buffer = Buffer::new(
            context.clone(),
            size,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;

        vertex_buffer.allocate_full()?;
        vertex_buffer.update_mapped_data(&VERTEX_DATA)?;

        let size = size_of::<u16>() * INDEX_DATA.len();
        let mut index_buffer = Buffer::new(
            context.clone(),
            size,
            ash::vk::BufferUsageFlags::INDEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;
        index_buffer.allocate_full()?;
        index_buffer.update_mapped_data(&INDEX_DATA)?;

        Ok(Self {
            device,
            swapchain,
            _command_pool: command_pool,
            command_buffer,
            render_fence,
            swap_acquired,
            render_complete,
            graphics_pipeline,
            vertex_buffer,
            index_buffer,
            window_size,
            start: Instant::now(),
        })
    }

    pub fn render(&self) -> anyhow::Result<()> {
        // Wait one sec for the fence to be available.
        self.render_fence.wait(1_000_000_000)?;
        self.render_fence.reset()?;

        let wnd_width = self.window_size.width as f32;
        let wnd_height = self.window_size.height as f32;

        let projection = glam::Mat4::perspective_rh(
            f32::to_radians(45.0),
            wnd_width / wnd_height,
            0.001,
            1000.0,
        );

        let dt = Instant::now().duration_since(self.start).as_secs_f32();

        let view = glam::Mat4::from_rotation_translation(
            glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                f32::to_radians(45.0),
                f32::to_radians((dt / 10.0).sin() * 360.0).abs(),
                0.0,
            ),
            glam::vec3(-0.5, 0.5, -3.0),
        );

        let model = glam::Mat4::IDENTITY;

        let mvp = projection * view * model;

        self.command_buffer
            .begin(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;

        let swap_image = self.swapchain.acquire_image(&self.swap_acquired)?;

        util::swap_acquire_transition(self.device.clone(), &self.command_buffer, swap_image.image);

        let clear_value = ash::vk::ClearValue::default();

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
            .width(wnd_width)
            // These are little weird so that we can flip the viewport and have Y up,
            // like good old OpenGL.
            .height(-wnd_height)
            .y(wnd_height);

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

            self.device.handle().cmd_bind_vertex_buffers(
                self.command_buffer.handle(),
                0,
                &[self.vertex_buffer.handle()],
                &[0],
            );

            self.device.handle().cmd_bind_index_buffer(
                self.command_buffer.handle(),
                self.index_buffer.handle(),
                0,
                ash::vk::IndexType::UINT16,
            );

            self.device.handle().cmd_push_constants(
                self.command_buffer.handle(),
                self.graphics_pipeline.layout(),
                ash::vk::ShaderStageFlags::ALL_GRAPHICS,
                0,
                bytes_of(&mvp),
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
                .cmd_draw_indexed(self.command_buffer.handle(), 36, 1, 0, 0, 0);

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
