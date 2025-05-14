use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

pub struct BindGroupManager {
    pub bind_groups: HashMap<usize, (wgpu::BindGroup, usize)>,
}

const BIND_GROUP_LIFETIME: usize = 100;

impl BindGroupManager {
    pub fn new() -> Self {
        Self {
            bind_groups: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        attachment: &[wgpu::BindGroupEntry],
    ) -> wgpu::BindGroup {
        let mut hasher = DefaultHasher::new();
        layout.hash(&mut hasher);

        for entry in attachment {
            entry.binding.hash(&mut hasher);

            match &entry.resource {
                wgpu::BindingResource::Buffer(buffer) => {
                    buffer.buffer.hash(&mut hasher);
                    buffer.offset.hash(&mut hasher);
                    buffer.size.hash(&mut hasher);
                }
                wgpu::BindingResource::TextureView(texture_view) => {
                    texture_view.hash(&mut hasher);
                }
                wgpu::BindingResource::Sampler(sampler) => {
                    sampler.hash(&mut hasher);
                }
                _ => {}
            }
        }

        let key = hasher.finish() as usize;

        if !self.bind_groups.contains_key(&key) {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: attachment,
                label: None,
            });

            self.bind_groups.insert(key, (bind_group, 0));
        }

        // reset lifetime
        self.bind_groups.get_mut(&key).unwrap().1 = 0;

        self.bind_groups.get(&key).unwrap().0.clone()
    }

    pub fn cycle(&mut self) {
        self.bind_groups
            .retain(|_, value| value.1 < BIND_GROUP_LIFETIME);

        for (_, value) in self.bind_groups.iter_mut() {
            value.1 += 1;
        }
    }
}
