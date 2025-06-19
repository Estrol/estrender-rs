use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use wgpu::CommandEncoder;

#[cfg(any(debug_assertions, feature = "enable-release-validation"))]
use crate::gpu::ShaderReflect;
use crate::{
    dbg_log,
    gpu::{
        BindGroupCreateInfo, BindGroupLayout, backed_command::BakedComputePass,
        pipeline_manager::ComputePipelineDesc, shader::ComputeShader,
    },
    prelude::{Buffer, BufferUsages, GPUInner, ShaderBindingType},
    utils::ArcRef,
};

use super::{BindGroupAttachment, BindGroupType};

#[derive(Clone, Debug, Hash)]
pub(crate) struct ComputeShaderBinding {
    pub shader: wgpu::ShaderModule,
    pub layout: Vec<BindGroupLayout>,
    pub entry_point: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ComputePassInner {
    pub cmd: ArcRef<CommandEncoder>,
    pub shader: Option<ComputeShaderBinding>,

    pub queues: Vec<ComputePassQueue>,
    pub attachments: Vec<BindGroupAttachment>,
    pub push_constant: Option<Vec<u8>>,

    #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
    pub reflection: Option<ShaderReflect>,
}

#[derive(Clone, Debug)]
pub struct ComputePass {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<ComputePassInner>,
}

impl ComputePass {
    pub(crate) fn new(graphics: ArcRef<GPUInner>, cmd: ArcRef<CommandEncoder>) -> Self {
        let inner = ComputePassInner {
            cmd,
            shader: None,

            queues: Vec::new(),
            attachments: Vec::new(),
            push_constant: None,

            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            reflection: None,
        };

        ComputePass {
            graphics,
            inner: ArcRef::new(inner),
        }
    }

