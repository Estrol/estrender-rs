use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

pub struct BufferEntry {
    pub buffer: wgpu::Buffer,

    pub reference: usize,
    pub lifetime: usize,
    pub is_temporary: bool,
}

pub struct BufferManager {
    pub temp_buffers: HashMap<usize, BufferEntry>,
}

#[allow(dead_code)]
impl BufferManager {
    pub fn new() -> Self {
        Self {
            temp_buffers: HashMap::new(),
        }
    }

    pub fn create_buffer(
        &mut self,
        device: &wgpu::Device,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages,
        mapped_at_creation: bool,
    ) -> wgpu::Buffer {
        let buffer = self.internal_make(device, size, usage, mapped_at_creation);

        buffer
    }

    pub fn create_buffer_with<T>(
        &mut self,
        device: &wgpu::Device,
        data: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        let size = (data.len() * std::mem::size_of::<T>()) as wgpu::BufferAddress;
        let buffer = self.internal_make(device, size, usage, true);

        let mut mapped_range = buffer.slice(..)
            .get_mapped_range_mut();

        let dst = &mut mapped_range[..data.len() * std::mem::size_of::<T>()];
        dst.copy_from_slice(bytemuck::cast_slice(data));

        drop(mapped_range);

        buffer.unmap();

        buffer
    }

    pub(crate) fn internal_make(
        &mut self,
        device: &wgpu::Device,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages,
        mapped_at_creation: bool,
    ) -> wgpu::Buffer {
        // This is to honor vulkan's requirement that buffer sizes must be a multiple of COPY_BUFFER_ALIGNMENT.
        let unaligned_size = wgpu::COPY_BUFFER_ALIGNMENT - 1;
        let size = ((size + unaligned_size) & !unaligned_size).max(wgpu::COPY_BUFFER_ALIGNMENT);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(
                format!("Internal Buffer, usage: {}, size: {}", usage.bits(), size).as_str(),
            ),
            size,
            usage,
            mapped_at_creation,
        });

        buffer
    }
}
