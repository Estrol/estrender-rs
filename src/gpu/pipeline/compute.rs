use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::utils::ArcRef;

use super::{
    manager::ComputePipelineDesc,
    super::{
        GPUInner,
        texture::{Texture, TextureSampler},
        buffer::Buffer,
        command::{
            BindGroupAttachment,
            utils::BindGroupType,
            computepass::IntermediateComputeBinding
        },
        shader::{
            bind_group_manager::BindGroupCreateInfo,
            types::{ShaderReflect, ShaderBindingType},
            compute::ComputeShader,
        },
    },
};

#[derive(Debug, Clone, Hash)]
pub struct ComputePipeline {
    pub(crate) bind_group: Vec<(u32, wgpu::BindGroup)>,
    pub(crate) pipeline_desc: ComputePipelineDesc,
}

#[derive(Debug, Clone)]
pub struct ComputePipelineBuilder {
    pub(crate) gpu: ArcRef<GPUInner>,
    pub(crate) attachments: Vec<BindGroupAttachment>,
    pub(crate) shader: Option<IntermediateComputeBinding>,
    pub(crate) shader_reflection: Option<ShaderReflect>,
}

impl ComputePipelineBuilder {
    pub(crate) fn new(gpu: ArcRef<GPUInner>) -> Self {
        Self {
            gpu,
            attachments: Vec::new(),
            shader: None,
            shader_reflection: None,
        }
    }

    #[inline]
    pub fn set_shader(mut self, shader: Option<&ComputeShader>) -> Self {
        match shader {
            Some(shader) => {
                let shader_inner = shader.inner.borrow();
                let shader_module = shader_inner.shader.clone();
                let layout = shader_inner.bind_group_layouts.clone();

                let shader_reflect = shader_inner.reflection.clone();
                let entry_point = match &shader_reflect {
                    ShaderReflect::Compute { entry_point, .. } => entry_point.clone(),
                    _ => panic!("Shader must be a compute shader"),
                };

                let shader_binding = IntermediateComputeBinding {
                    shader: shader_module,
                    entry_point,
                    layout,
                };

                self.shader = Some(shader_binding);
                self.shader_reflection = Some(shader_reflect);
            }
            None => {
                self.shader = None;
                self.shader_reflection = None;
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
                            sampler.make_wgpu(gpu_inner.device()),
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

    pub fn build(self) -> Result<ComputePipeline, CompuitePipelineError> {
        if self.shader.is_none() {
            return Err(CompuitePipelineError::ShaderNotSet);
        }

        let shader_binding = self.shader.unwrap();
        for attachment in &self.attachments {
            let r#type = {
                let shader_reflection = self.shader_reflection.as_ref().unwrap();

                match shader_reflection {
                    ShaderReflect::Compute { bindings, .. } => bindings
                        .iter()
                        .find(|b| b.group == attachment.group && b.binding == attachment.binding),
                    _ => None,
                }
            };

            if r#type.is_none() {
                return Err(CompuitePipelineError::AttachmentNotSet(
                    attachment.group,
                    attachment.binding,
                ));
            }

            let r#type = r#type.unwrap();

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
                return Err(CompuitePipelineError::InvalidAttachmentType(
                    attachment.group,
                    attachment.binding,
                    r#type.ty,
                ));
            }
        }

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

        let layout = shader_binding
            .layout
            .iter()
            .map(|l| l.layout.clone())
            .collect::<Vec<_>>();

        let pipeline_desc = ComputePipelineDesc {
            shader_module: shader_binding.shader,
            entry_point: shader_binding.entry_point,
            bind_group_layout: layout,
        };

        let pipeline = ComputePipeline {
            bind_group: bind_group_attachments,
            pipeline_desc,
        };

        Ok(pipeline)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompuitePipelineError {
    ShaderNotSet,
    InvalidShaderType,
    AttachmentNotSet(u32, u32),
    InvalidAttachmentType(u32, u32, ShaderBindingType),
}