    pub fn set_shader(&mut self, shader: Option<&ComputeShader>) {
        match shader {
            Some(shader) => {
                let shader_inner = shader.inner.borrow();

                let shader_entry_point = match &shader_inner.reflection {
                    ShaderReflect::Compute { entry_point, .. } => entry_point.clone(),
                    _ => panic!("Shader is not a compute shader"),
                };

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    if shader_entry_point.is_empty() {
                        panic!("Compute shader entry point is empty");
                    }
                }

                let layout = shader_inner.bind_group_layouts.clone();

                let shader_binding = ComputeShaderBinding {
                    shader: shader_inner.shader.clone(),
                    layout,
                    entry_point: shader_entry_point,
                };

                self.inner.borrow_mut().shader = Some(shader_binding);
            }
            None => {
                self.inner.borrow_mut().shader = None;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_push_constants(&mut self, push_constant: Option<&[u8]>) {
        let mut inner = self.inner.borrow_mut();

        match push_constant {
            Some(push_constant) => {
                let mut push_constant = push_constant.to_vec();
                if push_constant.len() % 4 != 0 {
                    push_constant.resize(push_constant.len() + (4 - push_constant.len() % 4), 0);
                }

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    if inner.shader.is_none() {
                        panic!("Shader must be set before setting push constants");
                    }

                    let size = {
                        let shader_reflection = inner.reflection.as_ref().unwrap();

                        match &shader_reflection {
                            ShaderReflect::Compute { bindings, .. } => bindings
                                .iter()
                                .find_map(|binding| {
                                    if let ShaderBindingType::PushConstant(size) = binding.ty {
                                        Some(size)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0),
                            _ => panic!("Shader is not a compute shader"),
                        }
                    };

                    if size == 0 {
                        panic!("No push constant found in shader");
                    }

                    if push_constant.len() > size as usize {
                        panic!("Push constant size is too large");
                    }
                }

                inner.push_constant = Some(push_constant);
            }
            None => {
                inner.push_constant = None;
            }
        }
    }

    pub fn set_attachment_buffer(&mut self, group: u32, binding: u32, attachment: Option<&Buffer>) {
        match attachment {
            Some(attachment) => {
                let buffer = attachment.inner.borrow().buffer.clone();

                self.insert_or_replace_attachment(
                    group,
                    binding,
                    BindGroupAttachment {
                        group,
                        binding,
                        attachment: BindGroupType::Storage(buffer),
                    },
                );
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }
    }

    pub fn set_attachment_buffer_raw<T>(
        &mut self,
        group: u32,
        binding: u32,
        attachment: Option<&[T]>,
        usages: BufferUsages,
    ) where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match attachment {
            Some(attachment) => {
                let buffer = self
                    .graphics
                    .borrow_mut()
                    .create_buffer_with(attachment, usages.into());

                self.insert_or_replace_attachment(
                    group,
                    binding,
                    BindGroupAttachment {
                        group,
                        binding,
                        attachment: BindGroupType::Storage(buffer),
                    },
                );
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }
    }

    pub(crate) fn remove_attachment(&mut self, group: u32, binding: u32) {
        let mut inner = self.inner.borrow_mut();

        inner
            .attachments
            .retain(|a| a.group != group || a.binding != binding);
    }

    pub(crate) fn insert_or_replace_attachment(
        &mut self,
        group: u32,
        binding: u32,
        attachment: BindGroupAttachment,
    ) {
        let mut inner = self.inner.borrow_mut();

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if inner.shader.is_none() {
                panic!("Shader is not set");
            }

            let reflection = inner.reflection.as_ref().unwrap();

            let r#type = match reflection {
                ShaderReflect::Compute { bindings, .. } => bindings
                    .iter()
                    .find_map(|shaderbinding| {
                        if shaderbinding.group == group && shaderbinding.binding == binding {
                            Some(shaderbinding)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "Shader does not have binding group: {} binding: {}",
                            group, binding
                        );
                    }),
                _ => panic!("Shader is not a compute shader"),
            };

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

        let index = inner
            .attachments
            .iter()
            .position(|a| a.group == group && a.binding == binding);

        if let Some(index) = index {
            inner.attachments[index] = attachment;
        } else {
            inner.attachments.push(attachment);
        }
    }

    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();

            if inner.shader.is_none() {
                panic!("Shader must be set before dispatching");
            }
        }

        let (pipeline, bind_group) = self.prepare_pipeline();
        let mut inner = self.inner.borrow_mut();

        let queue = ComputePassQueue {
            pipeline,
            bind_group,
            ty: DispatchType::Dispatch { x, y, z },
            push_constant: inner.push_constant.clone(),
            debug: None,
        };

        inner.queues.push(queue);
    }

    pub fn dispatch_indirect(&mut self, buffer: &Buffer, offset: u64) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();

            if inner.shader.is_none() {
                panic!("Shader must be set before dispatching");
            }
        }

        let (pipeline, bind_group) = self.prepare_pipeline();
        let mut inner = self.inner.borrow_mut();

        let queue = ComputePassQueue {
            pipeline,
            bind_group,
            ty: DispatchType::DispatchIndirect {
                buffer: buffer.inner.borrow().buffer.clone(),
                offset,
            },
            push_constant: inner.push_constant.clone(),
            debug: None,
        };

        inner.queues.push(queue);
    }

    fn prepare_pipeline(&self) -> (wgpu::ComputePipeline, Vec<(u32, wgpu::BindGroup)>) {
        let inner = self.inner.borrow();

        let shader_binding = inner
            .shader
            .as_ref()
            .expect("Shader must be set before preparing pipeline");

        let bind_group_hash_key = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            hasher.write_u64(1u64); // Compute shader hash id.

            for attachment in &inner.attachments {
                attachment.group.hash(&mut hasher);
                attachment.binding.hash(&mut hasher);

                match &attachment.attachment {
                    BindGroupType::Uniform(buffer) => {
                        buffer.hash(&mut hasher);
                    }
                    BindGroupType::Storage(buffer) => {
                        buffer.hash(&mut hasher);
                    }
                    BindGroupType::TextureStorage(texture) => {
                        texture.hash(&mut hasher);
                    }
                    BindGroupType::Sampler(sampler) => {
                        sampler.hash(&mut hasher);
                    }
                    BindGroupType::Texture(texture) => {
                        texture.hash(&mut hasher);
                    }
                }
            }

            hasher.finish()
        };

