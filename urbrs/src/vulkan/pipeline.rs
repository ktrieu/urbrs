use std::{fmt::Display, sync::Arc};

use ash::prelude::VkResult;

use super::device::Device;

pub struct Pipeline {
    device: Arc<Device>,
    handle: ash::vk::Pipeline,
}

pub enum PipelineBuildError {
    NoVertexShader,
    NoFragmentShader,
    NoColorFormat,
    NoDepthFormat,
    VulkanError(ash::vk::Result),
}

impl Display for PipelineBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineBuildError::NoVertexShader => {
                write!(f, "no vertex shader specified for the pipeline")
            }
            PipelineBuildError::NoFragmentShader => {
                write!(f, "no fragment shader specified for the pipeline")
            }
            PipelineBuildError::NoColorFormat => {
                write!(f, "no color format specified for the pipeline")
            }
            PipelineBuildError::NoDepthFormat => {
                write!(f, "no depth format specified for the pipeline")
            }
            PipelineBuildError::VulkanError(vk_err) => {
                write!(f, "vulkan error {vk_err}")
            }
        }
    }
}

impl From<ash::vk::Result> for PipelineBuildError {
    fn from(value: ash::vk::Result) -> Self {
        PipelineBuildError::VulkanError(value)
    }
}

pub struct PipelineBuilder<'s> {
    vertex_shader_data: Option<&'s Vec<u32>>,
    fragment_shader_data: Option<&'s Vec<u32>>,

    color_format: Option<ash::vk::Format>,
    depth_format: Option<ash::vk::Format>,
}

impl<'s> PipelineBuilder<'s> {
    pub fn new() -> Self {
        Self {
            vertex_shader_data: None,
            fragment_shader_data: None,
            color_format: None,
            depth_format: None,
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

    fn create_shader_module(
        device: Arc<Device>,
        data: &Vec<u32>,
    ) -> VkResult<ash::vk::ShaderModule> {
        let info = ash::vk::ShaderModuleCreateInfo::default().code(data.as_slice());

        unsafe { device.handle().create_shader_module(&info, None) }
    }

    fn create_shader_stage_info<'a>(
        module: ash::vk::ShaderModule,
        flags: ash::vk::ShaderStageFlags,
    ) -> ash::vk::PipelineShaderStageCreateInfo<'a> {
        ash::vk::PipelineShaderStageCreateInfo::default()
            .module(module)
            .name(c"main")
            .stage(flags)
    }

    pub fn build(self, device: Arc<Device>) -> Result<Pipeline, PipelineBuildError> {
        let vertex_shader_data = self
            .vertex_shader_data
            .ok_or(PipelineBuildError::NoVertexShader)?;

        let fragment_shader_data = self
            .fragment_shader_data
            .ok_or(PipelineBuildError::NoFragmentShader)?;

        let vertex_shader_module = Self::create_shader_module(device.clone(), vertex_shader_data)?;
        let vertex_shader_info =
            Self::create_shader_stage_info(vertex_shader_module, ash::vk::ShaderStageFlags::VERTEX);

        let fragment_shader_module =
            Self::create_shader_module(device.clone(), fragment_shader_data)?;
        let fragment_shader_info = Self::create_shader_stage_info(
            fragment_shader_module,
            ash::vk::ShaderStageFlags::FRAGMENT,
        );

        // Don't initialize this - we'll leave it as dynamic state.
        let viewport_info = ash::vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let vertex_input_info = ash::vk::PipelineVertexInputStateCreateInfo::default();

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
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(ash::vk::CompareOp::NEVER)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .front(ash::vk::StencilOpState::default())
            .back(ash::vk::StencilOpState::default())
            .min_depth_bounds(0.0f32)
            .max_depth_bounds(1.0f32);

        let color_format = self.color_format.ok_or(PipelineBuildError::NoColorFormat)?;
        let depth_format = self.depth_format.ok_or(PipelineBuildError::NoDepthFormat)?;

        let color_formats = &[color_format];
        let mut rendering_info = ash::vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(color_formats)
            .depth_attachment_format(depth_format);

        let dynamic_info = ash::vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR,
        ]);

        let layout_info = ash::vk::PipelineLayoutCreateInfo::default();

        let layout = unsafe { device.handle().create_pipeline_layout(&layout_info, None)? };

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

        return Ok(Pipeline { device, handle });
    }
}
