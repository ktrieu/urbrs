use std::{fs::File, io::Read, ops::Rem, path::Path, sync::Arc, time::Instant};

use anyhow::{anyhow, Context as anyhow_context};
use bytemuck::bytes_of;
use common::{Model, Vertex};
use rkyv::rancor;

use crate::{
    camera::Camera,
    vulkan::{
        buffer::Buffer,
        command::{CommandBuffer, CommandPool},
        context::Context,
        device::Device,
        mesh::MeshVertex,
        phys_device::PhysicalDevice,
        pipeline::{Pipeline, PipelineBuilder},
        swapchain::Swapchain,
        sync::{Fence, Semaphore},
        util::{self},
    },
};

struct DepthBuffer {
    context: Arc<Context>,
    image: ash::vk::Image,
    allocation: gpu_allocator::vulkan::Allocation,
    image_view: ash::vk::ImageView,
    format: ash::vk::Format,
}

impl DepthBuffer {
    const DESIRED_DEPTH_FORMATS: [ash::vk::Format; 3] = [
        ash::vk::Format::D32_SFLOAT,
        ash::vk::Format::D32_SFLOAT_S8_UINT,
        ash::vk::Format::D24_UNORM_S8_UINT,
    ];

    fn select_depth_format(physical_device: &PhysicalDevice) -> Option<ash::vk::Format> {
        for format in Self::DESIRED_DEPTH_FORMATS {
            let props = physical_device.get_format_properties(format);
            if props
                .optimal_tiling_features
                .contains(ash::vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                return Some(format);
            }
        }

        None
    }

    pub fn new(context: Arc<Context>, width: u32, height: u32) -> anyhow::Result<Self> {
        let depth_format = Self::select_depth_format(context.device().physical_device())
            .ok_or(anyhow::anyhow!("no valid depth format found"))?;

        let image_create_info = ash::vk::ImageCreateInfo::default()
            .image_type(ash::vk::ImageType::TYPE_2D)
            .extent(
                ash::vk::Extent3D::default()
                    .width(width)
                    .height(height)
                    .depth(1),
            )
            .mip_levels(1)
            .array_layers(1)
            .format(depth_format)
            .tiling(ash::vk::ImageTiling::OPTIMAL)
            .usage(ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .samples(ash::vk::SampleCountFlags::TYPE_1);

        unsafe {
            let image = context
                .device()
                .handle()
                .create_image(&image_create_info, None)?;

            let mem_reqs = context
                .device()
                .handle()
                .get_image_memory_requirements(image);

            let allocation =
                context.alloc_gpu_mem(&gpu_allocator::vulkan::AllocationCreateDesc {
                    name: "depth_buffer",
                    requirements: mem_reqs,
                    location: gpu_allocator::MemoryLocation::GpuOnly,
                    linear: false,
                    allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedImage(
                        image,
                    ),
                })?;

            context.device().handle().bind_image_memory(
                image,
                allocation.memory(),
                allocation.offset(),
            )?;

            let mapping = ash::vk::ComponentMapping::default()
                .r(ash::vk::ComponentSwizzle::IDENTITY)
                .g(ash::vk::ComponentSwizzle::IDENTITY)
                .b(ash::vk::ComponentSwizzle::IDENTITY)
                .a(ash::vk::ComponentSwizzle::IDENTITY);

            let range = ash::vk::ImageSubresourceRange::default()
                .aspect_mask(ash::vk::ImageAspectFlags::DEPTH)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let image_view_info = ash::vk::ImageViewCreateInfo::default()
                .image(image)
                .format(depth_format)
                .view_type(ash::vk::ImageViewType::TYPE_2D)
                .components(mapping)
                .subresource_range(range);

            let image_view = context
                .device()
                .handle()
                .create_image_view(&image_view_info, None)?;

            Ok(Self {
                context,
                image,
                allocation,
                image_view,
                format: depth_format,
            })
        }
    }
}

impl Drop for DepthBuffer {
    fn drop(&mut self) {
        let allocation = std::mem::take(&mut self.allocation);
        self.context.free_gpu_mem(allocation).unwrap();

        unsafe {
            self.context
                .device()
                .handle()
                .destroy_image_view(self.image_view, None);

            self.context
                .device()
                .handle()
                .destroy_image(self.image, None);
        };
    }
}

struct Frame {
    render_fence: Fence,
    swap_acquired: Semaphore,
    render_complete: Semaphore,

