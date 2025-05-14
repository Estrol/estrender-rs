use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use wgpu::util::DeviceExt;

pub struct BufferEntry {
    pub buffer: wgpu::Buffer,

    pub reference: usize,
    pub lifetime: usize,
    pub is_temporary: bool,
}

pub struct BufferManager {
    pub buffers: HashMap<usize, BufferEntry>,
}

const TEMPORARY_BUFFER_LIFETIME: usize = 60;

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        device: &wgpu::Device,
        data: &[u8],
        usage: wgpu::BufferUsages,
        temporary: bool,
    ) -> wgpu::Buffer {
        let key = {
            let mut hasher = DefaultHasher::new();
            let mut fxhasher = fxhash::FxHasher::default();

            let mut slices = data.chunks_exact(8);
            for slice in &mut slices {
                let u64 = u64::from_ne_bytes(slice.try_into().unwrap());
                fxhasher.write_u64(u64);
            }

            for slice in slices.remainder() {
                fxhasher.write_u8(*slice);
            }

            hasher.write_u64(fxhasher.finish());
            usage.hash(&mut hasher);

            hasher.finish() as usize
        };

        if self.buffers.get(&key).is_none() {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(
                    format!(
                        "Buffer {}, usage: {}, size: {}",
                        key,
                        usage.bits(),
                        data.len()
                    )
                    .as_str(),
                ),
                contents: data,
                usage,
            });

            self.buffers.insert(
                key,
                BufferEntry {
                    buffer,
                    reference: 0,
                    lifetime: 0,
                    is_temporary: temporary,
                },
            );
        }

        let entry = self.buffers.get_mut(&key).unwrap();
        entry.reference += 1;
        entry.buffer.clone()
    }

    pub fn make(
        &mut self,
        device: &wgpu::Device,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages,
        temporary: bool,
    ) -> wgpu::Buffer {
        let key = {
            let mut hasher = DefaultHasher::new();
            size.hash(&mut hasher);
            usage.hash(&mut hasher);

            hasher.finish() as usize
        };

        if self.buffers.get(&key).is_none() {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(
                    format!("Buffer {}, usage: {}, size: {}", key, usage.bits(), size).as_str(),
                ),
                size,
                usage,
                mapped_at_creation: false,
            });

            self.buffers.insert(
                key,
                BufferEntry {
                    buffer,
                    reference: 0,
                    lifetime: 0,
                    is_temporary: temporary,
                },
            );
        }

        let entry = self.buffers.get_mut(&key).unwrap();
        entry.reference += 1;

        entry.buffer.clone()
    }

    pub fn drop_buffer(&mut self, buffer: &wgpu::Buffer) {
        let key = self
            .buffers
            .iter_mut()
            .find(|(_, value)| &value.buffer == buffer);

        if key.is_none() {
            panic!("Buffer not found");
        }

        let key = key.unwrap();
        key.1.reference -= 1;

        if key.1.lifetime == 0 {
            let key = key.0.clone();
            self.buffers.remove(&key);
        }
    }

    pub fn cycle(&mut self) {
        let mut keys = Vec::new();

        for (key, entry) in self.buffers.iter_mut() {
            if entry.is_temporary {
                entry.lifetime += 1;
            }

            if entry.lifetime >= TEMPORARY_BUFFER_LIFETIME {
                keys.push(*key);
            }
        }

        for key in keys {
            self.buffers.remove(&key);
        }
    }
}
