use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    gpu::{
        BindGroupCreateInfo, BindGroupType, Buffer, GraphicsPipelineDesc, GraphicsShader,
        GraphicsShaderBinding, IndexBufferSize, ShaderBindingType, ShaderCullMode, ShaderFrontFace,
        ShaderPollygonMode, ShaderReflect, ShaderTopology, ShaderType, Texture, TextureBlend,
        TextureSampler, VertexAttributeLayout,
    },
    utils::ArcRef,
};

use super::{BindGroupAttachment, GPUInner};

#[derive(Debug, Clone, Hash)]
pub struct RenderPipeline {
    pub(crate) bind_group: Vec<(u32, wgpu::BindGroup)>,
    pub(crate) pipeline_desc: GraphicsPipelineDesc,
    pub(crate) index_format: Option<IndexBufferSize>,
}

#[derive(Debug, Clone)]
pub struct RenderPipelineBuilder {
    pub(crate) gpu: ArcRef<GPUInner>,
    pub(crate) attachments: Vec<BindGroupAttachment>,
    pub(crate) shader: Option<GraphicsShaderBinding>,
    pub(crate) blend: Option<wgpu::BlendState>,
    pub(crate) color_write_mask: Option<wgpu::ColorWrites>,
    #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
    pub(crate) shader_reflection: Option<Vec<ShaderReflect>>,
}

impl RenderPipelineBuilder {
    pub(crate) fn new(gpu: ArcRef<GPUInner>) -> Self {
        Self {
            gpu,
            attachments: Vec::new(),
            shader: None,
            blend: None,
            color_write_mask: None,
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            shader_reflection: None,
        }
    }

    #[inline]
    pub fn set_blend(mut self, blend: Option<&TextureBlend>) -> Self {
        match blend {
            Some(blend) => {
                self.blend = Some(blend.clone().into());
                self.color_write_mask = Some(blend.clone().into());
            }
            None => {
                self.blend = None;
                self.color_write_mask = None;
            }
        }

        self
    }

    #[inline]
    pub fn set_shader(self, shader: Option<&GraphicsShader>) -> Self {
        self.set_shader_with_options(shader, None, None, None, None, None)
    }

