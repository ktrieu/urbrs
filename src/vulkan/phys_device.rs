use std::ffi::CStr;

use ash::prelude::VkResult;

pub struct PhysicalDevice<'a> {
    handle: ash::vk::PhysicalDevice,
    properties: ash::vk::PhysicalDeviceProperties2<'a>,
    features: ash::vk::PhysicalDeviceFeatures,
    extensions: Vec<ash::vk::ExtensionProperties>,
    queue_families: Vec<ash::vk::QueueFamilyProperties2<'a>>,

    graphics_families: Vec<usize>,
    transfer_families: Vec<usize>,
}

fn get_queue_families_with_flag(
    families: &Vec<ash::vk::QueueFamilyProperties2>,
    flag: ash::vk::QueueFlags,
) -> Vec<usize> {
    families
        .iter()
        .enumerate()
        .filter(|(_, f)| f.queue_family_properties.queue_flags.contains(flag))
        .map(|(i, _)| i)
        .collect()
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

impl<'a> PhysicalDevice<'a> {
    pub fn select_device(instance: &ash::Instance) -> VkResult<Option<Self>> {
        let device_handles = unsafe { instance.enumerate_physical_devices() }?;

        let mut devices = device_handles.iter().filter_map(|handle| unsafe {
            match Self::new(instance, *handle) {
                Ok(device) => device,
                Err(err) => {
                    println!("Error creating device {err:?}");
                    None
                }
            }
        });

        return Ok(devices.next());
    }

    const REQUIRED_EXTENSIONS: &'static [&'static CStr; 3] = &[
        ash::vk::KHR_SWAPCHAIN_NAME,
        ash::vk::KHR_DYNAMIC_RENDERING_NAME,
        ash::vk::KHR_SYNCHRONIZATION2_NAME,
    ];

    unsafe fn new(
        instance: &ash::Instance,
        handle: ash::vk::PhysicalDevice,
    ) -> VkResult<Option<Self>> {
        let mut properties = ash::vk::PhysicalDeviceProperties2::default();
        instance.get_physical_device_properties2(handle, &mut properties);

        let features = instance.get_physical_device_features(handle);

        let extensions = instance.enumerate_device_extension_properties(handle)?;

        let all_required_exts_supported = Self::REQUIRED_EXTENSIONS
            .iter()
            .all(|required| is_extension_supported(&extensions, &required));

        if !all_required_exts_supported {
            return Ok(None);
        }

        let queue_families_len = instance.get_physical_device_queue_family_properties2_len(handle);

        let mut queue_families =
            vec![ash::vk::QueueFamilyProperties2::default(); queue_families_len];

        instance
            .get_physical_device_queue_family_properties2(handle, queue_families.as_mut_slice());

        let graphics_families =
            get_queue_families_with_flag(&queue_families, ash::vk::QueueFlags::GRAPHICS);
        let transfer_families =
            get_queue_families_with_flag(&queue_families, ash::vk::QueueFlags::TRANSFER);

        // TODO: surface properties...

        return Ok(Some(Self {
            handle,
            properties,
            features,
            extensions,
            queue_families,
            graphics_families,
            transfer_families,
        }));
    }

    pub fn name(&self) -> &str {
        self.properties
            .properties
            .device_name_as_c_str()
            .expect("device name should be valid CStr")
            .to_str()
            .expect("device name should be valid UTF-8")
    }
}
