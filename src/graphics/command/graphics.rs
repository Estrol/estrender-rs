use std::collections::HashMap;

use wgpu::{CommandEncoder, ShaderStages};

use crate::{
    graphics::{
        buffer::Buffer,
        inner::GPUInner,
        pipeline_manager::GraphicsPipelineDesc,
        shader::{
            GraphicsShader, GraphicsShaderBuilder, GraphicsShaderInner, IndexBufferSize,
            ShaderBindingInfo, ShaderBindingType, ShaderCullMode, ShaderFrontFace,
            ShaderPollygonMode, ShaderTopology,
        },
        texture::{
            SampleCount, Texture, TextureBlend, TextureBuilder, TextureFormat, TextureSampler,
            TextureUsage,
        },
    },
    math::{self, Color, Rect, RectF, Vector2, Vector3, Vertex},
    utils::ArcRef,
};

pub struct ShaderBinding {
    pub shader: ArcRef<GraphicsShaderInner>,
    pub bindings: Vec<ShaderBindingInfo>,

    // Optional shader overrides
    pub topology: Option<ShaderTopology>,
    pub cull_mode: Option<Option<ShaderCullMode>>,
    pub front_face: Option<ShaderFrontFace>,
    pub polygon_mode: Option<ShaderPollygonMode>,
    pub index_format: Option<IndexBufferSize>,
}

#[allow(dead_code)]
pub enum BindGroupType {
    Uniform(wgpu::Buffer),
    Texture(wgpu::TextureView, TextureBlend),
    Sampler(wgpu::Sampler),
    Storage(wgpu::Buffer),
}

#[allow(dead_code)]
pub enum Attachment<'a> {
    Texture(&'a Texture, &'a TextureBlend),
    Sampler(&'a TextureSampler),
    Buffer(&'a Buffer),
    Storage(&'a Buffer),
}

pub trait IntoOptionAttachment<'a> {
    fn into_option_attachment(self) -> Option<Attachment<'a>>;
}

#[allow(dead_code)]
pub struct BindGroupAttachment {
    group: u32,
    binding: u32,
    attachment: BindGroupType,
}

pub struct TextureInput<'a> {
    pub texture: Option<&'a Texture>,
    pub binding_texture: usize,
    pub binding_sampler: usize,
}

pub struct RenderPassQueue {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: Vec<(u32, wgpu::BindGroup)>,

    pub vbo: Option<wgpu::Buffer>,
    pub ibo: Option<wgpu::Buffer>,
    pub itype: Option<wgpu::IndexFormat>,

    pub viewport: Option<wgpu::Extent3d>,
    pub scissor: Option<RectF>,

    pub start: u32,
    pub count: u32,
    pub vertex_offset: u32,

    pub push_constant: Option<Vec<u8>>,
}

pub struct GraphicsPass<'a> {
    pub(crate) graphics: &'a ArcRef<GPUInner>,
    pub(crate) cmd: &'a mut CommandEncoder,
    pub(crate) render_target: wgpu::TextureView,
    pub(crate) render_target_format: wgpu::TextureFormat,
    pub(crate) depth_target: Option<wgpu::TextureView>,
    pub(crate) depth_target_format: Option<wgpu::TextureFormat>,

    pub(crate) multi_sample_target: Option<wgpu::TextureView>,
    pub(crate) multi_sample_count: Option<u32>,

    pub(crate) clear_color: Option<Color>,
    pub(crate) viewport: Option<Rect>,
    pub(crate) scissor: Option<RectF>,

    pub(crate) vertex: Option<wgpu::Buffer>,
    pub(crate) index: Option<wgpu::Buffer>,

    pub(crate) shader: Option<ShaderBinding>,
    pub(crate) attachments: Vec<BindGroupAttachment>,
    pub(crate) push_constant: Option<Vec<u8>>,

    pub(crate) queues: Vec<RenderPassQueue>,
}

