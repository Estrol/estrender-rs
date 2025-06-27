use std::collections::HashMap;

use crate::gpu::BindGroupLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindGroupManager {
    pub bind_groups: HashMap<usize, (Vec<(u32, wgpu::BindGroup)>, usize)>,
}

const BIND_GROUP_LIFETIME: usize = 100;

#[derive(Debug, Clone)]
pub struct BindGroupCreateInfo<'a> {
    pub entries: Vec<(&'a BindGroupLayout, &'a [wgpu::BindGroupEntry<'a>])>,
}

impl BindGroupManager {
    pub fn new() -> Self {
        Self {
            bind_groups: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: usize) -> Option<Vec<(u32, wgpu::BindGroup)>> {
        if let Some((bind_groups, lifetime)) = self.bind_groups.get_mut(&key) {
            // reset lifetime
            *lifetime = 0;

            Some(bind_groups.clone())
        } else {
            None
        }
    }

    pub fn create(
        &mut self,
        key: usize,
        device: &wgpu::Device,
        info: BindGroupCreateInfo,
    ) -> Vec<(u32, wgpu::BindGroup)> {
        let mut bind_groups = Vec::new();

        for (layout, entries) in info.entries {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout.layout,
                entries,
                label: None,
            });

            bind_groups.push((layout.group, bind_group));
        }

        self.bind_groups.insert(key, (bind_groups.clone(), 0));

        bind_groups
    }

    pub fn cycle(&mut self) {
        self.bind_groups
            .retain(|_, value| value.1 < BIND_GROUP_LIFETIME);

        for (_, value) in self.bind_groups.iter_mut() {
            value.1 += 1;
        }
    }
}
