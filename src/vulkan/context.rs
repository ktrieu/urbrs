use std::ffi::{c_void, CStr};
use std::fmt::Display;

use ash::prelude::VkResult;
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

#[derive(Debug)]
pub enum ContextCreateError {
    VkError(ash::vk::Result),
    UnsupportedValidationLayers(Vec<String>),
    UnsupportedExtensions(Vec<String>),
}

impl From<ash::vk::Result> for ContextCreateError {
    fn from(value: ash::vk::Result) -> Self {
        Self::VkError(value)
    }
}

impl Display for ContextCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextCreateError::VkError(vk_err) => writeln!(f, "vulkan error: {vk_err}"),
            ContextCreateError::UnsupportedValidationLayers(layers) => {
                let layers = layers.join(", ");
                writeln!(f, "unsupported validation layers: {layers}")
            }
            ContextCreateError::UnsupportedExtensions(exts) => {
                let exts = exts.join(", ");
                writeln!(f, "unsupported instance extensions: {exts}")
            }
        }
    }
}

impl<'a> Context<'a> {
    const REQUIRED_VALIDATION_LAYERS: &'static [&'static CStr; 1] =
        &[c"VK_LAYER_KHRONOS_validation"];

    const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

    const REQUIRED_INSTANCE_EXTENSIONS_BASE: &'static [&'static CStr; 1] =
        &[ash::vk::EXT_DEBUG_UTILS_NAME];

    fn get_required_instance_extensions() -> Vec<&'static CStr> {
        Self::REQUIRED_INSTANCE_EXTENSIONS_BASE
            .iter()
            .copied()
            .collect()
    }

    fn get_unsupported_instance_extensions<'b>(
        required_extensions: &'b Vec<&CStr>,
        extension_props: &Vec<ash::vk::ExtensionProperties>,
    ) -> Vec<&'b CStr> {
        required_extensions
            .iter()
            .copied()
            .filter(|required| {
                extension_props
                    .iter()
                    .find(|extension| {
                        let extension_name = extension
                            .extension_name_as_c_str()
                            .expect("extension name should be valid CStr");
                        *required == extension_name
                    })
                    .is_none()
            })
            .collect()
    }

    fn get_unsupported_validation_layers<'b>(
        required_layers: &'b [&CStr],
        layer_props: &Vec<ash::vk::LayerProperties>,
    ) -> Vec<&'b CStr> {
        required_layers
            .iter()
            .copied()
            .filter(|required| {
                layer_props
                    .iter()
                    .find(|layer| {
                        let layer_name = layer
                            .layer_name_as_c_str()
                            .expect("layer name should be valid CStr");
                        *required == layer_name
                    })
                    .is_none()
            })
            .collect()
    }

    pub fn new() -> Result<Self, ContextCreateError> {
        let entry = Entry::linked();

        let app_info = ash::vk::ApplicationInfo::default()
            .application_version(ash::vk::make_api_version(0, 1, 0, 0))
            .api_version(ash::vk::API_VERSION_1_3)
            .application_name(c"urbrs");
        let mut create_info = ash::vk::InstanceCreateInfo::default().application_info(&app_info);

        let required_extensions = Self::get_required_instance_extensions();
        let supported_extensions = unsafe { entry.enumerate_instance_extension_properties(None)? };

        let unsupported_extensions =
            Self::get_unsupported_instance_extensions(&required_extensions, &supported_extensions);

        if unsupported_extensions.len() > 0 {
            let unsupported_extensions = unsupported_extensions
                .iter()
                .map(|e| {
                    e.to_str()
                        .expect("extension name should be valid str")
                        .to_string()
                })
                .collect();
            return Err(ContextCreateError::UnsupportedExtensions(
                unsupported_extensions,
            ));
        }

        let enabled_extension_names: Vec<*const i8> =
            required_extensions.iter().map(|e| e.as_ptr()).collect();
        create_info = create_info.enabled_extension_names(&enabled_extension_names);

        let available_validation_layers = unsafe { entry.enumerate_instance_layer_properties()? };

        let mut enabled_layers: Vec<*const i8> = Vec::new();

        if Self::VALIDATION_ENABLED {
            let unsupported_layers = Self::get_unsupported_validation_layers(
                Self::REQUIRED_VALIDATION_LAYERS,
                &available_validation_layers,
            );

            if unsupported_layers.len() > 0 {
                let unsupported_layers = unsupported_layers
                    .iter()
                    .map(|e| {
                        e.to_str()
                            .expect("layer name should be valid str")
                            .to_string()
                    })
                    .collect();

                return Err(ContextCreateError::UnsupportedValidationLayers(
                    unsupported_layers,
                ));
            }

            enabled_layers = Self::REQUIRED_VALIDATION_LAYERS
                .iter()
                .map(|l| l.as_ptr())
                .collect();

            create_info = create_info.enabled_layer_names(enabled_layers.as_slice());
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
