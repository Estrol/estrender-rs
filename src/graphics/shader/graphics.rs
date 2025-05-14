use std::collections::HashMap;

use wgpu::{BindingType, SamplerBindingType, naga::front::wgsl};

use crate::{graphics::inner::GPUInner, math::Vertex, utils::ArcRef};

use super::{
    BindGroupLayout, IndexBufferSize, ShaderBindingType, ShaderCullMode, ShaderFrontFace,
    ShaderPollygonMode, ShaderReflect, ShaderTopology, StorageAccess, VertexInputDesc,
    reflection::parse,
};

pub struct GraphicsShaderBuilder {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) wgls_data: String,
}

impl GraphicsShaderBuilder {
    pub fn new(graphics: ArcRef<GPUInner>) -> Self {
        Self {
            graphics,
            wgls_data: String::new(),
        }
    }

    pub fn with_file(mut self, path: &str) -> Self {
        let data = std::fs::read_to_string(path);
        if let Err(err) = data {
            panic!("Failed to read shader file: {:?}", err);
        }

        self.wgls_data = data.unwrap();
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.wgls_data = source.to_string();
        self
    }

    pub fn build(self) -> Result<GraphicsShader, String> {
        GraphicsShader::new(self.graphics, &self.wgls_data)
    }
}

pub struct GraphicsShaderInner {
    pub shader: wgpu::ShaderModule,
    pub reflection: ShaderReflect,

    pub topology: ShaderTopology,
    pub cull_mode: Option<ShaderCullMode>,
    pub polygon_mode: ShaderPollygonMode,
    pub front_face: ShaderFrontFace,

    pub index_buffer_size: IndexBufferSize,
    pub vertex_input_desc: VertexInputDesc<'static>,

    pub bind_group_layouts: Vec<BindGroupLayout>,
}

#[derive(Clone)]
#[allow(unused)]
pub struct GraphicsShader {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<GraphicsShaderInner>,
}

impl GraphicsShader {
    pub fn new(graphics: ArcRef<GPUInner>, wgls_data: &str) -> Result<Self, String> {
        if graphics.borrow().is_invalid {
            panic!("Graphics context is invalid");
        }

        let module = wgsl::parse_str(wgls_data);
        if let Err(err) = module {
            return Err(format!("Failed to parse shader: {:?}", err));
        }

        let module = module.unwrap();
        let reflect = parse(module);

        let device_ref = &graphics.borrow().device;

        let shader = device_ref.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(wgls_data.into()),
        });

        let layout = Self::make_group_layout(device_ref, &reflect);

        let inner = GraphicsShaderInner {
            shader,
            reflection: reflect,
            topology: ShaderTopology::TriangleList,
            cull_mode: None,
            polygon_mode: ShaderPollygonMode::Fill,
            front_face: ShaderFrontFace::Clockwise,
            index_buffer_size: IndexBufferSize::U16,
            vertex_input_desc: Vertex::desc(),
            bind_group_layouts: layout,
        };

        Ok(Self {
            graphics: ArcRef::clone(&graphics),
            inner: ArcRef::new(inner),
        })
    }

    fn make_group_layout(device: &wgpu::Device, reflect: &ShaderReflect) -> Vec<BindGroupLayout> {
        let mut layouts: HashMap<u32, Vec<wgpu::BindGroupLayoutEntry>> = HashMap::new();
        for binding in reflect.bindings.iter() {
            if let ShaderBindingType::PushConstant(_) = binding.ty {
                continue;
            }

            let layout_desc = wgpu::BindGroupLayoutEntry {
                ty: match binding.ty {
                    ShaderBindingType::UniformBuffer(size) => BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: if size == u32::MAX {
                            None
                        } else {
                            wgpu::BufferSize::new(size as u64)
                        },
                    },

                    ShaderBindingType::Texture => BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },

                    ShaderBindingType::Sampler(comparsion) => BindingType::Sampler(if comparsion {
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
                        access: {
                            if access.contains(StorageAccess::READ)
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
                            }
                        },
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },

                    _ => unreachable!(),
                },
                binding: binding.binding,
                visibility: match binding.ty {
                    ShaderBindingType::UniformBuffer(_) => wgpu::ShaderStages::all(),
                    ShaderBindingType::Texture => wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ShaderBindingType::Sampler(_) => wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ShaderBindingType::StorageBuffer(_, _) => wgpu::ShaderStages::all(),
                    ShaderBindingType::StorageTexture(_) => wgpu::ShaderStages::COMPUTE,
                    _ => unreachable!(),
                },
                count: None,
            };

            let group = layouts.entry(binding.group).or_insert_with(Vec::new);
            group.push(layout_desc);
        }

        layouts
            .into_iter()
            .map(|(group, layout)| {
                let bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: &layout,
                    });

                BindGroupLayout {
                    group,
                    binding: layout[0].binding,
                    layout: bind_group_layout,
                }
            })
            .collect()
    }
}
