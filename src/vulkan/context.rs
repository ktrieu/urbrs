use std::ffi::{c_void, CStr};

use ash::{Entry, Instance};

use super::device::Device;
use super::phys_device::PhysicalDevice;

pub struct Context<'a> {
    instance: Instance,
    device: Device<'a>,
}

unsafe extern "system" fn debug_callback(
    _message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _p_user_data: *mut c_void,
) -> u32 {
    // Safety: we should always get a valid pointer from the debug callback.
    let msg_string = unsafe {
        (*p_callback_data)
            .message_as_c_str()
            .expect("debug message should be valid CStr")
    }
    .to_str()
    .expect("debug message should be valid UTF-8");

    println!("{}", msg_string);

    return ash::vk::FALSE;
}

impl<'a> Context<'a> {
    const REQUIRED_VALIDATION_LAYERS: &'static [&'static CStr; 1] =
        &[c"VK_LAYER_KHRONOS_validation"];

    const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

    pub fn new() -> Result<Self, ash::vk::Result> {
        let entry = Entry::linked();

        let app_info = ash::vk::ApplicationInfo::default()
            .application_version(ash::vk::make_api_version(0, 1, 0, 0))
            .api_version(ash::vk::API_VERSION_1_3)
            .application_name(c"urbrs");
        let mut create_info = ash::vk::InstanceCreateInfo::default().application_info(&app_info);

        let mut enabled_layers: Vec<*const i8> = Vec::new();
        let available_validation_layers = unsafe { entry.enumerate_instance_layer_properties()? };

        let exts = [ash::vk::EXT_DEBUG_UTILS_NAME.as_ptr()];

        if Self::VALIDATION_ENABLED {
            let all_validation_layers_supported =
                Self::REQUIRED_VALIDATION_LAYERS.iter().all(|l| {
                    available_validation_layers
                        .iter()
                        .find(|avail| match avail.layer_name_as_c_str() {
                            Ok(name) => *l == name,
                            Err(_) => false,
                        })
                        .is_some()
                });

            if !all_validation_layers_supported {
                // TODO: use a real error type instead of giving up and dying
                panic!("validation layer not supported!");
            }

            for l in Self::REQUIRED_VALIDATION_LAYERS {
                enabled_layers.push(l.as_ptr());
            }

            create_info = create_info.enabled_layer_names(enabled_layers.as_slice());

            create_info = create_info.enabled_extension_names(&exts);
        }

        // Safety: It's safe to use create_instance any time if it comes from Entry::linked.
        let instance = unsafe { entry.create_instance(&create_info, None)? };

        if Self::VALIDATION_ENABLED {
            let debug_msg_create_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                        | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                )
                .pfn_user_callback(Some(debug_callback));

            let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
            unsafe { debug_utils.create_debug_utils_messenger(&debug_msg_create_info, None)? };
        }

        let phys_device = PhysicalDevice::select_device(&instance)
            .expect("physical device selection should succeed")
            .expect("suitable physical device should exist");

        let phys_device_name = phys_device.name();
        println!("Selected physical device {phys_device_name}");

        let device =
            Device::new(&instance, phys_device).expect("logical device creation should succeed");

        Ok(Self { instance, device })
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }
}
