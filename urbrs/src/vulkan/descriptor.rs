use std::sync::Arc;

use crate::vulkan::device::Device;

pub struct DescriptorPool {
    device: Arc<Device>,
    handle: ash::vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(
        device: Arc<Device>,
        ty: ash::vk::DescriptorType,
        size: u32,
    ) -> anyhow::Result<Self> {
        // For now, we only support pools devoted to one type of descriptor.
        let pool_sizes: [ash::vk::DescriptorPoolSize; 1] = [ash::vk::DescriptorPoolSize::default()
            .ty(ty)
            .descriptor_count(size)];

        let info = ash::vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(size);

        let pool = unsafe { device.handle().create_descriptor_pool(&info, None) }?;

        Ok(Self {
            device,
            handle: pool,
        })
    }
}

pub struct DescriptorSet {
    handle: ash::vk::DescriptorSet,
    _pool: Arc<DescriptorPool>,
}

impl DescriptorSet {
    pub fn alloc_from_pool(
        pool: Arc<DescriptorPool>,
        layout: ash::vk::DescriptorSetLayout,
    ) -> anyhow::Result<DescriptorSet> {
        let layouts = [layout];

        let mut info = ash::vk::DescriptorSetAllocateInfo::default()
            .set_layouts(&layouts)
            .descriptor_pool(pool.handle);

        info.descriptor_set_count = 1;

        let sets = unsafe { pool.device.handle().allocate_descriptor_sets(&info) }?;

        let set = sets
            .get(0)
            .expect("allocate_descriptor_sets should return one item");

        Ok(Self {
            handle: *set,
            _pool: pool,
        })
    }
}
