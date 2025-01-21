use std::sync::Arc;

use ash::prelude::VkResult;

use super::{device::Device, instance::Instance, phys_device::PhysicalDevice, surface::Surface};

struct SwapchainImage {
    image: ash::vk::Image,
    view: ash::vk::ImageView,
}

pub struct Swapchain {
    device: Arc<Device>,
    surface: Arc<Surface>,
    handle: ash::vk::SwapchainKHR,
    swapchain_device: ash::khr::swapchain::Device,

    images: Vec<SwapchainImage>,
}

impl Swapchain {
    fn select_swap_extent(
        physical_device: &PhysicalDevice,
        window: &winit::window::Window,
    ) -> ash::vk::Extent2D {
        let current_extent = physical_device.surface_caps().current_extent;
        let min_extent = physical_device.surface_caps().min_image_extent;
        let max_extent = physical_device.surface_caps().max_image_extent;

        if current_extent.height == u32::MAX {
            let win_size = window.inner_size();
            ash::vk::Extent2D::default()
                .width(win_size.width.clamp(min_extent.width, max_extent.width))
                .height(win_size.height.clamp(min_extent.height, max_extent.height))
        } else {
            current_extent
        }
    }

    fn select_image_count(physical_device: &PhysicalDevice) -> u32 {
        // Add 1 so we don't wait on the driver.
        let desired = physical_device.surface_caps().min_image_count + 1;

        let max_count = physical_device.surface_caps().max_image_count;

        // A max of zero indicates no limit.
        if max_count == 0 {
            desired
        } else {
            desired.min(max_count)
        }
    }

    unsafe fn new_image_view(
        device: &ash::Device,
        image: ash::vk::Image,
        format: ash::vk::Format,
    ) -> VkResult<ash::vk::ImageView> {
        let mapping = ash::vk::ComponentMapping::default()
            .r(ash::vk::ComponentSwizzle::IDENTITY)
            .g(ash::vk::ComponentSwizzle::IDENTITY)
            .b(ash::vk::ComponentSwizzle::IDENTITY)
            .a(ash::vk::ComponentSwizzle::IDENTITY);

        let range = ash::vk::ImageSubresourceRange::default()
            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let info = ash::vk::ImageViewCreateInfo::default()
            .image(image)
            .format(format)
            .view_type(ash::vk::ImageViewType::TYPE_2D)
            .components(mapping)
            .subresource_range(range);

        device.create_image_view(&info, None)
    }

    pub fn new(
        instance: Arc<Instance>,
        device: Arc<Device>,
        surface: Arc<Surface>,
        window: &winit::window::Window,
    ) -> VkResult<Self> {
        let swapchain_device = ash::khr::swapchain::Device::new(instance.handle(), device.handle());

        let surface_format = device.physical_device().surface_format();
        let image_count = Self::select_image_count(device.physical_device());

        let mut info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(*surface.handle())
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .present_mode(device.physical_device().present_mode())
            .image_extent(Self::select_swap_extent(device.physical_device(), window))
            .min_image_count(image_count)
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(device.physical_device().surface_caps().current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE);

        let image_sharing_required = device.graphics_queue().idx != device.present_queue().idx;
        let indices = [device.graphics_queue().idx, device.present_queue().idx];

        if image_sharing_required {
            info = info
                .image_sharing_mode(ash::vk::SharingMode::CONCURRENT)
                .queue_family_indices(&indices);

            info.queue_family_index_count = 2;
        } else {
            info = info.image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE);
        }

        let handle = unsafe { swapchain_device.create_swapchain(&info, None)? };

        let vk_images = unsafe { swapchain_device.get_swapchain_images(handle)? };
        let mut images: Vec<SwapchainImage> = Vec::new();

        for img in vk_images {
            images.push(SwapchainImage {
                image: img,
                // Safety: image is a valid image since it came from get_swapchain_images
                view: unsafe { Self::new_image_view(device.handle(), img, surface_format.format)? },
            });
        }

        Ok(Self {
            device,
            surface,
            handle,
            swapchain_device,
            images,
        })
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        for img in &self.images {
            unsafe {
                self.device.handle().destroy_image_view(img.view, None);
            }
        }

        unsafe { self.swapchain_device.destroy_swapchain(self.handle, None) };
    }
}
