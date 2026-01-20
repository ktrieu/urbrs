use std::sync::Arc;

use crate::vulkan::descriptor::DescriptorSetLayout;

use super::{device::Device, mesh::VertexLayoutInfo};

struct ShaderModule {
    device: Arc<Device>,
    handle: ash::vk::ShaderModule,
    stage_flags: ash::vk::ShaderStageFlags,
}

impl ShaderModule {
    fn new(
        device: Arc<Device>,
        data: &Vec<u32>,
        stage_flags: ash::vk::ShaderStageFlags,
    ) -> anyhow::Result<Self> {
        let create_info = ash::vk::ShaderModuleCreateInfo::default().code(data.as_slice());
        let handle = unsafe { device.handle().create_shader_module(&create_info, None)? };

        Ok(Self {
            device,
            handle,
            stage_flags,
        })
    }

    fn shader_stage_create_info(&'_ self) -> ash::vk::PipelineShaderStageCreateInfo<'_> {
        ash::vk::PipelineShaderStageCreateInfo::default()
            .module(self.handle)
            .name(c"main")
            .stage(self.stage_flags)
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device
                .handle()
                .destroy_shader_module(self.handle, None);
        };
    }
}

pub struct Pipeline {
    device: Arc<Device>,
    handle: ash::vk::Pipeline,
    layout: ash::vk::PipelineLayout,
    _descriptor_layouts: Vec<Arc<DescriptorSetLayout>>,
}

impl Pipeline {
    pub fn handle(&self) -> ash::vk::Pipeline {
        self.handle
    }

    pub fn layout(&self) -> ash::vk::PipelineLayout {
        self.layout
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .handle()
                .destroy_pipeline_layout(self.layout, None);
            self.device.handle().destroy_pipeline(self.handle, None);
        }
    }
}

pub struct PipelineBuilder<'s> {
    vertex_shader_data: Option<&'s Vec<u32>>,
    fragment_shader_data: Option<&'s Vec<u32>>,

    color_format: Option<ash::vk::Format>,
    depth_format: Option<ash::vk::Format>,

    push_constant_range: Option<ash::vk::PushConstantRange>,

    vertex_layout_info: Option<VertexLayoutInfo>,
    descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,
}

impl<'s> PipelineBuilder<'s> {
    pub fn new() -> Self {
        Self {
            vertex_shader_data: None,
            fragment_shader_data: None,
            color_format: None,
            depth_format: None,
            vertex_layout_info: None,
            push_constant_range: None,
            descriptor_set_layouts: Vec::new(),
        }
    }

    pub fn with_vertex_shader_data(self, data: &'s Vec<u32>) -> Self {
        Self {
            vertex_shader_data: Some(data),
            ..self
        }
    }

    pub fn with_fragment_shader_data(self, data: &'s Vec<u32>) -> Self {
        Self {
            fragment_shader_data: Some(data),
            ..self
        }
    }

    pub fn with_color_format(self, format: ash::vk::Format) -> Self {
        Self {
            color_format: Some(format),
            ..self
        }
    }

    pub fn with_depth_format(self, format: ash::vk::Format) -> Self {
        Self {
            depth_format: Some(format),
            ..self
        }
    }

    pub fn with_vertex_layout_info(self, info: VertexLayoutInfo) -> Self {
        Self {
            vertex_layout_info: Some(info),
            ..self
        }
    }

    pub fn with_push_constants<T>(self) -> Self {
        let size = size_of::<T>();

        let range = ash::vk::PushConstantRange::default()
            .offset(0)
            .size(size as u32)
            .stage_flags(ash::vk::ShaderStageFlags::ALL_GRAPHICS);

        Self {
            push_constant_range: Some(range),
            ..self
        }
    }

    pub fn with_descriptor_set_layouts(self, layout: &[Arc<DescriptorSetLayout>]) -> Self {
        Self {
            descriptor_set_layouts: Vec::from(layout),
            ..self
        }
    }

