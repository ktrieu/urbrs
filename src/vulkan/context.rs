use ash::{Entry, Instance};

use super::phys_device::PhysicalDevice;

pub struct Context<'a> {
    instance: Instance,
    phys_device: PhysicalDevice<'a>,
}

impl<'a> Context<'a> {
    pub fn new() -> Result<Self, ash::vk::Result> {
        let entry = Entry::linked();

        let app_info = ash::vk::ApplicationInfo::default()
            .api_version(ash::vk::make_api_version(0, 1, 0, 0))
            .application_name(c"urbrs");
        let create_info = ash::vk::InstanceCreateInfo::default().application_info(&app_info);

        // Safety: It's safe to use create_instance any time if it comes from Entry::linked.
        let instance = unsafe { entry.create_instance(&create_info, None)? };

        let phys_device = PhysicalDevice::select_device(&instance)
            .expect("physical device selection should succeed")
            .expect("suitable physical device should exist");
        let phys_device_name = phys_device.name();

        println!("Selected device {phys_device_name}");

        Ok(Self {
            instance,
            phys_device,
        })
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }
}
