use std::{
    fs,
    io::{self, BufReader, Read, Seek},
    ops::{Div, Rem},
    path::Path,
    sync::Arc,
};

use super::{command::CommandBuffer, device::Device};

pub fn read_spirv(path: &Path) -> anyhow::Result<Vec<u32>> {
    let mut file = fs::File::open(path)?;

    // Get the size of file - need two seek ops for this.
    let size = file.seek(io::SeekFrom::End(0))?;
    file.rewind()?;

    let data_len = size.div(4);

    // We expect a file of 4 byte words for SPIR-V.
    let remainder = size.rem(4);
    if remainder != 0 {
        return Err(anyhow::anyhow!("SPIR-V file size was not a multiple of 4"));
    }

    let mut reader = BufReader::new(file);

    let mut data: Vec<u32> = Vec::with_capacity(data_len as usize);
    let mut bytes: [u8; 4] = [0, 0, 0, 0];

    loop {
        match reader.read_exact(&mut bytes) {
            Ok(_) => {
                data.push(u32::from_le_bytes(bytes));
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                } else {
                    return Err(err.into());
                }
            }
        }
    }

    return Ok(data);
}

pub struct ImageBarrierState {
    layout: ash::vk::ImageLayout,
    stage: ash::vk::PipelineStageFlags2,
    access: ash::vk::AccessFlags2,
}

fn transition_image(
    device: Arc<Device>,
    command_buffer: &CommandBuffer,
    image: ash::vk::Image,
    range: ash::vk::ImageSubresourceRange,
    src: ImageBarrierState,
    dst: ImageBarrierState,
) {
    let barrier = ash::vk::ImageMemoryBarrier2::default()
        .src_stage_mask(src.stage)
        .src_access_mask(src.access)
        .old_layout(src.layout)
        .dst_stage_mask(dst.stage)
        .dst_access_mask(dst.access)
        .new_layout(dst.layout)
        .subresource_range(range)
        .image(image);

    let slice = &[barrier];

    let dep_info = ash::vk::DependencyInfo::default().image_memory_barriers(slice);

    unsafe {
        device
            .handle()
            .cmd_pipeline_barrier2(command_buffer.handle(), &dep_info)
    };
}

fn get_subresource_range(aspect: ash::vk::ImageAspectFlags) -> ash::vk::ImageSubresourceRange {
    ash::vk::ImageSubresourceRange::default()
        .base_array_layer(0)
        .base_mip_level(0)
        .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
        .level_count(ash::vk::REMAINING_MIP_LEVELS)
        .aspect_mask(aspect)
}

pub fn swap_acquire_transition(
    device: Arc<Device>,
    command_buffer: &CommandBuffer,
    image: ash::vk::Image,
) {
    let src_state = ImageBarrierState {
        layout: ash::vk::ImageLayout::UNDEFINED,
        stage: ash::vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: ash::vk::AccessFlags2::empty(),
    };

    let dst_state = ImageBarrierState {
        layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        stage: ash::vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: ash::vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
    };

    let range = get_subresource_range(ash::vk::ImageAspectFlags::COLOR);

    transition_image(device, command_buffer, image, range, src_state, dst_state);
}

pub fn swap_present_transition(
    device: Arc<Device>,
    command_buffer: &CommandBuffer,
    image: ash::vk::Image,
) {
    let src_state = ImageBarrierState {
        layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        stage: ash::vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: ash::vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
    };

    let dst_state = ImageBarrierState {
        layout: ash::vk::ImageLayout::PRESENT_SRC_KHR,
        stage: ash::vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
        access: ash::vk::AccessFlags2::empty(),
    };

    let range = get_subresource_range(ash::vk::ImageAspectFlags::COLOR);

    transition_image(device, command_buffer, image, range, src_state, dst_state);
}
