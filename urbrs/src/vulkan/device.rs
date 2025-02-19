use std::sync::Arc;

use ash::prelude::VkResult;

use super::instance::Instance;
use super::phys_device::PhysicalDevice;

pub struct DeviceQueue {
    pub idx: u32,
    pub queue: ash::vk::Queue,
}

pub struct Device {
    _instance: Arc<Instance>,

    handle: ash::Device,
    physical_device: PhysicalDevice,

    graphics_queue: DeviceQueue,
    _transfer_queue: DeviceQueue,
    present_queue: DeviceQueue,
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

impl Device {
    pub fn new(instance: Arc<Instance>, physical_device: PhysicalDevice) -> VkResult<Self> {
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
        let present_family = physical_device.present_family();
        queue_infos.push(new_queue_create_info(graphics_family, &priorities));

        if transfer_family != graphics_family {
            queue_infos.push(new_queue_create_info(transfer_family, &priorities));
        }

        if present_family != graphics_family && present_family != transfer_family {
            queue_infos.push(new_queue_create_info(present_family, &priorities));
        }

        let create_info = ash::vk::DeviceCreateInfo::default()
            .enabled_features(&features)
            .enabled_extension_names(required_extensions.as_slice())
            .queue_create_infos(&queue_infos)
            .push_next(&mut dynamic_rendering)
            .push_next(&mut sync_2);

        let device = unsafe {
            instance
                .handle()
                .create_device(physical_device.handle(), &create_info, None)?
        };

        let graphics_queue = get_device_queue(&device, graphics_family);
        let transfer_queue = get_device_queue(&device, transfer_family);
        let present_queue = get_device_queue(&device, present_family);

        Ok(Self {
            _instance: instance,
            handle: device,
            physical_device,
            graphics_queue,
            _transfer_queue: transfer_queue,
            present_queue,
        })
    }

    pub fn handle(&self) -> &ash::Device {
        &self.handle
    }

    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.physical_device
    }

    pub fn graphics_queue(&self) -> &DeviceQueue {
        &self.graphics_queue
    }

    pub fn _transfer_queue(&self) -> &DeviceQueue {
        &self._transfer_queue
    }

    pub fn present_queue(&self) -> &DeviceQueue {
        &self.present_queue
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_device(None) };
    }
}