impl<'a> GraphicsPass<'a> {
    pub fn new(
        graphics: &'a ArcRef<GPUInner>,
        cmd: &'a mut CommandEncoder,
        render_target: wgpu::TextureView,
        render_target_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            graphics,
            cmd,
            render_target,
            render_target_format,
            depth_target: None,
            depth_target_format: None,

            multi_sample_count: None,
            multi_sample_target: None,

            clear_color: None,
            viewport: None,
            scissor: None,

            vertex: None,
            index: None,

            shader: None,
            attachments: Vec::new(),
            push_constant: None,

            queues: Vec::new(),
        }
    }

    pub fn clear_color(&mut self, _color: Color) {
        self.clear_color = Some(_color);
    }

    pub fn set_gpu_buffer(&mut self, vertex: Option<&Buffer>, index: Option<&Buffer>) {
        self.vertex = None;
        self.index = None;

        if let Some(vertex) = vertex {
            self.vertex = Some(vertex.inner.wait_borrow().buffer.clone());
        }

        if let Some(index) = index {
            self.index = Some(index.inner.wait_borrow().buffer.clone());
        }
    }

    pub fn set_gpu_buffer_raw<T, T2>(&mut self, vertex: Option<&[T]>, index: Option<&[T2]>)
    where
        T: bytemuck::Pod,
        T2: bytemuck::Pod,
    {
        let mut inner = self.graphics.borrow_mut();

        self.vertex = None;
        self.index = None;

        if let Some(vertex) = vertex {
            let vertex = bytemuck::cast_slice(vertex);

            let buffer = inner.insert_buffer(vertex, wgpu::BufferUsages::VERTEX);
            self.vertex = Some(buffer);
        }

        if let Some(index) = index {
            let index = bytemuck::cast_slice(index);

            let buffer = inner.insert_buffer(index, wgpu::BufferUsages::INDEX);
            self.index = Some(buffer);
        }
    }

    pub fn set_texture(&mut self, group: usize, texture: TextureInput<'_>) {
        if self.shader.is_none() {
            panic!("Shader is not set");
        }

        self.set_texture_ex(group, texture, None, None);
    }

    pub fn set_texture_ex(
        &mut self,
        group: usize,
        texture: TextureInput<'_>,
        blend: Option<TextureBlend>,
        sampler: Option<wgpu::Sampler>,
    ) {
        if self.shader.is_none() {
            panic!("Shader is not set");
        }

        if texture.texture.is_none() {
            self.attachments
                .retain(|a| a.group != group as u32 && a.binding != texture.binding_texture as u32);

            self.attachments
                .retain(|a| a.group != group as u32 && a.binding != texture.binding_sampler as u32);

            return;
        }

        let texture_binding_id = texture.binding_texture as u32;
        let sampler_binding_id = texture.binding_sampler as u32;

        let shader = self.shader.as_ref().unwrap().shader.borrow();
        shader
            .reflection
            .bindings
            .iter()
            .find(|b| b.group == group as u32 && b.binding == texture_binding_id)
            .expect(
                format!(
                    "Texture group: {} binding: {} not found",
                    group, texture_binding_id
                )
                .as_str(),
            );

        shader
            .reflection
            .bindings
            .iter()
            .find(|b| b.group == group as u32 && b.binding == sampler_binding_id)
            .expect(
                format!(
                    "Sample group: {} binding: {} not found",
                    group, sampler_binding_id
                )
                .as_str(),
            );

        // remove old texture and sampler
        self.attachments.retain(|a| {
            if a.group == group as u32
                && (a.binding == texture_binding_id || a.binding == sampler_binding_id)
            {
                false
            } else {
                true
            }
        });

        let texture = texture.texture.unwrap();
        let inner = texture.inner.borrow();

        let attachments = [
            BindGroupAttachment {
                group: group as u32,
                binding: texture_binding_id,
                attachment: BindGroupType::Texture(
                    inner.wgpu_view.clone(),
                    blend.unwrap_or(inner.blend.clone()),
                ),
            },
            BindGroupAttachment {
                group: group as u32,
                binding: sampler_binding_id,
                attachment: BindGroupType::Sampler(sampler.unwrap_or(inner.wgpu_sampler.clone())),
            },
        ];

        self.attachments.extend(attachments.into_iter());
    }

    pub fn set_shader(&mut self, _shader: Option<&GraphicsShader>) {
        self.set_shader_ex(_shader, None, None, None, None, None);
    }

    pub fn set_shader_ex(
        &mut self,
        _shader: Option<&GraphicsShader>,
        topology: Option<ShaderTopology>,
        cull_mode: Option<Option<ShaderCullMode>>,
        front_face: Option<ShaderFrontFace>,
        polygon_mode: Option<ShaderPollygonMode>,
        index_format: Option<IndexBufferSize>,
    ) {
        if _shader.is_none() {
            self.shader = None;
            return;
        }

        let shader_binding = ShaderBinding {
            shader: ArcRef::clone(&_shader.unwrap().inner),
            bindings: Vec::new(),

            topology,
            cull_mode,
            front_face,
            polygon_mode,
            index_format,
        };

        self.shader = Some(shader_binding);
    }

    pub fn set_attachment<T>(&mut self, _group: u32, _binding: u32, _attachment: T)
    where
        T: Into<Option<Attachment<'a>>>,
    {
        let _attachment = _attachment.into();

        if _attachment.is_none() {
            self.attachments
                .retain(|a| a.group != _group || a.binding != _binding);
            return;
        }

        let attachment = _attachment.unwrap();

        let attachment = match attachment {
            Attachment::Texture(texture, blend) => BindGroupAttachment {
                group: _group,
                binding: _binding,
                attachment: BindGroupType::Texture(
                    texture.inner.borrow().wgpu_view.clone(),
                    blend.clone(),
                ),
            },
            Attachment::Sampler(sampler) => {
                let sampler = sampler.make_wgpu(&self.graphics.borrow().device);

                BindGroupAttachment {
                    group: _group,
                    binding: _binding,
                    attachment: BindGroupType::Sampler(sampler),
                }
            }
            Attachment::Buffer(buffer) => BindGroupAttachment {
                group: _group,
                binding: _binding,
                attachment: BindGroupType::Uniform(buffer.inner.borrow().buffer.clone()),
            },
            Attachment::Storage(buffer) => BindGroupAttachment {
                group: _group,
                binding: _binding,
                attachment: BindGroupType::Storage(buffer.inner.borrow().buffer.clone()),
            },

            #[allow(unreachable_patterns)]
            _ => todo!("Attachment not implemented"),
        };

        // replace or insert attachment
        let index = self
            .attachments
            .iter()
            .position(|a| a.group == _group && a.binding == _binding);
        if let Some(index) = index {
            self.attachments[index] = attachment;
        } else {
            self.attachments.push(attachment);
        }
    }

    pub fn set_viewport(&mut self, _viewport: Option<Rect>) {
        self.viewport = _viewport;
    }

    pub fn set_scissor(&mut self, _scissor: Option<RectF>) {
        self.scissor = _scissor;
    }

    pub fn set_multi_sample_texture(&mut self, _texture: Option<&'a Texture>) {
        if _texture.is_none() {
            self.multi_sample_target = None;
            self.multi_sample_count = None;

            return;
        }

        let texture = _texture.unwrap();
        let inner = texture.inner.borrow();

        if inner.sample_count == SampleCount::SampleCount1 {
            panic!("Texture must have multi sample count");
        }

        self.multi_sample_target = Some(inner.wgpu_view.clone());
        self.multi_sample_count = Some(inner.sample_count.into());
    }

    pub fn set_depth_texture(&mut self, _texture: Option<&'a Texture>) {
        if _texture.is_none() {
            self.depth_target = None;
            self.depth_target_format = None;

            return;
        }

        let texture = _texture.unwrap();
        let inner = texture.inner.borrow();

        if !inner.usages.contains(TextureUsage::RenderAttachment) {
            panic!("Texture must have render attachment usage");
        }

        let expected_depth_format = [
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureFormat::Depth24Plus,
            wgpu::TextureFormat::Depth24PlusStencil8,
        ];

        let format = inner.format.into();

        if !expected_depth_format.contains(&format) {
            panic!("Texture must have depth format");
        }

        self.depth_target = Some(inner.wgpu_view.clone());
        self.depth_target_format = Some(format);
    }

    pub fn set_push_constants(&mut self, _data: &[u8]) {
        if self.shader.is_none() {
            panic!("Shader is not set");
        }

        let shader = self.shader.as_ref().unwrap();
        let size = shader.bindings.iter().fold(0, |acc, b| {
            if let ShaderBindingType::PushConstant(size) = b.ty {
                acc + size
            } else {
                acc
            }
        });

        if _data.len() > size as usize {
            panic!("Data size must be less or equal to the push constant size");
        }

        let data = _data.to_vec();
        self.push_constant = Some(data);
    }

    pub fn draw(&mut self, start_vertex: u32, count: u32) {
        self.prepare_draw(false, start_vertex, count, u32::MAX);
    }

    pub fn draw_indexed(&mut self, start_index: u32, count: u32, vertex_start: u32) {
        self.prepare_draw(true, start_index, count, vertex_start);
    }

    fn prepare_draw(&mut self, use_index_buffer: bool, start: u32, count: u32, vertex_offset: u32) {
        if self.vertex.is_none() {
            panic!("Vertex buffer is not set");
        }

        if use_index_buffer && self.index.is_none() {
            panic!("Index buffer is not set");
        }

        let shader_binding = self.shader.as_ref().unwrap();
        let shader = shader_binding.shader.borrow();
        let mut texture_blend = TextureBlend::NONE;

        let attributes = shader.vertex_input_desc.make_attributes();
        let vertex_desc = shader.vertex_input_desc.make(&attributes);

        let primitive_state = wgpu::PrimitiveState {
            topology: shader_binding.topology.unwrap_or(shader.topology).into(),
            strip_index_format: None,
            front_face: shader_binding
                .front_face
                .unwrap_or(shader.front_face)
                .into(),
            cull_mode: shader_binding
                .cull_mode
                .unwrap_or(shader.cull_mode)
                .map_or(None, |c| c.into()),
            polygon_mode: shader_binding
                .polygon_mode
                .unwrap_or(shader.polygon_mode)
                .into(),
            unclipped_depth: false,
            conservative: false,
        };

        let bind_group_attachments: HashMap<u32, Vec<wgpu::BindGroupEntry>> =
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
                    BindGroupType::Texture(texture, blend) => {
                        texture_blend = blend.clone();
                        wgpu::BindGroupEntry {
                            binding,
                            resource: wgpu::BindingResource::TextureView(texture),
                        }
                    }
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
                    .find(|l| l.group == *group)
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

        let entry_point = (
            shader.reflection.vertex_entry_point.as_str(),
            shader.reflection.fragment_entry_point.as_str(),
        );

        let index_format = shader_binding
            .index_format
            .unwrap_or(shader.index_buffer_size);

        let pipeline_desc = GraphicsPipelineDesc {
            shader_module: &shader.shader,
            entry_point,
            render_target: self.render_target_format,
            depth_stencil: self.depth_target_format,
            blend_state: texture_blend.clone().into(),
            write_mask: texture_blend.clone().into(),
            index_format: index_format.into(),
            vertex_desc,
            primitive_state,
            bind_group_layout,
            msaa_count: self.multi_sample_count.unwrap_or(1),
        };

        let pipeline = {
            let mut inner = self.graphics.borrow_mut();
            inner.insert_graphics_pipeline(pipeline_desc)
        };

        let mut wgpu_viewport = None;
        if let Some(viewport) = self.viewport.as_ref() {
            wgpu_viewport = Some(wgpu::Extent3d {
                width: viewport.w as u32,
                height: viewport.h as u32,
                depth_or_array_layers: 1,
            });
        }

        let queue = RenderPassQueue {
            pipeline,
            bind_group,
            vbo: self.vertex.clone(),
            ibo: if use_index_buffer {
                self.index.clone()
            } else {
                None
            },
            itype: if use_index_buffer {
                Some(index_format.into())
            } else {
                None
            },
            viewport: wgpu_viewport,
            scissor: self.scissor.clone(),
            start,
            count,
            vertex_offset,
            push_constant: self.push_constant.clone(),
        };

        self.queues.push(queue);
    }

    pub fn run_drawing(&mut self, callback: impl FnOnce(&mut DrawingContext)) {
        let mut begin = self.begin_drawing().unwrap();

        callback(&mut begin);
    }

    pub fn begin_drawing(&mut self) -> Option<DrawingContext<'_, 'a>> {
        DrawingContext::new(self)
    }

    pub(crate) fn end(&mut self) {
        let mut clear_color = self.clear_color.unwrap_or(Color::BLACK);
        math::utils::rgb_to_srgb(&mut clear_color);

        let mut color_attachment = wgpu::RenderPassColorAttachment {
            view: &self.render_target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear_color.r as f64,
                    g: clear_color.b as f64,
                    b: clear_color.g as f64,
                    a: clear_color.a as f64,
                }),
                store: wgpu::StoreOp::Store,
            },
        };

        if let Some(msaa_resolve_target) = self.multi_sample_target.as_ref() {
            color_attachment.resolve_target = Some(color_attachment.view);
            color_attachment.view = msaa_resolve_target;
        }

        let mut depth_stencil_attachment = None;
        if let Some(depth_target) = self.depth_target.as_ref() {
            depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_target,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            });
        }

        let mut render_pass = self.cmd.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment,
            ..Default::default()
        });

        for queue in &self.queues {
            render_pass.set_pipeline(&queue.pipeline);

            for (group, bind) in &queue.bind_group {
                render_pass.set_bind_group(*group, bind, &[]);
            }

            if let Some(vbo) = &queue.vbo {
                render_pass.set_vertex_buffer(0, vbo.slice(..));
            }

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(pc) = &queue.push_constant {
                render_pass.set_push_constants(ShaderStages::all(), 0, pc);
            }

            if let Some(ibo) = &queue.ibo {
                render_pass.set_index_buffer(ibo.slice(..), queue.itype.unwrap());
                render_pass.draw_indexed(
                    queue.start..(queue.start + queue.count),
                    queue.vertex_offset as i32,
                    0..1,
                );
            } else {
                render_pass.draw(queue.start..(queue.start + queue.count), 0..1);
            }
        }
    }
}

