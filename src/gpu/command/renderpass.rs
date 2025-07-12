use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}, ops::Range, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use crate::{
    math::{Color, Point2, RectF},
    utils::ArcRef,
};

use super::{
    utils::BindGroupType,
    drawing::DrawingContext,
    super::{
        GPUInner,
        texture::{
            Texture, 
            BlendState, 
            TextureSampler, 
            TextureUsage,
            TextureFormat, 
            SampleCount
        },
        buffer::{Buffer, BufferUsage},
        pipeline::{
            render::RenderPipeline,
            manager::{VertexAttributeLayout, GraphicsPipelineDesc},
        },
        shader::{
            graphics::{GraphicsShader, GraphicsShaderType},
            bind_group_manager::BindGroupCreateInfo,
            types::ShaderReflect,
            BindGroupLayout,
            ShaderTopology,
            ShaderCullMode,
            ShaderFrontFace,
            ShaderPollygonMode,
            IndexBufferSize,
            ShaderBindingType,
        },
        command::{BindGroupAttachment, SurfaceTexture},
    }
};


/// Represents a render pass in the graphics pipeline.
///
/// Renderpass support intermediate mode which includes setting up shaders, buffers, and attachments.
/// or using a pre-defined render pipeline.
///
/// It's generally recommended to use a render pipeline for better performance and validation upfront.
/// But for more dynamic scenarios, you can use the intermediate mode to set up shaders and buffers on the fly.
///
/// # Example Usage
/// Intermediate mode
/// ```rust
/// let mut render_pass = ...
/// render_pass.set_shader(Some(&my_shader));
/// render_pass.set_blend(Some(&my_blend));
/// render_pass.set_attachment_texture(0, 0, &my_texture);
/// render_pass.set_attachment_sampler(0, 1, &my_sampler);
/// render_pass.draw(0..3, 1);
/// ```
/// Render pipeline mode
/// ```rust
/// let pipeline = gpu.create_render_pipeline()
///    .set_shader(Some(&my_shader))
///    .set_blend(Some(&my_blend))
///    .set_attachment_texture(0, 0, &my_texture)
///    .set_attachment_sampler(0, 1, &my_sampler)
///    .build()
///    .expect("Failed to create render pipeline");
///
/// // Somewhere in your code
/// let mut render_pass = ...
/// render_pass.set_pipeline(Some(&pipeline));
/// render_pass.draw(0..3, 1);
/// ```
#[derive(Debug, Clone)]
pub struct RenderPass {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<RenderPassInner>,
}

impl RenderPass {
    pub(crate) fn new(
        graphics: ArcRef<GPUInner>,
        cmd: ArcRef<wgpu::CommandEncoder>,
        atomic_pass: Arc<AtomicBool>,
    ) -> Self {
        let inner = RenderPassInner {
            cmd,
            atomic_pass,

            render_targets: Vec::new(),
            depth_target: None,
            depth_target_format: None,
            surface_size: Point2::new(0.0, 0.0),

            multi_sample_count: None,
            multi_sample_target: Vec::new(),

            clear_color: None,
            viewport: None,
            scissor: None,

            vertex: None,
            index: None,

            shader: None,
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            shader_reflection: None,

            attachments: Vec::new(),
            push_constant: None,

            queues: Vec::new(),
        };

        Self {
            graphics,
            inner: ArcRef::new(inner),
        }
    }

    #[inline]
    pub fn set_clear_color(&mut self, _color: Color) {
        let mut inner = self.inner.borrow_mut();
        inner.clear_color = Some(_color);
    }

    #[inline]
    pub fn get_clear_color(&self) -> Option<Color> {
        let inner = self.inner.borrow();
        inner.clear_color.clone()
    }

    #[inline]
    pub fn set_blend(&mut self, index: usize, blend: Option<&BlendState>) {
        let mut inner = self.inner.borrow_mut();

        match inner.render_targets.get_mut(index) {
            Some(target) => {
                if let Some(blend) = blend {
                    target.blend = Some(blend.create_wgpu_blend_state());
                    target.write_mask = Some(blend.create_wgpu_color_write_mask());
                } else {
                    target.blend = None;
                    target.write_mask = Some(wgpu::ColorWrites::COLOR);
                }
            }
            None => {
                panic!("Render target at index {} does not exist", index);
            }
        }
    }

    #[inline]
    pub fn get_blend(&self, index: usize) -> Option<BlendState> {
        let inner = self.inner.borrow();

        match inner.render_targets.get(index) {
            Some(target) => {
                let state = target.blend.clone();
                let color_write_mask = target.write_mask.clone();

                Some(BlendState::from_wgpu(state, color_write_mask))
            }
            None => None,
        }
    }

    #[inline]
    pub fn set_gpu_buffer(&mut self, vertex: Option<&Buffer>, index: Option<&Buffer>) {
        self.set_gpu_buffer_wgpu(
            vertex.map(|v| v.inner.borrow().buffer.clone()),
            index.map(|i| i.inner.borrow().buffer.clone()),
        );
    }

