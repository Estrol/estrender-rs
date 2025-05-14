use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

pub struct PipelineManager {
    pub graphics_pipelines: HashMap<usize, (wgpu::RenderPipeline, usize)>,
    pub compute_pipelines: HashMap<usize, (wgpu::ComputePipeline, usize)>,
}

const PIPELINE_MANAGER_CAPACITY: usize = 500;
const PIPELINE_LIFETIME_FRAMES: usize = 50;
const PIPELINE_LIFETIME_FRAMES_EMERGENCY: usize = 10;

pub struct GraphicsPipelineDesc<'a> {
    pub shader_module: &'a wgpu::ShaderModule,
    pub entry_point: (&'a str, &'a str),
    pub render_target: wgpu::TextureFormat,
    pub depth_stencil: Option<wgpu::TextureFormat>,
    pub blend_state: wgpu::BlendState,
    pub vertex_desc: wgpu::VertexBufferLayout<'a>,
    pub index_format: wgpu::IndexFormat,
    pub write_mask: wgpu::ColorWrites,
    pub primitive_state: wgpu::PrimitiveState,
    pub bind_group_layout: Vec<&'a wgpu::BindGroupLayout>,
    pub msaa_count: u32,
}

pub struct ComputePipelineDesc<'a> {
    pub shader_module: &'a wgpu::ShaderModule,
    pub entry_point: &'a str,
    pub bind_group_layout: Vec<&'a wgpu::BindGroupLayout>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self {
            graphics_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
        }
    }

    pub fn insert_graphics_pipeline(
        &mut self,
        device: &wgpu::Device,
        cache: Option<&wgpu::PipelineCache>,
        desc: GraphicsPipelineDesc,
    ) -> wgpu::RenderPipeline {
        let mut hasher = DefaultHasher::new();
        desc.shader_module.hash(&mut hasher);
        desc.entry_point.hash(&mut hasher);
        desc.render_target.hash(&mut hasher);
        desc.depth_stencil.hash(&mut hasher);
        desc.blend_state.hash(&mut hasher);
        desc.primitive_state.hash(&mut hasher);
        desc.vertex_desc.hash(&mut hasher);
        desc.index_format.hash(&mut hasher);
        desc.write_mask.hash(&mut hasher);
        desc.msaa_count.hash(&mut hasher);

        // Graphics pipeline, to avoid clashes with compute pipelines
        0u64.hash(&mut hasher);

        let key = hasher.finish() as usize;
        if !self.graphics_pipelines.contains_key(&key) {
            if self.graphics_pipelines.len() >= PIPELINE_MANAGER_CAPACITY {
                self.graphics_pipelines
                    .retain(|_, value| value.1 < PIPELINE_LIFETIME_FRAMES_EMERGENCY);

                if self.graphics_pipelines.len() >= PIPELINE_MANAGER_CAPACITY {
                    panic!("PipelineManager capacity exceeded");
                }
            }

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("PipelineLayout {}", key).as_str()),
                bind_group_layouts: &desc.bind_group_layout,
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
                blend: Some(desc.blend_state),
                write_mask: desc.write_mask,
            });

            let binding = [color_target];

            let render_pipeline_desc = wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: desc.shader_module,
                    entry_point: Some(desc.entry_point.0),
                    buffers: &[desc.vertex_desc],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: desc.shader_module,
                    entry_point: Some(desc.entry_point.1),
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
            self.graphics_pipelines.insert(key, (pipeline, 0));
        }

        let value = self.graphics_pipelines.get_mut(&key).unwrap();
        value.1 = 0;

        value.0.clone()
    }

    pub fn insert_compute_pipeline(
        &mut self,
        device: &wgpu::Device,
        cache: Option<&wgpu::PipelineCache>,
        desc: ComputePipelineDesc,
    ) -> wgpu::ComputePipeline {
        let mut hasher = DefaultHasher::new();
        desc.shader_module.hash(&mut hasher);
        desc.entry_point.hash(&mut hasher);
        desc.bind_group_layout.hash(&mut hasher);

        // Compute pipeline, to avoid clashes with graphics pipelines
        1u64.hash(&mut hasher);

        let key = hasher.finish() as usize;
        if !self.graphics_pipelines.contains_key(&key) {
            if self.graphics_pipelines.len() >= PIPELINE_MANAGER_CAPACITY {
                self.graphics_pipelines
                    .retain(|_, value| value.1 < PIPELINE_LIFETIME_FRAMES_EMERGENCY);

                if self.graphics_pipelines.len() >= PIPELINE_MANAGER_CAPACITY {
                    panic!("PipelineManager capacity exceeded");
                }
            }

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("PipelineLayout {}", key).as_str()),
                bind_group_layouts: desc.bind_group_layout.as_slice(),
                push_constant_ranges: &[],
            });

            let compute_pipeline_desc = wgpu::ComputePipelineDescriptor {
                label: Some("compute_pipeline"),
                layout: Some(&pipeline_layout),
                module: desc.shader_module,
                entry_point: Some(desc.entry_point),
                cache,
                compilation_options: Default::default(),
            };

            let pipeline = device.create_compute_pipeline(&compute_pipeline_desc);
            self.compute_pipelines.insert(key, (pipeline, 0));
        }

        let value = self.compute_pipelines.get_mut(&key).unwrap();
        value.1 = 0;

        value.0.clone()
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
