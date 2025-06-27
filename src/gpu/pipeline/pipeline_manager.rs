use std::{collections::HashMap, hash::Hash};

use crate::dbg_log;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PipelineManager {
    pub graphics_pipelines: HashMap<usize, (wgpu::RenderPipeline, usize)>,
    pub compute_pipelines: HashMap<usize, (wgpu::ComputePipeline, usize)>,
}

const PIPELINE_LIFETIME_FRAMES: usize = 50;

#[derive(Debug, Clone, Hash)]
pub(crate) struct VertexAttributeLayout {
    pub stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

#[derive(Debug, Clone, Hash)]
pub(crate) struct GraphicsPipelineDesc {
    pub shaders: (wgpu::ShaderModule, wgpu::ShaderModule),
    pub entry_point: (String, String),
    pub render_target: wgpu::TextureFormat,
    pub depth_stencil: Option<wgpu::TextureFormat>,
    pub blend_state: Option<wgpu::BlendState>,
    pub write_mask: Option<wgpu::ColorWrites>,
    pub vertex_desc: VertexAttributeLayout,
    pub primitive_state: wgpu::PrimitiveState,
    pub bind_group_layout: Vec<wgpu::BindGroupLayout>,
    pub msaa_count: u32,
}

#[derive(Debug, Clone, Hash)]
pub(crate) struct ComputePipelineDesc {
    pub shader_module: wgpu::ShaderModule,
    pub entry_point: String,
    pub bind_group_layout: Vec<wgpu::BindGroupLayout>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self {
            graphics_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
        }
    }

    pub fn get_graphics_pipeline(&mut self, key: usize) -> Option<wgpu::RenderPipeline> {
        if let Some((pipeline, lifetime)) = self.graphics_pipelines.get_mut(&key) {
            // reset lifetime
            *lifetime = 0;
            Some(pipeline.clone())
        } else {
            None
        }
    }

    pub fn create_graphics_pipeline(
        &mut self,
        key: usize,
        device: &wgpu::Device,
        cache: Option<&wgpu::PipelineCache>,
        desc: GraphicsPipelineDesc,
    ) -> wgpu::RenderPipeline {
        let bind_group_layout_refs = desc.bind_group_layout.iter().map(|l| l).collect::<Vec<_>>();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("PipelineLayout {}", key).as_str()),
            bind_group_layouts: bind_group_layout_refs.as_slice(),
            push_constant_ranges: &[],
        });

        let mut depth_stencil_desc = None;
        if let Some(format) = desc.depth_stencil {
            depth_stencil_desc = Some(wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            });
        }

        let color_target = Some(wgpu::ColorTargetState {
            format: desc.render_target,
            blend: desc.blend_state,
            write_mask: desc.write_mask.unwrap_or(wgpu::ColorWrites::empty()),
        });

        let binding = [color_target];

        let label = format!("RenderPipeline {}", key);

        let vertex_attribute_layout = wgpu::VertexBufferLayout {
            array_stride: desc.vertex_desc.stride,
            step_mode: desc.vertex_desc.step_mode,
            attributes: desc.vertex_desc.attributes.as_slice(),
        };

        let render_pipeline_desc = wgpu::RenderPipelineDescriptor {
            label: Some(label.as_str()),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &desc.shaders.0,
                entry_point: Some(desc.entry_point.0.as_str()),
                buffers: &[vertex_attribute_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &desc.shaders.1,
                entry_point: Some(desc.entry_point.1.as_str()),
                targets: &binding,
                compilation_options: Default::default(),
            }),
            primitive: desc.primitive_state,
            depth_stencil: depth_stencil_desc,
            multisample: wgpu::MultisampleState {
                count: desc.msaa_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache,
            multiview: None,
        };

        let pipeline = device.create_render_pipeline(&render_pipeline_desc);
        self.graphics_pipelines.insert(key, (pipeline.clone(), 0));

        dbg_log!("Inserted new graphics pipeline with key: {}", key);

        pipeline
    }

    pub fn get_compute_pipeline(&mut self, key: usize) -> Option<wgpu::ComputePipeline> {
        if let Some((pipeline, lifetime)) = self.compute_pipelines.get_mut(&key) {
            // reset lifetime
            *lifetime = 0;
            Some(pipeline.clone())
        } else {
            None
        }
    }

    pub fn create_compute_pipeline(
        &mut self,
        key: usize,
        device: &wgpu::Device,
        cache: Option<&wgpu::PipelineCache>,
        desc: ComputePipelineDesc,
    ) -> wgpu::ComputePipeline {
        let bind_group_layout_refs = desc.bind_group_layout.iter().map(|l| l).collect::<Vec<_>>();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("PipelineLayout {}", key).as_str()),
            bind_group_layouts: bind_group_layout_refs.as_slice(),
            push_constant_ranges: &[],
        });

        let label = format!("ComputePipeline {}", key);

        let compute_pipeline_desc = wgpu::ComputePipelineDescriptor {
            label: Some(label.as_str()),
            layout: Some(&pipeline_layout),
            module: &desc.shader_module,
            entry_point: Some(desc.entry_point.as_str()),
            cache,
            compilation_options: Default::default(),
        };

        let pipeline = device.create_compute_pipeline(&compute_pipeline_desc);
        self.compute_pipelines.insert(key, (pipeline.clone(), 0));

        pipeline
    }

    pub fn cycle(&mut self) {
        self.graphics_pipelines
            .retain(|_, value| value.1 < PIPELINE_LIFETIME_FRAMES);

        for (_, value) in self.graphics_pipelines.iter_mut() {
            value.1 += 1;
        }

        self.compute_pipelines
            .retain(|_, value| value.1 < PIPELINE_LIFETIME_FRAMES);

        for (_, value) in self.compute_pipelines.iter_mut() {
            value.1 += 1;
        }
    }
}
