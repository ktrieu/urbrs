use ash::{
    vk::{make_api_version, ApplicationInfo, InstanceCreateInfo},
    Entry, Instance,
};

pub struct Context {
    instance: Instance,
}

impl Context {
    pub fn new() -> Result<Self, ash::vk::Result> {
        let entry = Entry::linked();

        let app_info = ApplicationInfo::default()
            .api_version(make_api_version(0, 1, 0, 0))
            .application_name(c"urbrs");
        let create_info = InstanceCreateInfo::default().application_info(&app_info);

        // Safety: It's safe to use create_instance any time if it comes from Entry::linked.
        unsafe {
            let instance = entry.create_instance(&create_info, None)?;
            Ok(Self { instance })
        }
    }
}
