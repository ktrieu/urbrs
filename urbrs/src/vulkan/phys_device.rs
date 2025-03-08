use std::ffi::CStr;

use super::surface::Surface;

pub struct PhysicalDevice {
    handle: ash::vk::PhysicalDevice,
    _properties: ash::vk::PhysicalDeviceProperties,
    _features: ash::vk::PhysicalDeviceFeatures,
    _extensions: Vec<ash::vk::ExtensionProperties>,
    _queue_families: Vec<ash::vk::QueueFamilyProperties>,

    surface_caps: ash::vk::SurfaceCapabilitiesKHR,
    surface_format: ash::vk::SurfaceFormatKHR,
    present_mode: ash::vk::PresentModeKHR,

    graphics_family: u32,
    transfer_family: u32,
    present_family: u32,
}

impl PhysicalDevice {
    pub fn select_device(
        instance: &ash::Instance,
        surface: &Surface,
    ) -> anyhow::Result<Option<Self>> {
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

    fn select_present_family(
        physical_device: ash::vk::PhysicalDevice,
        surface: ash::vk::SurfaceKHR,
        surface_instance: &ash::khr::surface::Instance,
        queue_families: &Vec<ash::vk::QueueFamilyProperties>,
        graphics_family: u32,
    ) -> anyhow::Result<Option<u32>> {
        let mut present_family: Option<u32> = None;

        for i in 0..queue_families.len() {
            let i = i as u32;
            let supported = unsafe {
                surface_instance.get_physical_device_surface_support(physical_device, i, surface)?
            };

            if supported {
                // We want a present queue that's the same as the graphics family.
                if graphics_family == i {
                    return Ok(Some(graphics_family));
                }

                // But we'll settle for any one that works.
                present_family = Some(i);
            }
        }

        Ok(present_family)
    }

    fn select_surface_format(
        formats: &Vec<ash::vk::SurfaceFormatKHR>,
    ) -> Option<ash::vk::SurfaceFormatKHR> {
        let desired = formats.iter().find(|sf| {
            sf.format == ash::vk::Format::B8G8R8A8_SRGB
                && sf.color_space == ash::vk::ColorSpaceKHR::SRGB_NONLINEAR
        });

        let first = formats.get(0);

        desired.or(first).copied()
    }

    fn select_present_mode(
        modes: &Vec<ash::vk::PresentModeKHR>,
    ) -> Option<ash::vk::PresentModeKHR> {
        let desired = modes
            .iter()
            .find(|pm| **pm == ash::vk::PresentModeKHR::MAILBOX);

        let first = modes.get(0);

        desired.or(first).copied()
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
    ) -> anyhow::Result<Self> {
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
            return Err(anyhow::anyhow!(
                "physical device extensions not supported: {:?}",
                unsupported_extensions
            ));
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

        let graphics_family = Self::select_graphics_family(&queue_families)
            .ok_or(anyhow::anyhow!("no graphics family available"))?;

        let transfer_family = Self::select_transfer_family(&queue_families, graphics_family);

        let present_family = Self::select_present_family(
            handle,
            *surface.handle(),
            surface.surface_instance(),
            &queue_families,
            graphics_family,
        )?
        .ok_or(anyhow::anyhow!("no present family found"))?;

        let surface_caps = surface
            .surface_instance()
            .get_physical_device_surface_capabilities(handle, *surface.handle())?;

        let surface_formats = surface
            .surface_instance()
            .get_physical_device_surface_formats(handle, *surface.handle())?;

        let surface_format = Self::select_surface_format(&surface_formats)
            .ok_or(anyhow::anyhow!("no surface format available"))?;

        let present_modes = surface
            .surface_instance()
            .get_physical_device_surface_present_modes(handle, *surface.handle())?;

        let present_mode = Self::select_present_mode(&present_modes)
            .ok_or(anyhow::anyhow!("no valid present mode available"))?;

        let phys_device = Self {
            handle,
            _properties: properties,
            _features: features,
            _extensions: extensions,
            _queue_families: queue_families,
            surface_caps,
            graphics_family,
            transfer_family,
            present_family,
            surface_format,
            present_mode,
        };

        Ok(phys_device)
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

    pub fn present_family(&self) -> u32 {
        self.present_family
    }

    pub fn surface_caps(&self) -> &ash::vk::SurfaceCapabilitiesKHR {
        &self.surface_caps
    }

    pub fn surface_format(&self) -> ash::vk::SurfaceFormatKHR {
        self.surface_format
    }

    pub fn present_mode(&self) -> ash::vk::PresentModeKHR {
        self.present_mode
    }
}
