use std::fmt::Display;
use std::io::{BufReader, Read, Seek};
use std::ops::{Div, Rem};
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::device::Device;
use super::instance::{Instance, InstanceCreateError};
use super::phys_device::PhysicalDevice;
use super::pipeline::PipelineBuilder;
use super::surface::Surface;
use super::swapchain::Swapchain;

pub struct Context {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
}

#[derive(Debug)]
pub enum ContextCreateError {
    InstanceError(InstanceCreateError),
    VkError(ash::vk::Result),
    NoDevice,
}

impl From<InstanceCreateError> for ContextCreateError {
    fn from(value: InstanceCreateError) -> Self {
        ContextCreateError::InstanceError(value)
    }
}

impl From<ash::vk::Result> for ContextCreateError {
    fn from(value: ash::vk::Result) -> Self {
        ContextCreateError::VkError(value)
    }
}

impl Display for ContextCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextCreateError::InstanceError(instance_create_error) => {
                write!(f, "error creating instance: {instance_create_error}")
            }
            ContextCreateError::VkError(vk_error) => {
                write!(f, "vulkan error: {vk_error}")
            }
            ContextCreateError::NoDevice => write!(f, "no suitable device found"),
        }
    }
}

#[derive(Debug)]
enum SpirvReadError {
    IoError(io::Error),
    InvalidSpirvFile,
}

impl From<io::Error> for SpirvReadError {
    fn from(value: io::Error) -> Self {
        SpirvReadError::IoError(value)
    }
}

impl Display for SpirvReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpirvReadError::IoError(error) => write!(f, "io error: {error}"),
            SpirvReadError::InvalidSpirvFile => write!(f, "invalid SPIR-V file"),
        }
    }
}

fn read_spirv(path: &Path) -> Result<Vec<u32>, SpirvReadError> {
    let mut file = fs::File::open(path)?;

    // Get the size of file - need two seek ops for this.
    let size = file.seek(io::SeekFrom::End(0))?;
    file.rewind()?;

    let data_len = size.div(4);

    // We expect a file of 4 byte words for SPIR-V.
    let remainder = size.rem(4);
    if remainder != 0 {
        return Err(SpirvReadError::InvalidSpirvFile);
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

impl Context {
    pub fn new(
        window: &winit::window::Window,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, ContextCreateError> {
        let instance = Arc::new(Instance::new(display_handle)?);

        let surface = Arc::new(Surface::new(
            instance.clone(),
            window_handle,
            display_handle,
        )?);

        let phys_device = PhysicalDevice::select_device(&instance.handle(), &surface)?
            .ok_or(ContextCreateError::NoDevice)?;

        let device = Arc::new(Device::new(instance.clone(), phys_device)?);

        let swapchain = Arc::new(Swapchain::new(
            instance.clone(),
            device.clone(),
            surface.clone(),
            window,
        )?);

        let vertex_shader_data = read_spirv(Path::new("./data/shader/a.spv.vert"))
            .expect("vertex shader loading should succeed");
        let fragment_shader_data = read_spirv(Path::new("./data/shader/a.spv.frag"))
            .expect("vertex shader loading should succeed");

        let pipeline = PipelineBuilder::new()
            .with_color_format(swapchain.surface_color_format())
            .with_depth_format(ash::vk::Format::UNDEFINED)
            .with_vertex_shader_data(&vertex_shader_data)
            .with_fragment_shader_data(&fragment_shader_data)
            .build(device.clone())
            .expect("pipeline creation should succeed");

        Ok(Self {
            instance,
            surface,
            device,
            swapchain,
        })
    }
}
