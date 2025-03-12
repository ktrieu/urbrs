use std::sync::Arc;

use super::{
    device::{Device, DeviceQueue},
    instance::Instance,
    phys_device::PhysicalDevice,
    surface::Surface,
    sync::Semaphore,
};

pub struct SwapchainImage {
    pub image: ash::vk::Image,
    pub view: ash::vk::ImageView,
    pub idx: u32,
}

pub struct Swapchain {
    device: Arc<Device>,
    _surface: Arc<Surface>,
    surface_format: ash::vk::SurfaceFormatKHR,
    handle: ash::vk::SwapchainKHR,
    swapchain_device: ash::khr::swapchain::Device,

    images: Vec<SwapchainImage>,
    swap_area: ash::vk::Rect2D,
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

    pub fn swap_area(&self) -> ash::vk::Rect2D {
        self.swap_area
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

    pub fn surface_color_format(&self) -> ash::vk::Format {
        self.surface_format.format
    }

    pub fn acquire_image(&self, completion: &Semaphore) -> anyhow::Result<&SwapchainImage> {
        let (idx, _) = unsafe {
            self.swapchain_device.acquire_next_image(
                self.handle,
                1_000_000_000,
                completion.handle(),
                ash::vk::Fence::null(),
            )?
        };

        let image = self
            .images
            .get(idx as usize)
            .expect("acquired image idx should be correct");

        Ok(image)
    }

    unsafe fn new_image_view(
        device: &ash::Device,
        image: ash::vk::Image,
        format: ash::vk::Format,
    ) -> anyhow::Result<ash::vk::ImageView> {
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

        Ok(device.create_image_view(&info, None)?)
    }

    pub fn present(
        &self,
        idx: u32,
        queue: &DeviceQueue,
        completion: &Semaphore,
    ) -> anyhow::Result<()> {
        let swapchains = &[self.handle];
        let semaphores = &[completion.handle()];
        let indices = &[idx];

        let present_info = ash::vk::PresentInfoKHR::default()
            .swapchains(swapchains)
            .wait_semaphores(semaphores)
            .image_indices(indices);

        unsafe {
            self.swapchain_device
                .queue_present(queue.queue, &present_info)
        }?;

        Ok(())
    }

    pub fn new(
        instance: Arc<Instance>,
        device: Arc<Device>,
        surface: Arc<Surface>,
        window: &winit::window::Window,
    ) -> anyhow::Result<Self> {
        let swapchain_device = ash::khr::swapchain::Device::new(instance.handle(), device.handle());

        let surface_format = device.physical_device().surface_format();
        let image_count = Self::select_image_count(device.physical_device());

        let extent = Self::select_swap_extent(device.physical_device(), window);

        let mut info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(*surface.handle())
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .present_mode(device.physical_device().present_mode())
            .image_extent(extent)
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

        for (idx, img) in vk_images.iter().enumerate() {
            images.push(SwapchainImage {
                image: *img,
                // Safety: image is a valid image since it came from get_swapchain_images
                view: unsafe {
                    Self::new_image_view(device.handle(), *img, surface_format.format)?
                },
                idx: idx as u32,
            });
        }

        Ok(Self {
            device,
            _surface: surface,
            surface_format,
            handle,
            swapchain_device,
            images,
            swap_area: extent.into(),
        })
    }

    pub fn extent(&self) -> ash::vk::Extent2D {
        self.swap_area.extent
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