        let bind_group_attachments = {
            let mut gpu_inner = self.graphics.borrow_mut();

            match gpu_inner.get_bind_group(bind_group_hash_key) {
                Some(bind_group) => bind_group,
                None => {
                    let mut bind_group_attachments: HashMap<u32, Vec<wgpu::BindGroupEntry>> =
                        inner.attachments.iter().fold(HashMap::new(), |mut map, e| {
                            let (group, binding, attachment) = (e.group, e.binding, &e.attachment);

                            let entry = match attachment {
                                BindGroupType::TextureStorage(texture) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::TextureView(texture),
                                },
                                BindGroupType::Storage(buffer) => wgpu::BindGroupEntry {
                                    binding,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer,
                                        offset: 0,
                                        size: None,
                                    }),
                                },
                                _ => panic!("Unsupported bind group type"),
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

        let pipeline_hash_key = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            shader_binding.hash(&mut hasher);

            hasher.finish()
        };

        let pipeline = {
            let mut gpu_inner = self.graphics.borrow_mut();

            match gpu_inner.get_compute_pipeline(pipeline_hash_key) {
                Some(pipeline) => pipeline,
                None => {
                    let bind_group_layout = shader_binding
                        .layout
                        .iter()
                        .map(|l| l.layout.clone())
                        .collect::<Vec<_>>();

                    let entry_point = shader_binding.entry_point.as_str();

                    let pipeline_desc = ComputePipelineDesc {
                        shader_module: shader_binding.shader.clone(),
                        entry_point: entry_point.to_owned(),
                        bind_group_layout,
                    };

                    gpu_inner.create_compute_pipeline(pipeline_hash_key, pipeline_desc)
                }
            }
        };

        (pipeline, bind_group_attachments)
    }

    pub fn dispatch_baked(&mut self, baked: &BakedComputePass) {
        let pipeline_desc = baked.pipeline.clone();
        let mut inner = self.inner.borrow_mut();

        let pipeline_hash_key = {
            let mut hasher = DefaultHasher::new();
            pipeline_desc.hash(&mut hasher);

            hasher.finish()
        };

        let pipeline = {
            let mut gpu_inner = self.graphics.borrow_mut();

            match gpu_inner.get_compute_pipeline(pipeline_hash_key) {
                Some(pipeline) => pipeline,
                None => gpu_inner.create_compute_pipeline(pipeline_hash_key, pipeline_desc),
            }
        };

        let queue = ComputePassQueue {
            pipeline,
            bind_group: baked.bind_group.clone(),
            ty: DispatchType::Dispatch {
                x: baked.dispatch.0,
                y: baked.dispatch.1,
                z: baked.dispatch.2,
            },
            push_constant: inner.push_constant.clone(),
            debug: None,
        };

        inner.queues.push(queue);
    }

    fn end(&mut self) {
        let mut inner = self.inner.borrow_mut();

        let queues = inner.queues.drain(..).collect::<Vec<_>>();
        let mut cmd = inner.cmd.borrow_mut();

        let mut cpass = cmd.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        for queue in queues {
            cpass.set_pipeline(&queue.pipeline);

            for (bind_group_index, bind_group) in &queue.bind_group {
                cpass.set_bind_group(*bind_group_index, bind_group, &[]);
            }

            if let Some(debug) = &queue.debug {
                cpass.insert_debug_marker(debug);
            }

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(push_constant) = &queue.push_constant {
                cpass.set_push_constants(0, push_constant);
            }

            match &queue.ty {
                DispatchType::Dispatch { x, y, z } => {
                    cpass.dispatch_workgroups(*x, *y, *z);
                }
                DispatchType::DispatchIndirect { buffer, offset } => {
                    cpass.dispatch_workgroups_indirect(buffer, *offset);
                }
            }
        }
    }
}

impl Drop for ComputePass {
    fn drop(&mut self) {
        self.end();
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum DispatchType {
    Dispatch { x: u32, y: u32, z: u32 },
    DispatchIndirect { buffer: wgpu::Buffer, offset: u64 },
}

#[derive(Clone, Debug)]
pub(crate) struct ComputePassQueue {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: Vec<(u32, wgpu::BindGroup)>,
    pub ty: DispatchType,
    pub push_constant: Option<Vec<u8>>,

    pub debug: Option<String>,
}
