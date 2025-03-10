use std::ffi::{c_void, CStr};

use winit::raw_window_handle::RawDisplayHandle;

struct DebugObjs {
    utils: ash::ext::debug_utils::Instance,
    messenger: ash::vk::DebugUtilsMessengerEXT,
}

pub struct Instance {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_objs: Option<DebugObjs>,
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

impl Instance {
    const REQUIRED_VALIDATION_LAYERS: &'static [&'static CStr; 1] =
        &[c"VK_LAYER_KHRONOS_validation"];

    const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

    const REQUIRED_INSTANCE_EXTENSIONS_BASE: &'static [&'static CStr; 1] =
        &[ash::vk::EXT_DEBUG_UTILS_NAME];

    fn get_required_instance_extensions(
        display_handle: RawDisplayHandle,
    ) -> anyhow::Result<Vec<&'static CStr>> {
        let mut base: Vec<&'static CStr> = Self::REQUIRED_INSTANCE_EXTENSIONS_BASE
            .iter()
            .copied()
            .collect();

        let mut surface_exts: Vec<&'static CStr> =
            ash_window::enumerate_required_extensions(display_handle)?
                .iter()
                // Safety: ash_window should always give us a pointer that's safe to use here.
                .map(|ext| unsafe { CStr::from_ptr(*ext) })
                .collect();

        base.append(&mut surface_exts);

        Ok(base)
    }

    fn get_unsupported_instance_extensions<'a>(
        required_extensions: &'a Vec<&CStr>,
        extension_props: &Vec<ash::vk::ExtensionProperties>,
    ) -> Vec<&'a CStr> {
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

    fn get_unsupported_validation_layers<'a>(
        required_layers: &'a [&CStr],
        layer_props: &Vec<ash::vk::LayerProperties>,
    ) -> Vec<&'a CStr> {
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

    pub fn new(display_handle: RawDisplayHandle) -> anyhow::Result<Self> {
        let entry = ash::Entry::linked();

        let app_info = ash::vk::ApplicationInfo::default()
            .application_version(ash::vk::make_api_version(0, 1, 0, 0))
            .api_version(ash::vk::API_VERSION_1_3)
            .application_name(c"urbrs");
        let mut create_info = ash::vk::InstanceCreateInfo::default().application_info(&app_info);

        let required_extensions = Self::get_required_instance_extensions(display_handle)?;
        let supported_extensions = unsafe { entry.enumerate_instance_extension_properties(None)? };

        let unsupported_extensions =
            Self::get_unsupported_instance_extensions(&required_extensions, &supported_extensions);

        if unsupported_extensions.len() > 0 {
            let unsupported_extensions: Vec<String> = unsupported_extensions
                .iter()
                .map(|e| {
                    e.to_str()
                        .expect("extension name should be valid str")
                        .to_string()
                })
                .collect();
            return Err(anyhow::anyhow!(
                "the following required extensions were unsupported: {:?}",
                unsupported_extensions
            ));
        }

        let enabled_extension_names: Vec<*const i8> =
            required_extensions.iter().map(|e| e.as_ptr()).collect();
        create_info = create_info.enabled_extension_names(&enabled_extension_names);

        let available_validation_layers = unsafe { entry.enumerate_instance_layer_properties()? };

        let enabled_layers: Vec<*const i8>;

        if Self::VALIDATION_ENABLED {
            let unsupported_layers = Self::get_unsupported_validation_layers(
                Self::REQUIRED_VALIDATION_LAYERS,
                &available_validation_layers,
            );

            if unsupported_layers.len() > 0 {
                let unsupported_layers: Vec<String> = unsupported_layers
                    .iter()
                    .map(|e| {
                        e.to_str()
                            .expect("layer name should be valid str")
                            .to_string()
                    })
                    .collect();

                return Err(anyhow::anyhow!(
                    "the following validation layers were unsupported: {:?}",
                    unsupported_layers
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

        let debug_objs = if Self::VALIDATION_ENABLED {
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

            let utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let messenger =
                unsafe { utils.create_debug_utils_messenger(&debug_msg_create_info, None)? };

            Some(DebugObjs { utils, messenger })
        } else {
            None
        };

        Ok(Self {
            entry,
            instance,
            debug_objs,
        })
    }

    pub fn handle(&self) -> &ash::Instance {
        &self.instance
    }

    pub fn entry(&self) -> &ash::Entry {
        &self.entry
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            if let Some(debug_objs) = &self.debug_objs {
                debug_objs
                    .utils
                    .destroy_debug_utils_messenger(debug_objs.messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}