    command_buffer: CommandBuffer,
}

struct FrameBeginResult<'frame> {
    command_buffer: &'frame CommandBuffer,
    swap_image_idx: u32,
}

impl Frame {
    fn new(device: Arc<Device>, command_pool: &CommandPool) -> anyhow::Result<Self> {
        let command_buffer = CommandBuffer::new(device.clone(), command_pool)?;
        let render_fence = Fence::new(device.clone(), ash::vk::FenceCreateFlags::SIGNALED)?;

        let swap_acquired = Semaphore::new(device.clone(), ash::vk::SemaphoreCreateFlags::empty())?;
        let render_complete =
            Semaphore::new(device.clone(), ash::vk::SemaphoreCreateFlags::empty())?;

        Ok(Self {
            render_fence: render_fence,
            swap_acquired: swap_acquired,
            render_complete: render_complete,
            command_buffer: command_buffer,
        })
    }

    fn begin(
        &'_ self,
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
        depth_buffer: &DepthBuffer,
        window_size: winit::dpi::PhysicalSize<u32>,
    ) -> anyhow::Result<FrameBeginResult<'_>> {
        let wnd_width = window_size.width as f32;
        let wnd_height = window_size.height as f32;

        // Wait one sec for the fence to be available.
        self.render_fence.wait(1_000_000_000)?;
        self.render_fence.reset()?;

        self.command_buffer
            .begin(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;

        let swap_image = swapchain.acquire_image(&self.swap_acquired)?;

        util::swap_acquire_transition(device.clone(), &self.command_buffer, swap_image.image);

        let color_clear_value = ash::vk::ClearValue::default();
        let mut depth_clear = ash::vk::ClearValue::default();
        depth_clear.depth_stencil = ash::vk::ClearDepthStencilValue::default().depth(1.0);

        let color_attachment_info = ash::vk::RenderingAttachmentInfo::default()
            .image_view(swap_image.view)
            .image_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .clear_value(color_clear_value)
            .store_op(ash::vk::AttachmentStoreOp::STORE);

        let color_attachments = &[color_attachment_info];

        let depth_attachment_info = ash::vk::RenderingAttachmentInfo::default()
            .image_view(depth_buffer.image_view)
            .image_layout(ash::vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .clear_value(depth_clear)
            .store_op(ash::vk::AttachmentStoreOp::STORE);

        let rendering_info = ash::vk::RenderingInfo::default()
            .color_attachments(color_attachments)
            .layer_count(1)
            .depth_attachment(&depth_attachment_info)
            .render_area(swapchain.swap_area());

        let viewport = ash::vk::Viewport::default()
            .max_depth(1.0f32)
            .width(wnd_width)
            // These are little weird so that we can flip the viewport and have Y up,
            // like good old OpenGL.
            .height(-wnd_height)
            .y(wnd_height);

        let scissor = swapchain.swap_area();
        let viewports = &[viewport];

        unsafe {
            device
                .handle()
                .cmd_begin_rendering(self.command_buffer.handle(), &rendering_info);

            device
                .handle()
                .cmd_set_viewport(self.command_buffer.handle(), 0, viewports);

            let scissors = &[scissor];
            device
                .handle()
                .cmd_set_scissor(self.command_buffer.handle(), 0, scissors);
        }

        Ok(FrameBeginResult {
            command_buffer: &self.command_buffer,
            swap_image_idx: swap_image.idx,
        })
    }

    fn end(
        &self,
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
        begin_result: FrameBeginResult,
    ) -> anyhow::Result<()> {
        let swap_image = swapchain
            .get_image(begin_result.swap_image_idx)
            .ok_or(anyhow!("swap image not found"))?;

        unsafe {
            device
                .handle()
                .cmd_end_rendering(self.command_buffer.handle());
        }

        util::swap_present_transition(device.clone(), &self.command_buffer, swap_image.image);

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
            device.handle().queue_submit2(
                device.graphics_queue().queue,
                submits,
                self.render_fence.handle(),
            )?
        };

        swapchain.present(
            swap_image.idx,
            device.present_queue(),
            &self.render_complete,
        )?;

        Ok(())
    }
}

const FRAMES_IN_FLIGHT: usize = 3;

pub struct Renderer {
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,

    _command_pool: CommandPool,
    graphics_pipeline: Pipeline,

    camera: Camera,

    start: Instant,
    window_size: winit::dpi::PhysicalSize<u32>,

    vertex_buffer: Buffer,
    index_buffer: Buffer,

    depth_buffer: DepthBuffer,

    frames: Vec<Frame>,
    frame_idx: usize,