    #[inline]
    pub fn set_shader_with_options(
        mut self,
        shader: Option<&GraphicsShader>,
        topology: Option<ShaderTopology>,
        cull_mode: Option<ShaderCullMode>,
        front_face: Option<ShaderFrontFace>,
        polygon_mode: Option<ShaderPollygonMode>,
        index_format: Option<IndexBufferSize>,
    ) -> Self {
        match shader {
            Some(shader) => {
                let shader_inner = shader.inner.borrow();
                let (vertex_shader, fragment_shader) = match &shader_inner.ty {
                    ShaderType::GraphicsSplit {
                        vertex_module,
                        fragment_module,
                    } => (vertex_module.clone(), fragment_module.clone()),
                    ShaderType::GraphicsSingle { module } => (module.clone(), module.clone()),
                    _ => panic!("Shader must be a graphics shader"),
                };

                let layout = shader_inner.bind_group_layouts.clone();

                let vertex_reflect = shader_inner.reflection.iter().find(|r| {
                    matches!(r, ShaderReflect::Vertex { .. })
                        || matches!(r, ShaderReflect::VertexFragment { .. })
                });

                let fragment_reflect = shader_inner.reflection.iter().find(|r| {
                    matches!(r, ShaderReflect::Fragment { .. })
                        || matches!(r, ShaderReflect::VertexFragment { .. })
                });

                let vertex_entry_point = match vertex_reflect {
                    Some(ShaderReflect::Vertex { entry_point, .. }) => Some(entry_point),
                    Some(ShaderReflect::VertexFragment {
                        vertex_entry_point, ..
                    }) => Some(vertex_entry_point),
                    _ => None,
                };

                let fragment_entry_point = match fragment_reflect {
                    Some(ShaderReflect::Fragment { entry_point, .. }) => Some(entry_point),
                    Some(ShaderReflect::VertexFragment {
                        fragment_entry_point,
                        ..
                    }) => Some(fragment_entry_point),
                    _ => None,
                };

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    if vertex_entry_point.is_none() {
                        panic!("Vertex shader entry point is not found in shader reflection");
                    }

                    if fragment_entry_point.is_none() {
                        panic!("Fragment shader entry point is not found in shader reflection");
                    }
                }

                let vertex_entry_point = vertex_entry_point.unwrap();
                let fragment_entry_point = fragment_entry_point.unwrap();

                let attrib_inner = shader.attrib.borrow();
                let shader_binding = GraphicsShaderBinding {
                    shader: (vertex_shader, fragment_shader),
                    vertex_attribute: (attrib_inner.stride, attrib_inner.attributes.clone()),
                    shader_entry: (vertex_entry_point.clone(), fragment_entry_point.clone()),
                    layout: layout,
                    topology: topology.unwrap_or(attrib_inner.topology),
                    cull_mode: cull_mode.into(),
                    front_face: front_face.unwrap_or(attrib_inner.front_face),
                    polygon_mode: polygon_mode.unwrap_or(attrib_inner.polygon_mode),
                    index_format: index_format.or_else(|| attrib_inner.index.clone()),
                };

                self.shader = Some(shader_binding);

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    self.shader_reflection = Some(shader_inner.reflection.clone());
                }
            }
            None => {
                self.shader = None;

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    self.shader_reflection = None;
                }
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_sampler(
        mut self,
        group: u32,
        binding: u32,
        sampler: Option<&TextureSampler>,
    ) -> Self {
        match sampler {
            Some(sampler) => {
                let attachment = {
                    let gpu_inner = self.gpu.borrow();

                    BindGroupAttachment {
                        group,
                        binding,
                        attachment: BindGroupType::Sampler(
                            sampler.make_wgpu(gpu_inner.get_device()),
                        ),
                    }
                };

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_texture(
        mut self,
        group: u32,
        binding: u32,
        texture: Option<&Texture>,
    ) -> Self {
        match texture {
            Some(texture) => {
                let attachment = {
                    BindGroupAttachment {
                        group,
                        binding,
                        attachment: BindGroupType::Texture(
                            texture.inner.borrow().wgpu_view.clone(),
                        ),
                    }
                };

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_texture_storage(
        mut self,
        group: u32,
        binding: u32,
        texture: Option<&Texture>,
    ) -> Self {
        match texture {
            Some(texture) => {
                let inner = texture.inner.borrow();
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::TextureStorage(inner.wgpu_view.clone()),
                };

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_uniform(
        mut self,
        group: u32,
        binding: u32,
        buffer: Option<&Buffer>,
    ) -> Self {
        match buffer {
            Some(buffer) => {
                let inner = buffer.inner.borrow();
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Uniform(inner.buffer.clone()),
                };

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_uniform_vec<T>(
        mut self,
        group: u32,
        binding: u32,
        buffer: Option<Vec<T>>,
    ) -> Self
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let attachment = {
                    let mut inner = self.gpu.borrow_mut();

                    let buffer = inner.create_buffer_with(&buffer, wgpu::BufferUsages::COPY_DST);
                    BindGroupAttachment {
                        group,
                        binding,
                        attachment: BindGroupType::Uniform(buffer),
                    }
                };

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_uniform_raw<T>(
        mut self,
        group: u32,
        binding: u32,
        buffer: Option<&[T]>,
    ) -> Self
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.gpu.borrow_mut();

                let buffer = inner.create_buffer_with(&buffer, wgpu::BufferUsages::COPY_DST);
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Uniform(buffer),
                };

                drop(inner);

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_storage(
        mut self,
        group: u32,
        binding: u32,
        buffer: Option<&Buffer>,
    ) -> Self {
        match buffer {
            Some(buffer) => {
                let inner = buffer.inner.borrow();
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Storage(inner.buffer.clone()),
                };

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_storage_raw<T>(
        mut self,
        group: u32,
        binding: u32,
        buffer: Option<&[T]>,
    ) -> Self
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.gpu.borrow_mut();

                let buffer = inner.create_buffer_with(&buffer, wgpu::BufferUsages::COPY_DST);
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Storage(buffer),
                };

                drop(inner);

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub fn set_attachment_storage_vec<T>(
        mut self,
        group: u32,
        binding: u32,
        buffer: Option<Vec<T>>,
    ) -> Self
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.gpu.borrow_mut();

                let buffer = inner.create_buffer_with(&buffer, wgpu::BufferUsages::COPY_DST);
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Storage(buffer),
                };

                drop(inner);

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }

        self
    }

    #[inline]
    pub(crate) fn remove_attachment(&mut self, group: u32, binding: u32) {
        self.attachments
            .retain(|a| a.group != group || a.binding != binding);
    }

    pub(crate) fn insert_or_replace_attachment(
        &mut self,
        group: u32,
        binding: u32,
        attachment: BindGroupAttachment,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if self.shader.is_none() {
                panic!("Shader is not set");
            }

            let r#type = self
                .shader_reflection
                .as_ref()
                .unwrap()
                .iter()
                .find_map(|b| {
                    let bindings = match b {
                        ShaderReflect::Vertex { bindings, .. }
                        | ShaderReflect::Fragment { bindings, .. }
                        | ShaderReflect::VertexFragment { bindings, .. } => bindings,
                        _ => return None,
                    };

                    bindings.iter().find_map(|shaderbinding| {
                        if shaderbinding.group == group && shaderbinding.binding == binding {
                            Some(shaderbinding)
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| {
                    panic!(
                        "Shader does not have binding group: {} binding: {}",
                        group, binding
                    );
                });

            if !match r#type.ty {
                ShaderBindingType::UniformBuffer(_) => {
                    matches!(attachment.attachment, BindGroupType::Uniform(_))
                }
                ShaderBindingType::StorageBuffer(_, _) => {
                    matches!(attachment.attachment, BindGroupType::Storage(_))
                }
                ShaderBindingType::StorageTexture(_) => {
                    matches!(attachment.attachment, BindGroupType::TextureStorage(_))
                }
                ShaderBindingType::Sampler(_) => {
                    matches!(attachment.attachment, BindGroupType::Sampler(_))
                }
                ShaderBindingType::Texture(_) => {
                    matches!(attachment.attachment, BindGroupType::Texture(_))
                }
                ShaderBindingType::PushConstant(_) => {
                    matches!(attachment.attachment, BindGroupType::Uniform(_))
                }
            } {
                panic!(
                    "Attachment group: {} binding: {} type: {} not match with shader type: {}",
                    group, binding, attachment.attachment, r#type.ty
                );
            }
        }

        let index = self
            .attachments
            .iter()
            .position(|a| a.group == group && a.binding == binding);

        if let Some(index) = index {
            self.attachments[index] = attachment;
        } else {
            self.attachments.push(attachment);
        }
    }

    pub fn build(self) -> Result<RenderPipeline, RenderPipelineError> {
        let shader_binding = self.shader.as_ref().unwrap();

        let bind_group_hash_key = {
            let mut hasher = DefaultHasher::new();
            hasher.write_u64(0u64); // Graphics shader hash id

            for attachment in &self.attachments {
                attachment.group.hash(&mut hasher);
                attachment.binding.hash(&mut hasher);
                match &attachment.attachment {
                    BindGroupType::Uniform(uniform) => {
                        uniform.hash(&mut hasher);
                    }
                    BindGroupType::Texture(texture) => {
                        texture.hash(&mut hasher);
                    }
                    BindGroupType::TextureStorage(texture) => texture.hash(&mut hasher),
                    BindGroupType::Sampler(sampler) => sampler.hash(&mut hasher),
                    BindGroupType::Storage(storage) => storage.hash(&mut hasher),
                }
            }

            hasher.finish()
        };

        let bind_group_attachments = {
            let mut gpu_inner = self.gpu.borrow_mut();

            match gpu_inner.get_bind_group(bind_group_hash_key) {
                Some(bind_group) => bind_group,
                None => {
                    let mut bind_group_attachments: HashMap<u32, Vec<wgpu::BindGroupEntry>> =
                        self.attachments.iter().fold(HashMap::new(), |mut map, e| {
                            let (group, binding, attachment) = (e.group, e.binding, &e.attachment);

                            let entry = match attachment {
                                BindGroupType::Uniform(buffer) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer,
                                        offset: 0,
                                        size: None,
                                    }),
                                },
                                BindGroupType::Texture(texture) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::TextureView(texture),
                                },
                                BindGroupType::Sampler(sampler) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::Sampler(sampler),
                                },
                                BindGroupType::Storage(buffer) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer,
                                        offset: 0,
                                        size: None,
                                    }),
                                },
                                BindGroupType::TextureStorage(texture) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::TextureView(texture),
                                },
                            };

                            map.entry(group).or_insert_with(Vec::new).push(entry);
                            map
                        });

                    // sort each group attachments
                    // group, binding
                    // this is important for the bind group to be created in the correct order
                    for entries in bind_group_attachments.values_mut() {
                        entries.sort_by_key(|e| e.binding);
                    }

                    let bind_group = bind_group_attachments
                        .iter()
                        .map(|(group, entries)| {
                            let layout = shader_binding
                                .layout
                                .iter()
                                .find(|l| l.group == *group)
                                .unwrap();

                            (layout, entries.as_slice())
                        })
                        .collect::<Vec<_>>();

                    let create_info = BindGroupCreateInfo {
                        entries: bind_group,
                    };

                    gpu_inner.create_bind_group(bind_group_hash_key, create_info)
                }
            }
        };

        let attribute = &shader_binding.vertex_attribute;
        let vertex_desc = VertexAttributeLayout {
            stride: attribute.0 as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: attribute.1.clone(),
        };

        let primitive_state = wgpu::PrimitiveState {
            topology: shader_binding.topology.into(),
            strip_index_format: None,
            front_face: shader_binding.front_face.into(),
            cull_mode: shader_binding.cull_mode.map(|c| c.into()),
            polygon_mode: shader_binding.polygon_mode.into(),
            unclipped_depth: false,
            conservative: false,
        };

        let layout = shader_binding
            .layout
            .iter()
            .map(|l| l.layout.clone())
            .collect::<Vec<_>>();

        let pipeline_desc = GraphicsPipelineDesc {
            shaders: shader_binding.shader.clone(),
            entry_point: shader_binding.shader_entry.clone(),
            render_target: wgpu::TextureFormat::Rgba8UnormSrgb,
            depth_stencil: None,
            blend_state: self.blend.clone(),
            write_mask: self.color_write_mask.clone(),
            vertex_desc,
            primitive_state,
            bind_group_layout: layout,
            msaa_count: 1,
        };

        Ok(RenderPipeline {
            bind_group: bind_group_attachments,
            pipeline_desc,
            index_format: shader_binding.index_format,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderPipelineError {
    ShaderNotSet,
    BindGroupNotFound,
    InvalidAttachmentType,
}