    #[inline]
    pub fn set_gpu_buffer_raw<T, T2>(&mut self, vertex: Option<&[T]>, index: Option<&[T2]>)
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
        T2: bytemuck::Pod + bytemuck::Zeroable,
    {
        let (vertex_buffer, index_buffer) = {
            let mut gpu_inner = self.graphics.borrow_mut();

            let vertex_buffer = match vertex {
                Some(data) => {
                    let buffer = gpu_inner.create_buffer_with(data, wgpu::BufferUsages::VERTEX);
                    Some(buffer)
                }
                None => None,
            };

            let index_buffer = match index {
                Some(data) => {
                    let buffer = gpu_inner.create_buffer_with(data, wgpu::BufferUsages::INDEX);
                    Some(buffer)
                }
                None => None,
            };

            (vertex_buffer, index_buffer)
        };

        self.set_gpu_buffer_wgpu(vertex_buffer, index_buffer);
    }

    pub(crate) fn set_gpu_buffer_wgpu(
        &mut self,
        vertex: Option<wgpu::Buffer>,
        index: Option<wgpu::Buffer>,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();
            if inner.shader.is_none() {
                panic!("Shader is not set");
            }

            let shader = inner.shader.as_ref().unwrap();

            let index_format = match shader {
                RenderShaderBinding::Intermediate(IntermediateRenderPipeline {
                    index_format,
                    ..
                }) => index_format,
                RenderShaderBinding::Pipeline(RenderPipeline { index_format, .. }) => index_format,
            };

            if index_format.is_none() && index.is_some() {
                panic!("Index buffer is set, but shader not configured to use index buffer");
            }
        }

        let mut inner = self.inner.borrow_mut();