impl<'a> Drop for GraphicsPass<'a> {
    fn drop(&mut self) {
        self.end();
    }
}

pub enum BufferInput<'a, T> {
    Buffer(&'a Buffer),
    Raw(&'a [T]),
}

pub trait IntoBufferInput<'a, T> {
    fn into_buffer_input(self) -> BufferInput<'a, T>;
}

impl<'a> IntoBufferInput<'a, u8> for &'a Buffer {
    fn into_buffer_input(self) -> BufferInput<'a, u8> {
        BufferInput::Buffer(self)
    }
}

impl<'a, T> IntoBufferInput<'a, T> for &'a [T] {
    fn into_buffer_input(self) -> BufferInput<'a, T> {
        BufferInput::Raw(self)
    }
}

pub struct DrawingContext<'a, 'gp: 'a> {
    pub(crate) pass: &'a mut GraphicsPass<'gp>,

    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u16>,

    pub(crate) texture: Option<Texture>,
    pub(crate) scissor: Option<RectF>,
    pub(crate) viewport: Option<Rect>,

    pub(crate) current_queue: Option<DrawingQueue>,
    pub(crate) queue: Vec<DrawingQueue>,
}

pub struct DrawingQueue {
    pub texture: Option<Texture>,
    pub shader: Option<GraphicsShader>,