    pub fn build(self, device: Arc<Device>) -> anyhow::Result<Pipeline> {
        let vertex_shader_data = self
            .vertex_shader_data
            .ok_or(anyhow::anyhow!("no vertex shader specified"))?;

        let fragment_shader_data = self
            .fragment_shader_data
            .ok_or(anyhow::anyhow!("no fragment shader specified"))?;

        let vertex_shader = ShaderModule::new(
            device.clone(),
            &vertex_shader_data,
            ash::vk::ShaderStageFlags::VERTEX,
        )?;

        let fragment_shader = ShaderModule::new(
            device.clone(),
            &fragment_shader_data,
            ash::vk::ShaderStageFlags::FRAGMENT,
        )?;

        // Don't initialize this - we'll leave it as dynamic state.
        let viewport_info = ash::vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let vertex_input_info = if let Some(vertex_layout_info) = &self.vertex_layout_info {
            ash::vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&vertex_layout_info.bindings)
                .vertex_attribute_descriptions(&vertex_layout_info.descs)
        } else {
            ash::vk::PipelineVertexInputStateCreateInfo::default()
        };

        let color_attachment = ash::vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(ash::vk::ColorComponentFlags::RGBA)
            .blend_enable(false);

        let attachments = &[color_attachment];
        let color_blend_info = ash::vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(ash::vk::LogicOp::COPY)
            .attachments(attachments);

        let input_assembly_info = ash::vk::PipelineInputAssemblyStateCreateInfo::default()
            .primitive_restart_enable(false)
            .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST);

        let raster_info = ash::vk::PipelineRasterizationStateCreateInfo::default()
            .cull_mode(ash::vk::CullModeFlags::NONE)
            .front_face(ash::vk::FrontFace::CLOCKWISE)
            .polygon_mode(ash::vk::PolygonMode::FILL)
            .line_width(1.0f32);

        let multisample_info = ash::vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(ash::vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0f32)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let depth_info = ash::vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(ash::vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .front(ash::vk::StencilOpState::default())
            .back(ash::vk::StencilOpState::default())
            .min_depth_bounds(0.0f32)
            .max_depth_bounds(1.0f32);

        let color_format = self
            .color_format
            .ok_or(anyhow::anyhow!("no color format specified!"))?;
        let depth_format = self
            .depth_format
            .ok_or(anyhow::anyhow!("no depth format specified"))?;

        let color_formats = &[color_format];
        let mut rendering_info = ash::vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(color_formats)
            .depth_attachment_format(depth_format);

        let dynamic_info = ash::vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR,
        ]);

        let mut push_constant_ranges: Vec<ash::vk::PushConstantRange> = Vec::new();
        if let Some(range) = self.push_constant_range {
            push_constant_ranges.push(range);
        }

        let layouts: Vec<ash::vk::DescriptorSetLayout> = self
            .descriptor_set_layouts
            .iter()
            .map(|l| l.handle())
            .collect();

        let layout_info = ash::vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(layouts.as_slice());

        let layout = unsafe { device.handle().create_pipeline_layout(&layout_info, None)? };

        let vertex_shader_info = vertex_shader.shader_stage_create_info();
        let fragment_shader_info = fragment_shader.shader_stage_create_info();
        let stages = &[vertex_shader_info, fragment_shader_info];

        let info = ash::vk::GraphicsPipelineCreateInfo::default()
            .stages(stages)
            .viewport_state(&viewport_info)
            .vertex_input_state(&vertex_input_info)
            .color_blend_state(&color_blend_info)
            .input_assembly_state(&input_assembly_info)
            .rasterization_state(&raster_info)
            .multisample_state(&multisample_info)
            .depth_stencil_state(&depth_info)
            .dynamic_state(&dynamic_info)
            .layout(layout)
            .push_next(&mut rendering_info);

        let pipelines_result = unsafe {
            device
                .handle()
                .create_graphics_pipelines(ash::vk::PipelineCache::null(), &[info], None)
        };

        // For now only assume we're making one pipeline, and unpack the odd format of the result.
        let handle = match pipelines_result {
            Ok(pipelines) => Ok(pipelines[0]),
            Err(pipelines) => Err(pipelines.1),
        }?;

        return Ok(Pipeline {
            device,
            layout,
            handle,
            _descriptor_layouts: self.descriptor_set_layouts,
        });
    }
}
