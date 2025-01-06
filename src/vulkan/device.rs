use ash::prelude::VkResult;

use super::phys_device::PhysicalDevice;

struct DeviceQueue {
    idx: u32,
    queue: ash::vk::Queue,
}

pub struct Device<'a> {
    handle: ash::Device,
    physical_device: PhysicalDevice<'a>,

    graphics_queue: DeviceQueue,
    transfer_queue: DeviceQueue,
}

fn new_queue_create_info<'a>(
    idx: u32,
    priorities: &'a [f32],
) -> ash::vk::DeviceQueueCreateInfo<'a> {
    let mut info = ash::vk::DeviceQueueCreateInfo::default()
        .queue_family_index(idx)
        .queue_priorities(&priorities);
    info.queue_count = 1;

    info
}

fn get_device_queue(device: &ash::Device, idx: u32) -> DeviceQueue {
    let info = ash::vk::DeviceQueueInfo2::default()
        .queue_family_index(idx)
        .queue_index(0);
    let queue = unsafe { device.get_device_queue2(&info) };

    DeviceQueue { idx, queue }
}

impl<'a> Device<'a> {
    pub fn new(instance: &ash::Instance, physical_device: PhysicalDevice<'a>) -> VkResult<Self> {
        let features = ash::vk::PhysicalDeviceFeatures::default();

        let mut dynamic_rendering =
            ash::vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

        let mut sync_2 =
            ash::vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);

        let required_extensions: Vec<*const i8> = PhysicalDevice::REQUIRED_EXTENSIONS
            .iter()
            .map(|s| s.as_ptr())
            .collect();

        let mut queue_infos: Vec<ash::vk::DeviceQueueCreateInfo> = Vec::new();
        let priorities = [1.0];

        let graphics_family = physical_device.graphics_family();
        let transfer_family = physical_device.transfer_family();
        queue_infos.push(new_queue_create_info(graphics_family, &priorities));

        if physical_device.graphics_family() != physical_device.transfer_family() {
            queue_infos.push(new_queue_create_info(transfer_family, &priorities));
        }

        let create_info = ash::vk::DeviceCreateInfo::default()
            .enabled_features(&features)
            .enabled_extension_names(required_extensions.as_slice())
            .queue_create_infos(&queue_infos)
            .push_next(&mut dynamic_rendering)
            .push_next(&mut sync_2);

        let device =
            unsafe { instance.create_device(physical_device.handle(), &create_info, None)? };

        println!("getting graphics queue");
        let graphics_queue = get_device_queue(&device, graphics_family);
        println!("getting transfer queue");
        let transfer_queue = get_device_queue(&device, transfer_family);

        Ok(Self {
            handle: device,
            physical_device,
            graphics_queue,
            transfer_queue,
        })
    }
}