    pub scissors: Option<RectF>,
    pub viewport: Option<Rect>,

    pub start_index: u32,
    pub start_vertex: u32,
    pub count: u32,
}

const DEFAULT_DRAING_SHADER: &str = r#"
// Vertex Shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) texCoord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) texCoord: vec2<f32>,
};

@vertex
fn main_vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.texCoord = input.texCoord;
    return output;
}

// Fragment Shader
@group(0) @binding(0) var myTexture: texture_2d<f32>;
@group(0) @binding(1) var mySampler: sampler;

struct FragmentInput {
    @location(0) color: vec4<f32>,
    @location(1) texCoord: vec2<f32>,
};

@fragment
fn main_fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    let textureColor = textureSample(myTexture, mySampler, input.texCoord);
    return input.color * textureColor;
}"#;

impl<'a, 'gp: 'a> DrawingContext<'a, 'gp> {
    pub fn new(pass: &'a mut GraphicsPass<'gp>) -> Option<Self> {
        {
            if pass.graphics.borrow().drawing_default_shader.is_none() {
                let shader = GraphicsShaderBuilder::new(ArcRef::clone(pass.graphics))
                    .with_source(DEFAULT_DRAING_SHADER)
                    .build();

                if shader.is_err() {
                    return None;
                }

                pass.graphics.borrow_mut().drawing_default_shader = Some(shader.unwrap());
            }

            if pass.graphics.borrow().drawing_default_texture.is_none() {
                let data = vec![255u8, 255, 255, 255];
                let texture = TextureBuilder::new(ArcRef::clone(pass.graphics))
                    .with_raw(&data, Rect::with_size(1, 1), TextureFormat::Bgra8Unorm)
                    .with_usage(TextureUsage::Sampler)
                    .build();

                if texture.is_err() {
                    return None;
                }

                pass.graphics.borrow_mut().drawing_default_texture = Some(texture.unwrap());
            }
        }

        Some(DrawingContext {
            pass,
            vertices: Vec::new(),
            indices: Vec::new(),

            texture: None,
            scissor: None,
            viewport: None,

            current_queue: None,
            queue: Vec::new(),
        })
    }

    pub fn rectangle(&mut self, rect: Rect, color: Color) {
        let vertices = [
            Vertex::new(
                Vector3::new(rect.x as f32, rect.y as f32, 0.0),
                color,
                Vector2::new(0.0, 0.0),
            ),
            Vertex::new(
                Vector3::new((rect.x + rect.w) as f32, rect.y as f32, 0.0),
                color,
                Vector2::new(1.0, 0.0),
            ),
            Vertex::new(
                Vector3::new((rect.x + rect.w) as f32, (rect.y + rect.h) as f32, 0.0),
                color,
                Vector2::new(1.0, 1.0),
            ),
            Vertex::new(
                Vector3::new(rect.x as f32, (rect.y + rect.h) as f32, 0.0),
                color,
                Vector2::new(0.0, 1.0),
            ),
        ];

        let base_index = self.vertices.len() as u16;
        let indices = [
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ];

        self.push_queue(indices.len() as u32);

        self.vertices.extend_from_slice(&vertices);
        self.indices.extend_from_slice(&indices);
    }

    pub fn set_scissor(&mut self, scissor: RectF) {
        self.scissor = Some(scissor);
    }

    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = Some(viewport);
    }

    pub fn set_texture(&mut self, texture: Option<&Texture>) {
        self.texture = texture.cloned();
    }

    pub(crate) fn push_queue(&mut self, index_count: u32) {
        let mut push_new_queue = false;

        if self.current_queue.is_some() {
            let ref_queue = self.current_queue.as_ref().unwrap();

            // Check if current queue has the same texture, if not push the queue
            if ref_queue.texture != self.texture {
                push_new_queue = true;
            }

            // Check if current queue has the same scissor, if not push the queue
            if ref_queue.scissors != self.scissor {
                push_new_queue = true;
            }

            // Check if current queue has the same viewport, if not push the queue
            if ref_queue.viewport != self.viewport {
                push_new_queue = true;
            }
        } else {
            push_new_queue = true;
        }

        // Figure a way to push queue with correct start, and count
        if push_new_queue {
            if let Some(queue) = self.current_queue.take() {
                self.queue.push(queue);
            }

            self.current_queue = Some(DrawingQueue {
                texture: self.texture.clone(),
                shader: None,
                scissors: self.scissor.clone(),
                viewport: self.viewport.clone(),
                start_index: self.indices.len() as u32,
                start_vertex: 0, // TODO: Fix this
                count: index_count,
            });
        } else {
            let queue = self.current_queue.as_mut().unwrap();
            queue.count += index_count;
        }
    }

    pub(crate) fn end(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        if let Some(queue) = self.current_queue.take() {
            self.queue.push(queue);
        }

        let inner = self.pass.graphics.borrow();
        let inner_config = inner.config.as_ref().unwrap();
        let swapchain_size = Vector2::new(inner_config.width as f32, inner_config.height as f32);

        for vertex in &mut self.vertices {
            vertex.position.x = vertex.position.x / swapchain_size.x * 2.0 - 1.0;
            vertex.position.y = 1.0 - (vertex.position.y / swapchain_size.y * 2.0);
        }

        for queue in self.queue.iter_mut() {
            if queue.texture.is_none() {
                queue.texture = Some(inner.drawing_default_texture.as_ref().unwrap().clone());
            }

            if queue.shader.is_none() {
                queue.shader = Some(inner.drawing_default_shader.as_ref().unwrap().clone());
            }
        }

        drop(inner);

        self.pass
            .set_gpu_buffer_raw(Some(&self.vertices), Some(&self.indices));

        for queue in self.queue.iter() {
            self.pass.set_shader(queue.shader.as_ref());
            self.pass.set_scissor(queue.scissors);
            self.pass.set_viewport(queue.viewport);

            self.pass.set_texture(
                0,
                TextureInput {
                    texture: queue.texture.as_ref(),
                    binding_texture: 0,
                    binding_sampler: 1,
                },
            );

            self.pass
                .draw_indexed(queue.start_index, queue.count, queue.start_vertex);
        }

        self.vertices.clear();
        self.indices.clear();
        self.queue.clear();
    }
}

impl<'a, 'gp> Drop for DrawingContext<'a, 'gp> {
    fn drop(&mut self) {
        self.end();
    }
}
