use std::collections::HashMap;

use wgpu::CommandEncoder;

use crate::{
    graphics::{pipeline_manager::ComputePipelineDesc, shader::ComputeShader},
    prelude::inner::GPUInner,
    utils::ArcRef,
};

pub struct ComputePass<'a> {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) cmd: &'a mut CommandEncoder,
    pub(crate) queues: Vec<ComputePassQueue>,

    pub(crate) current_shader: Option<&'a ComputeShader>,
    pub(crate) current_attachment: Vec<BindGroupAttachment>,
}

pub enum BindGroupType {
    Texture(wgpu::TextureView),
    Buffer(wgpu::Buffer),
}

pub struct BindGroupAttachment {
    pub index: u32,
    pub binding: u32,
    pub attachment: BindGroupType,
}

impl<'a> ComputePass<'a> {
    pub(crate) fn new(graphics: &ArcRef<GPUInner>, cmd: &'a mut CommandEncoder) -> ComputePass<'a> {
        ComputePass {
            graphics: ArcRef::clone(&graphics),
            cmd,
            queues: Vec::new(),
            current_shader: None,
            current_attachment: Vec::new(),
        }
    }

    pub fn set_shader(&mut self, _shader: &'a ComputeShader) {
        self.current_shader = Some(_shader);
    }

    pub fn set_attachment_texture(
        &mut self,
        _index: u32,
        _binding: u32,
        _attachment: BindGroupType,
    ) {
        assert!(
            self.current_shader.is_some(),
            "Shader must be set before setting attachments"
        );
        assert!(
            self.current_attachment.len() < 16,
            "Too many attachments set"
        );

        if let Some(attachment) = self
            .current_attachment
            .iter_mut()
            .find(|a| a.index == _index)
        {
            attachment.attachment = _attachment;
        } else {
            self.current_attachment.push(BindGroupAttachment {
                index: _index,
                binding: _binding,
                attachment: _attachment,
            });
        }
    }

    pub fn dispatch(&mut self, _x: u32, _y: u32, _z: u32) {
        assert!(
            self.current_shader.is_some(),
            "Shader must be set before dispatching"
        );
        assert!(self.current_attachment.len() > 0, "No attachments set");

        let shader_binding = self.current_shader.as_ref().unwrap();
        let shader = shader_binding.inner.borrow();

        let bind_group_attachments: HashMap<u32, Vec<wgpu::BindGroupEntry>> = self
            .current_attachment
            .iter()
            .fold(HashMap::new(), |mut map, e| {
                let (group, binding, attachment) = (e.index, e.binding, &e.attachment);

                let entry = match attachment {
                    BindGroupType::Texture(texture) => wgpu::BindGroupEntry {
                        binding,
                        resource: wgpu::BindingResource::TextureView(texture),
                    },
                    BindGroupType::Buffer(buffer) => wgpu::BindGroupEntry {
                        binding,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                };

                map.entry(group).or_insert_with(Vec::new).push(entry);
                map
            });

        let bind_group = bind_group_attachments
            .iter()
            .map(|(group, entries)| {
                let layout = shader
                    .bind_group_layouts
                    .iter()
                    .find(|l| l.binding == *group)
                    .unwrap();
                let bind_group = self
                    .graphics
                    .borrow_mut()
                    .insert_bind_group(&layout.layout, entries);
                (*group, bind_group)
            })
            .collect::<Vec<_>>();

        let bind_group_layout = shader
            .bind_group_layouts
            .iter()
            .map(|l| &l.layout)
            .collect::<Vec<_>>();

        let entry_point = shader.reflection.compute_entry_point.as_str();

        let pipeline_desc = ComputePipelineDesc {
            shader_module: &shader.shader,
            entry_point,
            bind_group_layout,
        };

        let pipeline = {
            let mut graphics = self.graphics.borrow_mut();
            graphics.insert_compute_pipeline(pipeline_desc)
        };

        let dispatch = (_x, _y, _z);
        let queue = ComputePassQueue {
            pipeline,
            dispatch,
            bind_group,
            debug: None,
        };

        self.queues.push(queue);
    }

    fn end(&mut self) {
        let mut cpass = self.cmd.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        for queue in &self.queues {
            cpass.set_pipeline(&queue.pipeline);

            for (bind_group_index, bind_group) in &queue.bind_group {
                cpass.set_bind_group(*bind_group_index, bind_group, &[]);
            }

            if let Some(debug) = &queue.debug {
                cpass.insert_debug_marker(debug);
            }

            let (x, y, z) = queue.dispatch;
            cpass.dispatch_workgroups(x, y, z);
        }
    }
}

impl<'a> Drop for ComputePass<'a> {
    fn drop(&mut self) {
        self.end();
    }
}

pub(crate) struct ComputePassQueue {
    pub pipeline: wgpu::ComputePipeline,
    pub dispatch: (u32, u32, u32),
    pub bind_group: Vec<(u32, wgpu::BindGroup)>,

    pub debug: Option<String>,
}