        inner.vertex = vertex;
        inner.index = index;
    }

    #[inline]
    pub fn get_gpu_buffer(&self) -> (Option<wgpu::Buffer>, Option<wgpu::Buffer>) {
        let inner = self.inner.borrow();
        (inner.vertex.clone(), inner.index.clone())
    }

    #[inline]
    pub fn set_shader(&mut self, shader: Option<&GraphicsShader>) {
        self.set_shader_ex(shader, None, None, None, None, None);
    }

    #[inline]
    pub fn set_shader_ex(
        &mut self,
        shader: Option<&GraphicsShader>,
        topology: Option<ShaderTopology>,
        cull_mode: Option<ShaderCullMode>,
        front_face: Option<ShaderFrontFace>,
        polygon_mode: Option<ShaderPollygonMode>,
        index_format: Option<IndexBufferSize>,
    ) {
        let mut inner = self.inner.borrow_mut();

        match shader {
            Some(shader) => {
                let shader_inner = shader.inner.borrow();
                let (vertex_shader, fragment_shader) = match &shader_inner.ty {
                    GraphicsShaderType::GraphicsSplit {
                        vertex_module,
                        fragment_module,
                    } => (vertex_module.clone(), fragment_module.clone()),
                    GraphicsShaderType::GraphicsSingle { module } => (module.clone(), module.clone()),
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
                let shader_binding = IntermediateRenderPipeline {
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

                inner.shader = Some(RenderShaderBinding::Intermediate(shader_binding));

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    inner.shader_reflection = Some(shader_inner.reflection.clone());
                }
            }
            None => {
                inner.shader = None;

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    inner.shader_reflection = None;
                }
            }
        }
    }

    pub fn set_pipeline(&mut self, pipeline: Option<&RenderPipeline>) {
        let mut inner = self.inner.borrow_mut();

        match pipeline {
            Some(pipeline) => {
                inner.shader = Some(RenderShaderBinding::Pipeline(pipeline.clone()));
            }
            None => {
                inner.shader = None;
            }
        }
    }

    #[inline]
    pub(crate) fn remove_attachment(&mut self, group: u32, binding: u32) {
        let mut inner = self.inner.borrow_mut();

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            match &inner.shader {
                Some(RenderShaderBinding::Pipeline(_)) => {
                    panic!("Cannot insert or replace attachment when using a pipeline shader");
                }
                _ => {}
            }
        }

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

            match &inner.shader {
                Some(RenderShaderBinding::Pipeline(_)) => {
                    panic!("Cannot insert or replace attachment when using a pipeline shader");
                }
                _ => {}
            }

            let r#type = inner
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

    #[inline]
    pub fn set_viewport(&mut self, _viewport: Option<RectF>, min_depth: f32, max_depth: f32) {
        let mut inner = self.inner.borrow_mut();

        match _viewport {
            Some(viewport) => {
                inner.viewport = Some((viewport, min_depth, max_depth));
            }
            None => {
                inner.viewport = None;
            }
        }
    }

    #[inline]
    pub fn get_viewport(&self) -> Option<(RectF, f32, f32)> {
        let inner = self.inner.borrow();
        inner.viewport.clone()
    }

    #[inline]
    pub fn set_scissor(&mut self, _scissor: Option<RectF>) {
        let mut inner = self.inner.borrow_mut();

        match _scissor {
            Some(scissor) => {
                inner.scissor = Some(scissor);
            }
            None => {
                inner.scissor = None;
            }
        }
    }

    #[inline]
    pub fn get_scissor(&self) -> Option<RectF> {
        let inner = self.inner.borrow();
        inner.scissor.clone()
    }

    #[inline]
    pub fn push_msaa_texture(&mut self, texture: &Texture) {
        let mut inner = self.inner.borrow_mut();

        if inner.multi_sample_count.is_none() {
            inner.multi_sample_count = Some(texture.inner.borrow().sample_count.into());
        }

        // check msaa count
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let msaa_count = texture.inner.borrow().sample_count.into();
            if inner.multi_sample_count.unwrap() != msaa_count {
                panic!("Multi sample texture count must match render target count");
            }
        }

        inner
            .multi_sample_target
            .push(texture.inner.borrow().wgpu_view.clone());
    }

    #[inline]
    pub fn set_depth_texture(&mut self, texture: Option<&Texture>) {
        let mut inner = self.inner.borrow_mut();

        match texture {
            Some(texture) => {
                let texture_inner = texture.inner.borrow();
                let format = texture_inner.format.into();

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    if !texture_inner
                        .usages
                        .contains(TextureUsage::RenderAttachment)
                    {
                        panic!("Texture must have render attachment usage");
                    }

                    let expected_depth_format = [
                        wgpu::TextureFormat::Depth32Float,
                        wgpu::TextureFormat::Depth24Plus,
                        wgpu::TextureFormat::Depth24PlusStencil8,
                    ];

                    if !expected_depth_format.contains(&format) {
                        panic!("Texture must have depth format");
                    }

                    if texture_inner.sample_count != SampleCount::SampleCount1 {
                        panic!("Depth texture must be single sampled");
                    }

                    let depth_size = texture_inner.size;
                    if depth_size.x == 0 || depth_size.y == 0 {
                        panic!("Depth texture size must be greater than 0");
                    }

                    if depth_size.x != inner.surface_size.x || depth_size.y != inner.surface_size.y
                    {
                        panic!("Depth texture size must match render target size");
                    }
                }

                inner.depth_target = Some(texture_inner.wgpu_view.clone());
                inner.depth_target_format = Some(format);
            }
            None => {
                inner.depth_target = None;
                inner.depth_target_format = None;
            }
        }
    }

    #[inline]
    pub fn set_push_constants(&mut self, _data: Option<&[u8]>) {
        let mut inner = self.inner.borrow_mut();

        match _data {
            Some(data) => {
                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    if inner.shader.is_none() {
                        panic!("Shader is not set");
                    }

                    let size = inner
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

                            bindings.iter().find_map(|binding| {
                                if let ShaderBindingType::PushConstant(size) = binding.ty {
                                    Some(size)
                                } else {
                                    None
                                }
                            })
                        })
                        .unwrap_or(0);

                    if data.len() > size as usize {
                        panic!("Data size must be less or equal to the push constant size");
                    }
                }

                let mut data = data.to_vec();
                if data.len() % 4 != 0 {
                    let padding = 4 - (data.len() % 4);
                    data.extend(vec![0; padding]);
                }

                inner.push_constant = Some(data);
            }
            None => {
                inner.push_constant = None;
                return;
            }
        }
    }

    #[inline]
    pub fn set_push_constants_raw<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
        data: Option<&[T]>,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();

            if inner.shader.is_none() {
                panic!("Shader is not set");
            }
        }

        match data {
            Some(data) => {
                let mut bytemuck_data: Vec<u8> = bytemuck::cast_slice(data).to_vec();

                if bytemuck_data.len() % 4 != 0 {
                    let padding = 4 - (bytemuck_data.len() % 4);
                    bytemuck_data.extend(vec![0; padding]);
                }

                self.set_push_constants(Some(&bytemuck_data));
            }
            None => {
                self.set_push_constants(None);
            }
        }
    }

    #[inline]
    pub fn set_push_constants_struct_raw<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
        data: Option<&[T]>,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();

            if inner.shader.is_none() {
                panic!("Shader is not set");
            }
        }

        match data {
            Some(data) => {
                let mut bytemuck_data: Vec<u8> = bytemuck::cast_slice(data).to_vec();

                if bytemuck_data.len() % 4 != 0 {
                    let padding = 4 - (bytemuck_data.len() % 4);
                    bytemuck_data.extend(vec![0; padding]);
                }

                self.set_push_constants(Some(&bytemuck_data));
            }
            None => {
                self.set_push_constants(None);
            }
        }
    }

    #[inline]
    pub fn set_attachment_sampler(
        &mut self,
        group: u32,
        binding: u32,
        sampler: Option<&TextureSampler>,
    ) {
        match sampler {
            Some(sampler) => {
                let inner = self.graphics.borrow();
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Sampler(sampler.make_wgpu(inner.device())),
                };

                drop(inner);

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }
    }

    #[inline]
    pub fn set_attachment_texture(&mut self, group: u32, binding: u32, texture: Option<&Texture>) {
        match texture {
            Some(texture) => {
                let inner = texture.inner.borrow();
                let attachment = BindGroupAttachment {
                    group,
                    binding,
                    attachment: BindGroupType::Texture(inner.wgpu_view.clone()),
                };

                drop(inner);

                self.insert_or_replace_attachment(group, binding, attachment);
            }
            None => {
                self.remove_attachment(group, binding);
            }
        }
    }

    #[inline]
    pub fn set_attachment_texture_storage(
        &mut self,
        group: u32,
        binding: u32,
        texture: Option<&Texture>,
    ) {
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
    }

    #[inline]
    pub fn set_attachment_uniform(&mut self, group: u32, binding: u32, buffer: Option<&Buffer>) {
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
    }

    #[inline]
    pub fn set_attachment_uniform_vec<T>(&mut self, group: u32, binding: u32, buffer: Option<Vec<T>>)
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.graphics.borrow_mut();

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
    }

    #[inline]
    pub fn set_attachment_uniform_raw<T>(&mut self, group: u32, binding: u32, buffer: Option<&[T]>)
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.graphics.borrow_mut();

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
    }

    #[inline]
    pub fn set_attachment_storage(&mut self, group: u32, binding: u32, buffer: Option<&Buffer>) {
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
    }

    #[inline]
    pub fn set_attachment_storage_raw<T>(&mut self, group: u32, binding: u32, buffer: Option<&[T]>)
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.graphics.borrow_mut();

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
    }

    #[inline]
    pub fn set_attachment_storage_vec<T>(&mut self, group: u32, binding: u32, buffer: Option<Vec<T>>)
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        match buffer {
            Some(buffer) => {
                let mut inner = self.graphics.borrow_mut();

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
    }

    #[inline]
    pub fn draw(&mut self, vertex_ranges: Range<u32>, num_of_instances: u32) {
        self.prepare_draw(false, vertex_ranges, 0, num_of_instances);
    }

    #[inline]
    pub fn draw_indexed(
        &mut self,
        index_ranges: Range<u32>,
        vertex_offset: i32,
        num_of_instances: u32,
    ) {
        self.prepare_draw(true, index_ranges, vertex_offset, num_of_instances);
    }

    #[inline]
    fn prepare_draw(
        &mut self,
        use_index_buffer: bool,
        ranges: Range<u32>,
        vertex_offset: i32,
        num_of_instances: u32,
    ) {
        // Checking if scissor and viewport are NonZero
        //
        // If any of them is set to zero, we skip the draw call, since wgpu will panic
        // if we try to draw with zero-sized viewport or scissor.
        {
            let inner = self.inner.borrow();

            if let Some((viewport, _, _)) = &inner.viewport {
                if viewport.w <= 0.0 || viewport.h <= 0.0 {
                    return;
                }
            }

            if let Some(scissor) = &inner.scissor {
                if scissor.w <= 0.0 || scissor.h <= 0.0 {
                    return;
                }
            }
        }

        // Validating the vertex and index buffers
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();
            if inner.vertex.is_none() {
                panic!("Vertex buffer is not set");
            }

            if use_index_buffer && inner.index.is_none() {
                panic!("Index buffer is not set");
            }
        }

        // Preparing the pipeline and bind group
        let (pipeline, bind_group, index_format) = self.prepare_pipeline();

        // Validating the index format, if it is required
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if index_format.is_none() && use_index_buffer {
                panic!(
                    "Index format is not set, setup with shader.set_index_format() or render_pass.set_shader_ex()"
                );
            }
        }

        let mut inner = self.inner.borrow_mut();

        let queue = RenderPassQueue {
            pipeline,
            bind_group,
            vbo: inner.vertex.clone(),
            ibo: if use_index_buffer {
                inner.index.clone()
            } else {
                None
            },
            itype: if use_index_buffer {
                Some(index_format.unwrap().into())
            } else {
                None
            },
            viewport: inner.viewport.clone(),
            scissor: inner.scissor.clone(),
            ty: DrawCallType::Direct {
                ranges,
                vertex_offset,
                num_of_instances,
            },
            push_constant: inner.push_constant.clone(),
        };

        inner.queues.push(queue);
    }

    #[inline]
    pub fn draw_indirect(&mut self, buffer: &Buffer, offset: u64) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if buffer.inner.borrow().usage.contains(BufferUsage::INDIRECT) {
            panic!("Buffer must have INDIRECT usage");
        }

        self.prepare_draw_indirect(buffer, offset, false);
    }

    #[inline]
    pub fn draw_indexed_indirect(&mut self, buffer: &Buffer, offset: u64) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if buffer.inner.borrow().usage.contains(BufferUsage::INDIRECT) {
            panic!("Buffer must have INDIRECT usage");
        }

        self.prepare_draw_indirect(buffer, offset, true);
    }

    #[inline]
    fn prepare_draw_indirect(&mut self, buffer: &Buffer, offset: u64, use_index_buffer: bool) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.borrow();
            if inner.vertex.is_none() {
                panic!("Vertex buffer is not set");
            }

            if use_index_buffer && inner.index.is_none() {
                panic!("Index buffer is not set");
            }
        }

        let (pipeline, bind_group, index_format) = self.prepare_pipeline();

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if index_format.is_none() && use_index_buffer {
                panic!(
                    "Index format is not set, setup with shader.set_index_format() or render_pass.set_shader_ex()"
                );
            }
        }

        let mut inner = self.inner.borrow_mut();
        let queue = RenderPassQueue {
            pipeline,
            bind_group,
            vbo: inner.vertex.clone(),
            ibo: if use_index_buffer {
                inner.index.clone()
            } else {
                None
            },
            itype: if use_index_buffer {
                Some(index_format.unwrap().into())
            } else {
                None
            },
            viewport: inner.viewport.clone(),
            scissor: inner.scissor.clone(),
            ty: DrawCallType::InDirect {
                buffer: buffer.inner.borrow().buffer.clone(),
                offset,
            },
            push_constant: inner.push_constant.clone(),
        };

        inner.queues.push(queue);
    }

    fn prepare_pipeline(
        &self,
    ) -> (
        wgpu::RenderPipeline,
        Vec<(u32, wgpu::BindGroup)>,
        Option<IndexBufferSize>,
    ) {
        let inner = self.inner.borrow();

        match &inner.shader {
            Some(RenderShaderBinding::Intermediate(shader_binding)) => {
                let bind_group_hash_key = {
                    let mut hasher = DefaultHasher::new();
                    hasher.write_u64(0u64); // Graphics shader hash id

                    for attachment in &inner.attachments {
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
                    let mut gpu_inner = self.graphics.borrow_mut();

                    match gpu_inner.get_bind_group(bind_group_hash_key) {
                        Some(bind_group) => bind_group,
                        None => {
                            let mut bind_group_attachments: HashMap<
                                u32,
                                Vec<wgpu::BindGroupEntry>,
                            > = inner.attachments.iter().fold(HashMap::new(), |mut map, e| {
                                let (group, binding, attachment) =
                                    (e.group, e.binding, &e.attachment);

                                let entry = match attachment {
                                    BindGroupType::Uniform(buffer) => wgpu::BindGroupEntry {
                                        binding,
                                        resource: wgpu::BindingResource::Buffer(
                                            wgpu::BufferBinding {
                                                buffer,
                                                offset: 0,
                                                size: None,
                                            },
                                        ),
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
                                        resource: wgpu::BindingResource::Buffer(
                                            wgpu::BufferBinding {
                                                buffer,
                                                offset: 0,
                                                size: None,
                                            },
                                        ),
                                    },
                                    BindGroupType::TextureStorage(texture) => {
                                        wgpu::BindGroupEntry {
                                            binding,
                                            resource: wgpu::BindingResource::TextureView(texture),
                                        }
                                    }
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
                    let mut hasher = DefaultHasher::new();
                    shader_binding.hash(&mut hasher);

                    for target in &inner.render_targets {
                        target.format.hash(&mut hasher);
                        target.blend.hash(&mut hasher);
                        target.write_mask.hash(&mut hasher);
                    }

                    inner.depth_target_format.hash(&mut hasher);
                    inner.multi_sample_count.hash(&mut hasher);

                    hasher.finish()
                };

                let pipeline = {
                    let mut graphics_inner = self.graphics.borrow_mut();
                    match graphics_inner.get_graphics_pipeline(pipeline_hash_key) {
                        Some(pipeline) => pipeline,
                        None => {
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

                            let mut pipeline_desc = GraphicsPipelineDesc {
                                shaders: shader_binding.shader.clone(),
                                entry_point: shader_binding.shader_entry.clone(),
                                render_target: Vec::with_capacity(inner.render_targets.len()),
                                depth_stencil: inner.depth_target_format,
                                vertex_desc,
                                primitive_state,
                                bind_group_layout: layout,
                                msaa_count: inner.multi_sample_count.unwrap_or(1),
                            };

                            for target in &inner.render_targets {
                                pipeline_desc.render_target.push((
                                    target.format,
                                    target.blend,
                                    target.write_mask,
                                ));
                            }

                            graphics_inner
                                .create_graphics_pipeline(pipeline_hash_key, pipeline_desc)
                        }
                    }
                };

                (
                    pipeline,
                    bind_group_attachments,
                    shader_binding.index_format,
                )
            }
            Some(RenderShaderBinding::Pipeline(pipeline)) => {
                let mut pipeline_desc = pipeline.pipeline_desc.clone();

                for target in &inner.render_targets {
                    pipeline_desc.render_target.push((
                        target.format,
                        target.blend,
                        target.write_mask,
                    ));
                }

                pipeline_desc.depth_stencil = inner.depth_target_format;
                pipeline_desc.msaa_count = inner.multi_sample_count.unwrap_or(1);

                let pipeline_hash_key = {
                    let mut hasher = DefaultHasher::new();
                    pipeline_desc.hash(&mut hasher);

                    for target in &inner.render_targets {
                        target.format.hash(&mut hasher);
                        target.blend.hash(&mut hasher);
                        target.write_mask.hash(&mut hasher);
                    }

                    inner.depth_target_format.hash(&mut hasher);
                    inner.multi_sample_count.hash(&mut hasher);

                    hasher.finish()
                };

                let wgpu_pipeline = {
                    let mut graphics_inner = self.graphics.borrow_mut();
                    match graphics_inner.get_graphics_pipeline(pipeline_hash_key) {
                        Some(pipeline) => pipeline,
                        None => graphics_inner
                            .create_graphics_pipeline(pipeline_hash_key, pipeline_desc),
                    }
                };

                let bind_group_attachments = pipeline.bind_group.clone();
                let index_format = pipeline.index_format.clone();

                (wgpu_pipeline, bind_group_attachments, index_format)
            }
            None => {
                panic!("Shader is not set");
            }
        }
    }

    #[inline]
    pub fn begin_drawing(&mut self) -> Option<DrawingContext> {
        DrawingContext::new(self.clone())
    }

    pub(crate) fn end(&mut self) {
        let inner = self.inner.borrow_mut();
        let mut cmd = inner.cmd.borrow_mut();

        let clear_color = inner.clear_color.unwrap_or(Color::BLACK);

        let load_op = if clear_color.a <= 0.0 {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: clear_color.r as f64,
                g: clear_color.g as f64,
                b: clear_color.b as f64,
                a: clear_color.a as f64,
            })
        };

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if inner.multi_sample_count.is_some()
                && inner.multi_sample_target.len() != inner.render_targets.len()
            {
                panic!("Multi sample target must match the number of render targets");
            }
        }

        let mut color_attachments = Vec::with_capacity(inner.render_targets.len());
        let has_msaa = inner.multi_sample_count.is_some();

        for i in 0..inner.render_targets.len() {
            let target_view = if has_msaa {
                &inner.multi_sample_target[i]
            } else {
                &inner.render_targets[i].view
            };

            color_attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: if has_msaa {
                    Some(&inner.render_targets[i].view)
                } else {
                    None
                },
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        let mut depth_stencil_attachment = None;
        if let Some(depth_target) = inner.depth_target.as_ref() {
            depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_target,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            });
        }

        let mut render_pass = cmd.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: color_attachments.as_slice(),
            depth_stencil_attachment,
            ..Default::default()
        });

        for queue in &inner.queues {
            render_pass.set_pipeline(&queue.pipeline);

            for (group, bind) in &queue.bind_group {
                render_pass.set_bind_group(*group, bind, &[]);
            }

            if let Some(vbo) = &queue.vbo {
                render_pass.set_vertex_buffer(0, vbo.slice(..));
            }

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(pc) = &queue.push_constant {
                use wgpu::ShaderStages;

                render_pass.set_push_constants(ShaderStages::all(), 0, pc);
            }

            if let Some(scissor) = queue.scissor.as_ref() {
                if scissor.w <= 0.0 || scissor.h <= 0.0 {
                    continue; // Skip if scissor is invalid
                }

                render_pass.set_scissor_rect(
                    scissor.x as u32,
                    scissor.y as u32,
                    scissor.w as u32,
                    scissor.h as u32,
                );
            }

            if let Some(viewport) = queue.viewport.as_ref() {
                let size = viewport.0;
                let min_depth = viewport.1;
                let max_depth = viewport.2;

                if size.w <= 0.0 || size.h <= 0.0 {
                    continue; // Skip if viewport is invalid
                }

                render_pass.set_viewport(size.x, size.y, size.w, size.h, min_depth, max_depth);
            }

            match &queue.ty {
                DrawCallType::Direct {
                    ranges,
                    vertex_offset,
                    num_of_instances,
                } => {
                    if let Some(ibo) = &queue.ibo {
                        render_pass.set_index_buffer(ibo.slice(..), queue.itype.unwrap());
                        render_pass.draw_indexed(
                            ranges.clone(),
                            *vertex_offset,
                            0..*num_of_instances,
                        );
                    } else {
                        render_pass.draw(ranges.clone(), 0..*num_of_instances);
                    }
                }
                DrawCallType::InDirect { buffer, offset } => {
                    if let Some(ibo) = &queue.ibo {
                        render_pass.set_index_buffer(ibo.slice(..), queue.itype.unwrap());
                        render_pass.draw_indexed_indirect(buffer, *offset);
                    } else {
                        render_pass.draw_indirect(buffer, *offset);
                    }
                }
            }
        }

        inner.atomic_pass.store(false, Ordering::Relaxed);
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }

        self.end();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RenderpassRenderTarget {
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub blend: Option<wgpu::BlendState>,
    pub write_mask: Option<wgpu::ColorWrites>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderPassInner {
    pub cmd: ArcRef<wgpu::CommandEncoder>,
    pub atomic_pass: Arc<AtomicBool>,

    pub render_targets: Vec<RenderpassRenderTarget>,
    pub depth_target: Option<wgpu::TextureView>,
    pub depth_target_format: Option<wgpu::TextureFormat>,

    pub surface_size: Point2,

    pub multi_sample_target: Vec<wgpu::TextureView>,
    pub multi_sample_count: Option<u32>,

    pub clear_color: Option<Color>,
    pub viewport: Option<(RectF, f32, f32)>,
    pub scissor: Option<RectF>,

    pub vertex: Option<wgpu::Buffer>,
    pub index: Option<wgpu::Buffer>,

    pub shader: Option<RenderShaderBinding>,
    #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
    pub shader_reflection: Option<Vec<ShaderReflect>>,

    pub attachments: Vec<BindGroupAttachment>,
    pub push_constant: Option<Vec<u8>>,

    pub queues: Vec<RenderPassQueue>,
}

#[derive(Clone, Debug)]
pub(crate) enum RenderpassAttachment<'a> {
    SurfaceTexture(&'a SurfaceTexture),
    Texture(&'a Texture),
}

#[derive(Clone, Debug)]
pub struct RenderpassBuilder<'a> {
    gpu: ArcRef<GPUInner>,
    cmd: ArcRef<wgpu::CommandEncoder>,
    atomic_pass: Arc<AtomicBool>,

    color_attachments: Vec<(RenderpassAttachment<'a>, Option<BlendState>)>,
    msaa_attachments: Vec<&'a Texture>,
    depth_attachment: Option<&'a Texture>,
}

impl<'a> RenderpassBuilder<'a> {
    pub(crate) fn new(
        gpu: ArcRef<GPUInner>,
        cmd: ArcRef<wgpu::CommandEncoder>,
        atomic_pass: Arc<AtomicBool>,
    ) -> Self {
        Self {
            gpu,
            cmd,
            atomic_pass,

            color_attachments: Vec::new(),
            msaa_attachments: Vec::new(),
            depth_attachment: None,
        }
    }

    /// Add swapchain's SurfaceTexture color attachment.
    pub fn add_surface_color_attachment(
        mut self,
        surface: &'a SurfaceTexture,
        blend: Option<&BlendState>,
    ) -> Self {
        self.color_attachments.push((
            RenderpassAttachment::SurfaceTexture(surface),
            blend.cloned(),
        ));

        self
    }

    pub fn add_color_attachment(
        mut self,
        texture: &'a Texture,
        blend: Option<&BlendState>,
    ) -> Self {
        self.color_attachments
            .push((RenderpassAttachment::Texture(texture), blend.cloned()));

        self
    }

    pub fn add_msaa_attachment(mut self, texture: &'a Texture) -> Self {
        self.msaa_attachments.push(texture);

        self
    }

    pub fn set_depth_attachment(mut self, texture: &'a Texture) -> Self {
        self.depth_attachment = Some(texture);

        self
    }

    pub fn build(self) -> Result<RenderPass, RenderPassBuildError> {
        let mut surface_size = None;

        let mut color_attachments = Vec::with_capacity(self.color_attachments.len());
        for (attachment, blend) in self.color_attachments {
            let (view, format, size) = match attachment {
                RenderpassAttachment::SurfaceTexture(surface_texture) => {
                    let view = surface_texture.get_view();
                    let format = surface_texture.get_format();
                    let size = surface_texture.get_size();

                    (view, format, Point2::new(size.width, size.height))
                }
                RenderpassAttachment::Texture(texture) => {
                    let texture_inner = texture.inner.borrow();

                    if !texture_inner
                        .usages
                        .contains(TextureUsage::RenderAttachment)
                    {
                        return Err(RenderPassBuildError::ColorAttachmentNotRenderTarget);
                    }

                    if texture_inner.size.x == 0 || texture_inner.size.y == 0 {
                        return Err(RenderPassBuildError::MismatchedAttachmentSize(
                            Point2::new(0.0, 0.0),
                            texture_inner.size,
                        ));
                    }

                    if texture_inner.sample_count != SampleCount::SampleCount1 {
                        return Err(RenderPassBuildError::ColorAttachmentMultiSampled);
                    }

                    (
                        texture_inner.wgpu_view.clone(),
                        texture_inner.format.into(),
                        texture_inner.size,
                    )
                }
            };

            if surface_size.is_some() {
                let surface_size = surface_size.unwrap();
                if surface_size != size {
                    return Err(RenderPassBuildError::MismatchedAttachmentSize(
                        surface_size,
                        size,
                    ));
                }
            }

            if surface_size.is_none() {
                surface_size = Some(size);
            }

            color_attachments.push(RenderpassRenderTarget {
                view,
                format,
                blend: blend.map(|b| b.create_wgpu_blend_state()),
                write_mask: blend.map(|b| b.create_wgpu_color_write_mask()),
            });
        }

        let mut multi_sample_target = Vec::with_capacity(self.msaa_attachments.len());
        let mut multi_sample_count = None;

        for msaa_texture in self.msaa_attachments {
            let texture_inner = msaa_texture.inner.borrow();

            if !texture_inner
                .usages
                .contains(TextureUsage::RenderAttachment)
            {
                return Err(RenderPassBuildError::MsaaTextureNotRenderAttachment);
            }

            if texture_inner.sample_count == SampleCount::SampleCount1 {
                return Err(RenderPassBuildError::MsaaTextureNotMultiSampled);
            }

            if texture_inner.size.x == 0 || texture_inner.size.y == 0 {
                return Err(RenderPassBuildError::MsaaTextureInvalidSize(Point2::new(
                    0.0, 0.0,
                )));
            }

            if surface_size.is_some() {
                let surface_size = surface_size.unwrap();
                if surface_size != texture_inner.size {
                    return Err(RenderPassBuildError::MismatchedAttachmentSize(
                        surface_size,
                        texture_inner.size,
                    ));
                }
            }

            let sample_count: u32 = texture_inner.sample_count.into();

            if multi_sample_count.is_some() && multi_sample_count.unwrap() != sample_count {
                return Err(RenderPassBuildError::MismatchedAttachmentSampleCount(
                    multi_sample_count.unwrap(),
                    sample_count,
                ));
            }

            if multi_sample_count.is_none() {
                multi_sample_count = Some(sample_count);
            }

            multi_sample_target.push(texture_inner.wgpu_view.clone());
        }

        let mut depth_view = None;
        let mut depth_format = None;

        if let Some(depth_texture) = self.depth_attachment {
            let texture_inner = depth_texture.inner.borrow();

            if !texture_inner
                .usages
                .contains(TextureUsage::RenderAttachment)
            {
                return Err(RenderPassBuildError::DepthTextureNotRenderAttachment);
            }

            if texture_inner.size.x == 0 || texture_inner.size.y == 0 {
                return Err(RenderPassBuildError::DepthTextureInvalidSize(Point2::new(
                    0.0, 0.0,
                )));
            }

            if texture_inner.format != TextureFormat::Depth32Float
                && texture_inner.format != TextureFormat::Depth24PlusStencil8
            {
                return Err(RenderPassBuildError::DepthTextureFormatNotSupported(
                    texture_inner.format,
                ));
            }

            if surface_size.is_some() {
                let surface_size = surface_size.unwrap();
                if surface_size != texture_inner.size {
                    return Err(RenderPassBuildError::MismatchedAttachmentSize(
                        surface_size,
                        texture_inner.size,
                    ));
                }
            }

            if surface_size.is_none() {
                surface_size = Some(texture_inner.size);
            }

            depth_view = Some(texture_inner.wgpu_view.clone());
            depth_format = Some(texture_inner.format.into());
        }

        if surface_size.is_none() {
            return Err(RenderPassBuildError::NoColorOrDepthAttachment);
        }

        let renderpass = RenderPass::new(self.gpu, self.cmd, self.atomic_pass);
        {
            let mut inner = renderpass.inner.borrow_mut();

            inner.render_targets = color_attachments;
            inner.multi_sample_target = multi_sample_target;
            inner.multi_sample_count = multi_sample_count;
            inner.depth_target = depth_view;
            inner.depth_target_format = depth_format;
            inner.surface_size = surface_size.unwrap();
        }

        Ok(renderpass)
    }
}

pub enum RenderPassBuildError {
    NoColorOrDepthAttachment,
    ColorAttachmentNotRenderTarget,
    ColorAttachmentMultiSampled,
    MismatchedAttachmentCount(usize, usize),
    MismatchedAttachmentSize(Point2, Point2),
    MismatchedAttachmentSampleCount(u32, u32),
    MismatchedAttachmentFormat(TextureFormat, TextureFormat),
    MsaaTextureNotMultiSampled,
    MsaaTextureNotRenderAttachment,
    MsaaTextureInvalidSize(Point2),
    DepthTextureNotRenderAttachment,
    DepthTextureInvalidSize(Point2),
    DepthTextureFormatNotSupported(TextureFormat),
    SwapchainError(String),
}

impl std::fmt::Display for RenderPassBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderPassBuildError::NoColorOrDepthAttachment => write!(f, "No color attachment provided"),
            RenderPassBuildError::ColorAttachmentNotRenderTarget => {
                write!(f, "Color attachment is not a render target")
            }
            RenderPassBuildError::ColorAttachmentMultiSampled => {
                write!(f, "Color attachment is multi-sampled")
            }
            RenderPassBuildError::MismatchedAttachmentCount(expected, actual) => {
                write!(f, "Expected {} attachments, but got {}", expected, actual)
            }
            RenderPassBuildError::MismatchedAttachmentSize(expected, actual) => write!(
                f,
                "Expected attachment size {:?}, but got {:?}",
                expected, actual
            ),
            RenderPassBuildError::MismatchedAttachmentSampleCount(expected, actual) => {
                write!(f, "Expected sample count {}, but got {}", expected, actual)
            }
            RenderPassBuildError::MismatchedAttachmentFormat(expected, actual) => {
                write!(f, "Expected format {:?}, but got {:?}", expected, actual)
            }
            RenderPassBuildError::MsaaTextureNotMultiSampled => {
                write!(f, "MSAA texture is not multi-sampled")
            }
            RenderPassBuildError::MsaaTextureNotRenderAttachment => {
                write!(f, "MSAA texture is not a render attachment")
            }
            RenderPassBuildError::MsaaTextureInvalidSize(size) => {
                write!(f, "MSAA texture has invalid size {:?}", size)
            }
            RenderPassBuildError::DepthTextureNotRenderAttachment => {
                write!(f, "Depth texture is not a render attachment")
            }
            RenderPassBuildError::DepthTextureInvalidSize(size) => {
                write!(f, "Depth texture has invalid size {:?}", size)
            }
            RenderPassBuildError::DepthTextureFormatNotSupported(format) => {
                write!(f, "Depth texture format {:?} is not supported", format)
            }
            RenderPassBuildError::SwapchainError(err) => write!(f, "Swapchain error: {}", err),
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct IntermediateRenderPipeline {
    pub shader: (wgpu::ShaderModule, wgpu::ShaderModule),
    pub vertex_attribute: (u64, Vec<wgpu::VertexAttribute>),
    pub shader_entry: (String, String),
    pub layout: Vec<BindGroupLayout>,
    pub topology: ShaderTopology,
    pub cull_mode: Option<ShaderCullMode>,
    pub front_face: ShaderFrontFace,
    pub polygon_mode: ShaderPollygonMode,
    pub index_format: Option<IndexBufferSize>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderPassQueue {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: Vec<(u32, wgpu::BindGroup)>,

    pub vbo: Option<wgpu::Buffer>,
    pub ibo: Option<wgpu::Buffer>,
    pub itype: Option<wgpu::IndexFormat>,

    pub viewport: Option<(RectF, f32, f32)>,
    pub scissor: Option<RectF>,

    pub ty: DrawCallType,
    pub push_constant: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub(crate) enum RenderShaderBinding {
    Intermediate(IntermediateRenderPipeline),
    Pipeline(RenderPipeline),
}

#[derive(Debug, Clone)]
pub enum DrawCallType {
    Direct {
        ranges: Range<u32>,
        vertex_offset: i32,
        num_of_instances: u32,
    },

    InDirect {
        buffer: wgpu::Buffer,
        offset: u64,
    },
}