    // some hack code to get model rendering working
    num_indices: u32,
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

        let vertex_shader_data = util::read_spirv(Path::new("./data/shader/a.spv.vert"))
            .with_context(|| "failed to read vertex shader a.spv.vert")?;
        let fragment_shader_data = util::read_spirv(Path::new("./data/shader/a.spv.frag"))
            .with_context(|| "failed to read vertex shader a.spv.frag")?;

        let depth_buffer = DepthBuffer::new(
            context.clone(),
            swapchain.extent().width,
            swapchain.extent().height,
        )?;

        let graphics_pipeline = PipelineBuilder::new()
            .with_color_format(swapchain.surface_color_format())
            .with_depth_format(depth_buffer.format)
            .with_vertex_shader_data(&vertex_shader_data)
            .with_fragment_shader_data(&fragment_shader_data)
            .with_vertex_layout_info(Vertex::layout())
            .with_push_constants::<glam::Mat4>()
            .build(device.clone())?;

        let mut bytes: Vec<u8> = Vec::new();
        File::open(Path::new("./data/models/jerma.mdl"))?.read_to_end(&mut bytes)?;
        let archived_model = rkyv::from_bytes::<Model, rancor::Error>(&bytes)?;

        let vert_size = Vertex::size() * archived_model.vertices.len();

        let mut vertex_buffer = Buffer::new(
            context.clone(),
            vert_size,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;

        vertex_buffer.allocate_full()?;
        vertex_buffer.update_mapped_data(&archived_model.vertices)?;

        let index_size = size_of::<u32>() * archived_model.indices.len();
        let mut index_buffer = Buffer::new(
            context.clone(),
            index_size,
            ash::vk::BufferUsageFlags::INDEX_BUFFER,
            ash::vk::SharingMode::EXCLUSIVE,
        )?;
        index_buffer.allocate_full()?;
        index_buffer.update_mapped_data(&archived_model.indices)?;

        let frames = (0..FRAMES_IN_FLIGHT)
            .map(|_| Frame::new(device.clone(), &command_pool))
            .collect::<anyhow::Result<Vec<Frame>>>()?;

        let camera = Camera::new(
            glam::vec2(window_size.width as f32, window_size.height as f32),
            f32::to_radians(45.0),
        );

        Ok(Self {
            device,
            swapchain,
            frames,
            frame_idx: 0,
            camera,
            _command_pool: command_pool,
            graphics_pipeline,
            vertex_buffer,
            index_buffer,
            window_size,
            start: Instant::now(),
            depth_buffer,
            num_indices: archived_model.indices.len() as u32,
        })
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let dt = Instant::now().duration_since(self.start).as_secs_f32();

        let pitch = f32::to_radians(-45.0);
        let yaw = f32::to_radians((dt * 20.0) % 360.0);

        self.camera
            .set_arcball(glam::vec3(0.5, 0.5, 0.5), glam::vec2(pitch, yaw), 50.0);

        // Identity model matrix for now.
        let mvp = self.camera.vp();

        let frame = self
            .frames
            .get_mut(self.frame_idx)
            .ok_or(anyhow!("invalid frame idx {}", self.frame_idx))?;

        let begin_result = frame.begin(
            self.device.clone(),
            self.swapchain.clone(),
            &self.depth_buffer,
            self.window_size,
        )?;

        unsafe {
            let command_buffer = begin_result.command_buffer;
            self.device.handle().cmd_bind_pipeline(
                command_buffer.handle(),
                ash::vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.handle(),
            );

            self.device.handle().cmd_bind_vertex_buffers(
                command_buffer.handle(),
                0,
                &[self.vertex_buffer.handle()],
                &[0],
            );

            self.device.handle().cmd_bind_index_buffer(
                command_buffer.handle(),
                self.index_buffer.handle(),
                0,
                ash::vk::IndexType::UINT32,
            );

            self.device.handle().cmd_push_constants(
                command_buffer.handle(),
                self.graphics_pipeline.layout(),
                ash::vk::ShaderStageFlags::ALL_GRAPHICS,
                0,
                bytes_of(&mvp),
            );

            self.device.handle().cmd_draw_indexed(
                command_buffer.handle(),
                self.num_indices,
                1,
                0,
                0,
                0,
            );
        }

        frame.end(self.device.clone(), self.swapchain.clone(), begin_result)?;

        self.frame_idx = (self.frame_idx + 1).rem(FRAMES_IN_FLIGHT);

        Ok(())
    }
}
