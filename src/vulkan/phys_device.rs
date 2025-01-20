use std::{ffi::CStr, fmt::Display};

use ash::prelude::VkResult;

use super::surface::Surface;

pub struct PhysicalDevice {
    handle: ash::vk::PhysicalDevice,
    properties: ash::vk::PhysicalDeviceProperties,
    features: ash::vk::PhysicalDeviceFeatures,
    extensions: Vec<ash::vk::ExtensionProperties>,
    queue_families: Vec<ash::vk::QueueFamilyProperties>,

    surface_caps: ash::vk::SurfaceCapabilitiesKHR,
    surface_formats: Vec<ash::vk::SurfaceFormatKHR>,
    present_modes: Vec<ash::vk::PresentModeKHR>,

    graphics_family: u32,
    transfer_family: u32,
}

pub enum PhysicalDeviceCreateError {
    VkError(ash::vk::Result),
    UnsuitableDevice(String),
}

impl From<ash::vk::Result> for PhysicalDeviceCreateError {
    fn from(value: ash::vk::Result) -> Self {
        PhysicalDeviceCreateError::VkError(value)
    }
}

impl Display for PhysicalDeviceCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicalDeviceCreateError::VkError(vk_result) => {
                writeln!(f, "vulkan error: {vk_result}")
            }
            PhysicalDeviceCreateError::UnsuitableDevice(reason) => {
                writeln!(f, "device not suitable: {reason}")
            }
        }
    }
}

impl PhysicalDevice {
    pub fn select_device(instance: &ash::Instance, surface: &Surface) -> VkResult<Option<Self>> {
        let device_handles = unsafe { instance.enumerate_physical_devices() }?;

        let mut devices = device_handles.iter().filter_map(|handle| unsafe {
            Self::new(instance, surface, *handle)
                .inspect_err(|err| {
                    println!("error creating physical device: {err}. skipping device.")
                })
                .ok()
        });

        return Ok(devices.next());
    }

    fn get_queue_families_with_flag<'props>(
        families: &'props Vec<ash::vk::QueueFamilyProperties>,
        flag: ash::vk::QueueFlags,
    ) -> impl Iterator<Item = u32> + 'props {
        families
            .iter()
            .enumerate()
            .filter(move |(_, f)| f.queue_flags.contains(flag))
            .map(|(i, _)| {
                i.try_into()
                    .expect("queue family index should fit into an u32")
            })
    }

    fn is_extension_supported(extensions: &Vec<ash::vk::ExtensionProperties>, name: &CStr) -> bool {
        extensions
            .iter()
            .find(|e| {
                match e.extension_name_as_c_str() {
                    Ok(ext_name) => ext_name == name,
                    // Just ignore invalid extension names. Maybe log an error or something someday.
                    Err(_) => false,
                }
            })
            .is_some()
    }

    fn select_graphics_family(queue_families: &Vec<ash::vk::QueueFamilyProperties>) -> Option<u32> {
        // Just return any family with graphics support.
        Self::get_queue_families_with_flag(queue_families, ash::vk::QueueFlags::GRAPHICS).next()
    }

    fn select_transfer_family(
        queue_families: &Vec<ash::vk::QueueFamilyProperties>,
        graphics_family: u32,
    ) -> u32 {
        // Try and find a family that isn't our graphics family,
        // But fall back to the graphics family.
        Self::get_queue_families_with_flag(queue_families, ash::vk::QueueFlags::TRANSFER)
            .filter(|f| *f != graphics_family)
            .next()
            .unwrap_or(graphics_family)
    }

    pub const REQUIRED_EXTENSIONS: &'static [&'static CStr; 3] = &[
        ash::vk::KHR_SWAPCHAIN_NAME,
        ash::vk::KHR_DYNAMIC_RENDERING_NAME,
        ash::vk::KHR_SYNCHRONIZATION2_NAME,
    ];

    unsafe fn new(
        instance: &ash::Instance,
        surface: &Surface,
        handle: ash::vk::PhysicalDevice,
    ) -> Result<Self, PhysicalDeviceCreateError> {
        let mut properties = ash::vk::PhysicalDeviceProperties2::default();
        instance.get_physical_device_properties2(handle, &mut properties);

        let properties = properties.properties;

        let features = instance.get_physical_device_features(handle);

        let extensions = instance.enumerate_device_extension_properties(handle)?;

        let unsupported_extensions: Vec<&CStr> = Self::REQUIRED_EXTENSIONS
            .iter()
            .copied()
            .filter(|required| !Self::is_extension_supported(&extensions, &required))
            .collect();

        if unsupported_extensions.len() > 0 {
            let unsupported_extension_str = unsupported_extensions
                .iter()
                .map(|unsupported| {
                    unsupported
                        .to_str()
                        .expect("extension names should be valid str")
                })
                .collect::<Vec<&str>>()
                .join(", ");
            return Err(PhysicalDeviceCreateError::UnsuitableDevice(format!(
                "physical device extensions not supported: {unsupported_extension_str}"
            )));
        }

        let queue_families_len = instance.get_physical_device_queue_family_properties2_len(handle);

        let mut queue_families =
            vec![ash::vk::QueueFamilyProperties2::default(); queue_families_len];

        instance
            .get_physical_device_queue_family_properties2(handle, queue_families.as_mut_slice());

        let queue_families: Vec<ash::vk::QueueFamilyProperties> = queue_families
            .iter()
            .map(|qf| qf.queue_family_properties)
            .collect();

        let graphics_family = Self::select_graphics_family(&queue_families).ok_or(
            PhysicalDeviceCreateError::UnsuitableDevice("no graphics family available".to_string()),
        )?;

        let transfer_family = Self::select_transfer_family(&queue_families, graphics_family);

        let surface_caps = surface
            .surface_instance()
            .get_physical_device_surface_capabilities(handle, *surface.handle())?;

        let surface_formats = surface
            .surface_instance()
            .get_physical_device_surface_formats(handle, *surface.handle())?;

        let present_modes = surface
            .surface_instance()
            .get_physical_device_surface_present_modes(handle, *surface.handle())?;

        let phys_device = Self {
            handle,
            properties,
            features,
            extensions,
            queue_families,
            surface_caps,
            surface_formats,
            present_modes,
            graphics_family,
            transfer_family,
        };

        Ok(phys_device)
    }

    pub fn name(&self) -> &str {
        self.properties
            .device_name_as_c_str()
            .expect("device name should be valid CStr")
            .to_str()
            .expect("device name should be valid UTF-8")
    }

    pub fn handle(&self) -> ash::vk::PhysicalDevice {
        self.handle
    }

    pub fn graphics_family(&self) -> u32 {
        self.graphics_family
    }

    pub fn transfer_family(&self) -> u32 {
        self.transfer_family
    }
}
