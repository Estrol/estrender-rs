use std::collections::HashMap;

use wgpu::{BindingType, SamplerBindingType, naga::front::wgsl};

use crate::utils::ArcRef;
use super::{
    super::GPUInner,
    types::{
        ShaderReflect, BindGroupLayout,
        ShaderBindingType, StorageAccess,
    }
};

pub struct ComputeShaderBuilder {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) wgls_data: String,
}

impl ComputeShaderBuilder {
    pub(crate) fn new(graphics: ArcRef<GPUInner>) -> Self {
        Self {
            graphics,
            wgls_data: String::new(),
        }
    }

    pub fn set_file(mut self, path: &str) -> Self {
        let data = std::fs::read_to_string(path);
        if let Err(err) = data {
            panic!("Failed to read shader file: {:?}", err);
        }

        self.wgls_data = data.unwrap();
        self
    }

    pub fn set_source(mut self, source: &str) -> Self {
        self.wgls_data = source.to_string();
        self
    }

    pub fn build(self) -> Result<ComputeShader, String> {
        ComputeShader::new(self.graphics, &self.wgls_data)
    }
}

pub(crate) struct ComputeShaderInner {
    pub shader: wgpu::ShaderModule,
    pub reflection: ShaderReflect,

    pub bind_group_layouts: Vec<BindGroupLayout>,
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct ComputeShader {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<ComputeShaderInner>,
}

impl ComputeShader {
    pub(crate) fn new(graphics: ArcRef<GPUInner>, wgls_data: &str) -> Result<Self, String> {
        if graphics.borrow().is_invalid {
            panic!("Graphics context is invalid");
        }

        let module = wgsl::parse_str(wgls_data);
        if let Err(err) = module {
            return Err(format!("Failed to parse shader: {:?}", err));
        }

        let module = module.unwrap();
        let reflect = super::reflection::parse(module);

        if reflect.is_err() {
            return Err(format!("Failed to reflect shader: {:?}", reflect.err()));
        }

        let reflect = reflect.unwrap();

        let graphics_ref = graphics.borrow();
        let device_ref = graphics_ref.device();

        let shader = device_ref.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(wgls_data.into()),
        });

        let bind_group_layouts = Self::make_group_layout(device_ref, &[reflect.clone()]);

        let inner = ComputeShaderInner {
            shader,
            reflection: reflect,
            bind_group_layouts,
        };

        Ok(Self {
            graphics: ArcRef::clone(&graphics),
            inner: ArcRef::new(inner),
        })
    }

    fn create_layout_ty(ty: ShaderBindingType) -> wgpu::BindingType {
        match ty {
            ShaderBindingType::UniformBuffer(size) => BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: if size == u32::MAX {
                    None
                } else {
                    wgpu::BufferSize::new(size as u64)
                },
            },
            ShaderBindingType::Texture(multisampled) => BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
            ShaderBindingType::Sampler(comparison) => BindingType::Sampler(if comparison {
                SamplerBindingType::Comparison
            } else {
                SamplerBindingType::Filtering
            }),
            ShaderBindingType::StorageBuffer(size, access) => BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: access.contains(StorageAccess::READ)
                        && !access.contains(StorageAccess::WRITE),
                },
                has_dynamic_offset: false,
                min_binding_size: if size == u32::MAX {
                    None
                } else {
                    wgpu::BufferSize::new(size as u64)
                },
            },
            ShaderBindingType::StorageTexture(access) => BindingType::StorageTexture {
                access: if access.contains(StorageAccess::READ)
                    && access.contains(StorageAccess::WRITE)
                {
                    wgpu::StorageTextureAccess::ReadWrite
                } else if access.contains(StorageAccess::READ) {
                    wgpu::StorageTextureAccess::ReadOnly
                } else if access.contains(StorageAccess::WRITE) {
                    wgpu::StorageTextureAccess::WriteOnly
                } else if access.contains(StorageAccess::ATOMIC) {
                    wgpu::StorageTextureAccess::Atomic
                } else {
                    panic!("Invalid storage texture access")
                },
                format: wgpu::TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            _ => unreachable!(),
        }
    }

    fn make_group_layout(
        device: &wgpu::Device,
        reflects: &[ShaderReflect],
    ) -> Vec<BindGroupLayout> {
        let mut layouts: HashMap<u32, Vec<wgpu::BindGroupLayoutEntry>> = HashMap::new();

        for reflect in reflects {
            match reflect {
                ShaderReflect::Compute { bindings, .. } => {
                    for binding in bindings.iter() {
                        let ty = Self::create_layout_ty(binding.ty.clone());

                        // Push new layout entry
                        let layout_desc = wgpu::BindGroupLayoutEntry {
                            ty,
                            binding: binding.binding,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            count: None,
                        };

                        let group = layouts.entry(binding.group).or_insert_with(Vec::new);

                        group.push(layout_desc);
                    }
                }
                _ => continue,
            }
        }

        layouts
            .into_iter()
            .map(|(group, layout)| {
                // Label: "BindGroupLayout for group {group}, binding: {binding} (ex: 0, 1, 2)"
                let label = if !layout.is_empty() {
                    let mut s = format!("BindGroupLayout for group {}, binding: ", group);
                    for (i, entry) in layout.iter().enumerate() {
                        s.push_str(&entry.binding.to_string());
                        if i != layout.len() - 1 {
                            s.push_str(", ");
                        }
                    }
                    Some(s)
                } else {
                    None
                };

                let bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: label.as_deref(),
                        entries: &layout,
                    });

                BindGroupLayout {
                    group,
                    bindings: layout.iter().map(|entry| entry.binding).collect(),
                    layout: bind_group_layout,
                }
            })
            .collect()
    }

    pub fn get_uniform_location(&self, name: &str) -> Option<(u32, u32)> {
        let reflection = self.inner.borrow().reflection.clone();
        match reflection {
            ShaderReflect::Compute { bindings, .. } => bindings.iter().find_map(|binding| {
                if binding.name == name && matches!(binding.ty, ShaderBindingType::UniformBuffer(_))
                {
                    Some((binding.group, binding.binding))
                } else {
                    None
                }
            }),
            _ => None,
        }
    }
}
