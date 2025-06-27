use std::cell::{RefCell, RefMut};

use crate::{
    dbg_log,
    gpu::AttachmentConfigurator,
    math::{Color, Rect, RectF, Vector2, Vector3, Vertex},
    prelude::{
        BufferBuilder, BufferUsage, GraphicsShader, GraphicsShaderBuilder, IndexBufferSize,
        ShaderBindingType, Texture, TextureBlend, TextureBuilder, TextureFormat, TextureSampler,
        TextureUsage,
    },
    utils::ArcRef,
};

use super::RenderPass;

pub(crate) struct DrawingContextInner {
    pass: RenderPass,

    vertices: Vec<Vertex>,
    indices: Vec<u16>,

    texture: Option<(Texture, TextureBlend, TextureSampler)>,
    shader: Option<GraphicsShader>,
    scissor: Option<RectF>,
    viewport: Option<RectF>,
    current_queue: Option<DrawingQueue>,
    queue: Vec<DrawingQueue>,
}

pub(crate) struct DrawingQueue {
    pub texture: Option<(Texture, TextureBlend, TextureSampler)>,
    pub shader: Option<GraphicsShader>,

    pub scissors: Option<RectF>,
    pub viewport: Option<RectF>,

    pub start_index: u32,
    pub start_vertex: u32,
    pub count: u32,
}

pub struct DrawingContext {
    pub(crate) inner: ArcRef<DrawingContextInner>,

    pub(crate) vertex_cache: Vec<Vector2>,
    pub(crate) index_cache: Vec<u16>,
}

pub(crate) const VERTEX_DRAWING_SHADER: &str = r#"
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
}"#;

pub(crate) const FRAGMENT_DRAWING_SHADER: &str = r#"
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

impl DrawingContext {
    pub(crate) fn new(pass: RenderPass) -> Option<Self> {
        if pass.graphics.borrow().drawing_default_shader.is_none() {
            let shader = GraphicsShaderBuilder::new(ArcRef::clone(&pass.graphics))
                // .set_source(VERTEX_DRAWING_SHADER)
                .set_vertex_code(VERTEX_DRAWING_SHADER)
                .set_fragment_code(FRAGMENT_DRAWING_SHADER)
                .build();

            if shader.is_err() {
                return None;
            }

            let mut shader = shader.unwrap();
            shader
                .set_vertex_index_ty(Some(IndexBufferSize::U16))
                .expect("Failed to set vertex index type");

            pass.graphics.borrow_mut().drawing_default_shader = Some(shader);
        }

        if pass.graphics.borrow().drawing_default_texture.is_none() {
            let data = vec![255u8, 255, 255, 255];
            let texture = TextureBuilder::new(ArcRef::clone(&pass.graphics))
                .with_raw(&data, Rect::with_size(1, 1), TextureFormat::Bgra8Unorm)
                .with_usage(TextureUsage::Sampler)
                .build();

            if texture.is_err() {
                return None;
            }

            pass.graphics.borrow_mut().drawing_default_texture = texture.ok();
        }

        if pass.graphics.borrow().drawing_vertex_buffer.is_none() {
            let vertex_buffer = BufferBuilder::<Vertex>::new(ArcRef::clone(&pass.graphics))
                .set_usage(BufferUsage::VERTEX | BufferUsage::COPY_DST)
                .set_data_empty(1)
                .build();

            if vertex_buffer.is_err() {
                return None;
            }

            pass.graphics.borrow_mut().drawing_vertex_buffer = vertex_buffer.ok();
        }

        if pass.graphics.borrow().drawing_index_buffer.is_none() {
            let index_buffer = BufferBuilder::<u16>::new(ArcRef::clone(&pass.graphics))
                .set_usage(BufferUsage::INDEX | BufferUsage::COPY_DST)
                .set_data_empty(1)
                .build();

            if index_buffer.is_err() {
                return None;
            }

            pass.graphics.borrow_mut().drawing_index_buffer = index_buffer.ok();
        }

        let inner = DrawingContextInner {
            pass: pass,
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
            shader: None,
            scissor: None,
            viewport: None,
            current_queue: None,
            queue: Vec::new(),
        };

        Some(DrawingContext {
            inner: ArcRef::new(inner),

            vertex_cache: Vec::new(),
            index_cache: Vec::new(),
        })
    }

    fn construct_line(a: Vector2, b: Vector2, thickness: f32) -> Option<([Vector2; 4], [u16; 6])> {
        let dir = b - a;
        let len = dir.length();
        if len == 0.0 {
            return None;
        }

        let dir = dir / len;
        let perp = Vector2::new(-dir.y, dir.x) * (thickness * 0.5);

        let vertices = [a + perp, b + perp, b - perp, a - perp];

        let indices = [0, 1, 2, 0, 2, 3];

        Some((vertices, indices))
    }

    #[allow(dead_code)]
    fn construct_quad(pos: Vector2, size: Vector2) -> ([Vector2; 4], [u16; 6]) {
        let vertices = [
            Vector2::new(pos.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y + size.y),
            Vector2::new(pos.x, pos.y + size.y),
        ];

        let indices = [0, 1, 2, 0, 2, 3];

        (vertices, indices)
    }

    pub fn rectangle(&mut self, pos: Vector2, size: Vector2, thickness: f32, color: Color) {
        let corners = [
            pos,
            pos + Vector2::new(size.x, 0.0),
            pos + size,
            pos + Vector2::new(0.0, size.y),
        ];

        let all_vertices = &mut self.vertex_cache;
        let all_indices = &mut self.index_cache;
        let mut index_offset = 0u16;

        vec_clear(all_vertices);
        vec_clear(all_indices);

        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let line = Self::construct_line(a, b, thickness);
            if line.is_none() {
                continue;
            }

            let (vertices, mut indices) = line.unwrap();

            indices.iter_mut().for_each(|idx| *idx += index_offset);
            index_offset += vertices.len() as u16;

            all_vertices.extend(vertices);
            all_indices.extend(indices);
        }

        Self::submit_geometry(
            &mut self.inner.borrow_mut(),
            &all_vertices,
            &all_indices,
            color,
        );
    }

    pub fn line(&mut self, a: Vector2, b: Vector2, thickness: f32, color: Color) {
        let line = Self::construct_line(a, b, thickness);
        if line.is_none() {
            return;
        }

        let (vertices, indices) = line.unwrap();

        Self::submit_geometry(&mut self.inner.borrow_mut(), &vertices, &indices, color);
    }

    pub fn rectangle_filled(&mut self, pos: Vector2, size: Vector2, color: Color) {
        let vertices = [
            Vector2::new(pos.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y + size.y),
            Vector2::new(pos.x, pos.y + size.y),
        ];

        let indices = [0, 1, 2, 0, 2, 3];

        Self::submit_geometry(&mut self.inner.borrow_mut(), &vertices, &indices, color);
    }

    pub fn rectangle_filled_colors(
        &mut self,
        pos: Vector2,
        size: Vector2,
        color_tl: Color,
        color_tr: Color,
        color_br: Color,
        color_bl: Color,
    ) {
        let vertices = [
            Vector2::new(pos.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y + size.y),
            Vector2::new(pos.x, pos.y + size.y),
        ];

        let colors = [color_tl, color_tr, color_br, color_bl];

        let mut inner = self.inner.borrow_mut();
        let base_index = inner.vertices.len() as u16;
        let indices = [
            base_index + 0,
            base_index + 1,
            base_index + 2,
            base_index + 0,
            base_index + 2,
            base_index + 3,
        ];

        Self::push_queue(&mut inner, indices.len() as u32);

        let uvs = [
            Vector2::new(0.0, 0.0), // Top-left
            Vector2::new(1.0, 0.0), // Top-right
            Vector2::new(1.0, 1.0), // Bottom-right
            Vector2::new(0.0, 1.0), // Bottom-left
        ];

        for (i, vertex) in vertices.iter().enumerate() {
            inner.vertices.push(Vertex::new(
                Vector3::new(vertex.x, vertex.y, 0.0),
                colors[i],
                uvs[i],
            ));
        }

        inner.indices.extend_from_slice(&indices);
    }

    pub fn triangle(&mut self, a: Vector2, b: Vector2, c: Vector2, thickness: f32, color: Color) {
        let points = [
            Vector2::new(a.x, a.y),
            Vector2::new(b.x, b.y),
            Vector2::new(c.x, c.y),
        ];

        let all_vertices = &mut self.vertex_cache;
        let all_indices = &mut self.index_cache;

        vec_clear(all_vertices);
        vec_clear(all_indices);

        let mut index_offset = 0u16;
        for i in 0..3 {
            let a = points[i];
            let b = points[(i + 1) % 3];

            let line = Self::construct_line(a, b, thickness);
            if line.is_none() {
                continue;
            }

            let (vertices, mut indices) = line.unwrap();

            indices.iter_mut().for_each(|idx| *idx += index_offset);
            index_offset += vertices.len() as u16;

            all_vertices.extend(vertices);
            all_indices.extend(indices);
        }

        if all_indices.is_empty() {
            return;
        }

        Self::submit_geometry(
            &mut self.inner.borrow_mut(),
            &all_vertices,
            &all_indices,
            color,
        );
    }

    pub fn triangle_filled(&mut self, a: Vector2, b: Vector2, c: Vector2, color: Color) {
        let vertices = [
            Vector2::new(a.x, a.y),
            Vector2::new(b.x, b.y),
            Vector2::new(c.x, c.y),
        ];

        let indices = [0, 1, 2];

        Self::submit_geometry(&mut self.inner.borrow_mut(), &vertices, &indices, color);
    }

    pub fn circle(
        &mut self,
        center: Vector2,
        radius: f32,
        segments: u32,
        thickness: f32,
        color: Color,
    ) {
        if segments < 3 {
            return;
        }

        let angle_step = std::f32::consts::PI * 2.0 / segments as f32;

        let mut vertices = Vec::with_capacity(segments as usize * 2);
        let mut indices = Vec::with_capacity(segments as usize * 6);

        for i in 0..segments {
            let angle_a = i as f32 * angle_step;
            let angle_b = (i + 1) as f32 * angle_step;

            let a = Vector2::new(
                center.x + radius * angle_a.cos(),
                center.y + radius * angle_a.sin(),
            );
            let b = Vector2::new(
                center.x + radius * angle_b.cos(),
                center.y + radius * angle_b.sin(),
            );

            let line = Self::construct_line(a, b, thickness);
            if line.is_none() {
                continue;
            }

            let (line_vertices, line_indices) = line.unwrap();

            let base_index = vertices.len() as u16;
            vertices.extend(line_vertices);
            indices.extend(line_indices.into_iter().map(|i| i + base_index));
        }

        if indices.is_empty() {
            return;
        }

        Self::submit_geometry(&mut self.inner.borrow_mut(), &vertices, &indices, color);
    }

    pub fn circle_filled(&mut self, center: Vector2, radius: f32, segments: u32, color: Color) {
        if segments < 3 {
            return;
        }

        let angle_step = std::f32::consts::PI * 2.0 / segments as f32;
        let vertices = &mut self.vertex_cache;
        let indices = &mut self.index_cache;

        vec_clear(vertices);
        vec_clear(indices);

        vertices.push(Vector2::new(center.x, center.y));

        for i in 0..segments {
            let angle = angle_step * i as f32;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();

            vertices.push(Vector2::new(x, y));
            indices.push(i as u16 + 1);
        }

        triangle_fan_to_list_indices_ref(&mut *indices);

        if indices.is_empty() {
            return;
        }

        Self::submit_geometry(&mut self.inner.borrow_mut(), &vertices, &indices, color);
    }

    fn submit_geometry(
        inner: &mut RefMut<'_, DrawingContextInner>,
        vertices: &[Vector2],
        indices: &[u16],
        color: Color,
    ) {
        if vertices.is_empty() || indices.is_empty() {
            return;
        }

        let base_index = inner.vertices.len() as u16;
        let indices: Vec<u16> = indices.iter().map(|i| i + base_index).collect();
        Self::push_queue(inner, indices.len() as u32);

        // Compute bounding box
        let (min_x, max_x) = vertices
            .iter()
            .map(|v| v.x)
            .fold((f32::MAX, f32::MIN), |(min, max), x| {
                (min.min(x), max.max(x))
            });
        let (min_y, max_y) = vertices
            .iter()
            .map(|v| v.y)
            .fold((f32::MAX, f32::MIN), |(min, max), y| {
                (min.min(y), max.max(y))
            });
        let width = max_x - min_x;
        let height = max_y - min_y;

        // Normalize UVs
        for vertex in vertices {
            let uv = Vector2::new((vertex.x - min_x) / width, (vertex.y - min_y) / height);

            inner.vertices.push(Vertex::new(
                Vector3::new(vertex.x, vertex.y, 0.0),
                color,
                uv,
            ));
        }

        inner.indices.extend_from_slice(&indices);
    }

    pub fn set_scissor(&mut self, scissor: RectF) {
        let mut inner = self.inner.borrow_mut();
        inner.scissor = Some(scissor);
    }

    pub fn set_viewport(&mut self, viewport: RectF) {
        let mut inner = self.inner.borrow_mut();
        inner.viewport = Some(viewport);
    }

    pub fn set_texture(&mut self, texture: Option<&Texture>) {
        self.set_texture_ex(texture, None, None);
    }

    pub fn set_texture_ex(
        &mut self,
        texture: Option<&Texture>,
        blend: Option<TextureBlend>,
        sampler: Option<TextureSampler>,
    ) {
        let mut inner = self.inner.borrow_mut();

        match texture {
            Some(texture) => {
                let texture_ref = texture.inner.borrow();

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                if !texture_ref.usages.contains(TextureUsage::Sampler) {
                    panic!("Texture must be created with TextureUsage::Sampler");
                }

                let blend = blend.unwrap_or(texture_ref.blend.clone());
                let sampler = sampler.unwrap_or(texture_ref.sampler_info.clone());

                inner.texture = Some((texture.clone(), blend, sampler));
            }
            None => {
                inner.texture = None;
            }
        }
    }

    pub fn set_shader(&mut self, shader: Option<&GraphicsShader>) {
        let mut inner = self.inner.borrow_mut();

        match shader {
            Some(shader) => {
                let shader_ref = shader.inner.borrow();

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                {
                    let mut fullfiled = false;
                    for binding in shader_ref.reflection.iter() {
                        use crate::prelude::ShaderReflect;

                        let bindings = match binding {
                            ShaderReflect::Fragment { bindings, .. } => bindings,
                            ShaderReflect::VertexFragment { bindings, .. } => bindings,
                            _ => continue,
                        };

                        if bindings.iter().any(|b| {
                            b.group == 0
                                && b.binding == 0
                                && matches!(b.ty, ShaderBindingType::Texture(_))
                        }) && bindings.iter().any(|b| {
                            b.group == 0
                                && b.binding == 1
                                && matches!(b.ty, ShaderBindingType::Sampler(_))
                        }) {
                            fullfiled = true;
                            break;
                        }
                    }

                    if !fullfiled {
                        panic!(
                            "Required shader bindings where group 0, binding 0 for texture or group 0, binding 1 are missing for sampler"
                        );
                    }
                }

                inner.shader = Some(shader.clone());
            }
            None => {
                inner.shader = None;
            }
        }
    }

    pub(crate) fn push_queue(inner: &mut RefMut<'_, DrawingContextInner>, index_count: u32) {
        let mut push_new_queue = false;

        if inner.current_queue.is_some() {
            let ref_queue = inner.current_queue.as_ref().unwrap();

            // Check if current queue has the same texture, if not push the queue
            let texture_changed = match (&ref_queue.texture, &inner.texture) {
                (None, None) => false,
                (Some(_), None) | (None, Some(_)) => true,
                (
                    Some((old_texture, old_blend, old_sampler)),
                    Some((new_texture, new_blend, new_sampler)),
                ) => {
                    old_texture != new_texture
                        || old_blend != new_blend
                        || old_sampler != new_sampler
                }
            };

            if texture_changed {
                push_new_queue = true;
            }

            // Check if current queue has the same scissor, if not push the queue
            if ref_queue.scissors != inner.scissor {
                push_new_queue = true;
            }

            // Check if current queue has the same viewport, if not push the queue
            if ref_queue.viewport != inner.viewport {
                push_new_queue = true;
            }

            // check if current queue has the same shader, if not push the queue
            if ref_queue.shader != inner.shader {
                push_new_queue = true;
            }
        } else {
            push_new_queue = true;
        }

        // Figure a way to push queue with correct start, and count
        if push_new_queue {
            if let Some(queue) = inner.current_queue.take() {
                inner.queue.push(queue);
            }

            inner.current_queue = Some(DrawingQueue {
                texture: inner.texture.clone(),
                shader: None,
                scissors: inner.scissor.clone(),
                viewport: inner.viewport.clone(),
                start_index: inner.indices.len() as u32,
                start_vertex: 0, // TODO: Fix this
                count: index_count,
            });
        } else {
            let queue = inner.current_queue.as_mut().unwrap();
            queue.count += index_count;
        }
    }

    pub(crate) fn end(&mut self) {
        let mut inner = self.inner.borrow_mut();

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if inner.vertices.is_empty() {
            dbg_log!(
                "DrawingContext::end: No vertices to draw, did you forget to call a drawing function?"
            );

            return;
        }

        if let Some(queue) = inner.current_queue.take() {
            inner.queue.push(queue);
        }

        let mut queues = inner.queue.drain(..).collect::<Vec<_>>();
        let mut vertices = inner.vertices.drain(..).collect::<Vec<_>>();
        let indices = inner.indices.drain(..).collect::<Vec<_>>();

        let graphics_inner = inner.pass.graphics.borrow_mut();

        let swapchain_size = {
            let renderpass_inner = inner.pass.inner.borrow_mut();

            Vector2::new(
                renderpass_inner.render_target_size.x as f32,
                renderpass_inner.render_target_size.y as f32,
            )
        };

        for vertex in vertices.iter_mut() {
            vertex.position.x = vertex.position.x / swapchain_size.x * 2.0 - 1.0;
            vertex.position.y = 1.0 - (vertex.position.y / swapchain_size.y * 2.0);
            vertex.color = vertex.color.into_srgb();
        }

        for queue in queues.iter_mut() {
            if queue.texture.is_none() {
                let default_texture = graphics_inner.drawing_default_texture.clone().unwrap();

                let inner = default_texture.inner.borrow();

                let sampler = inner.sampler_info.clone();
                let blend = inner.blend.clone();

                drop(inner);

                queue.texture = Some((default_texture, blend, sampler));
            }

            if queue.shader.is_none() {
                let default_shader = graphics_inner.drawing_default_shader.clone();

                queue.shader = default_shader;
            }
        }

        let mut vertex_buffer = graphics_inner
            .drawing_vertex_buffer
            .as_ref()
            .unwrap()
            .clone();

        let mut index_buffer = graphics_inner
            .drawing_index_buffer
            .as_ref()
            .unwrap()
            .clone();

        drop(graphics_inner);

        // Resize vertex and index buffers if needed
        if vertex_buffer.size() < (vertices.len() * std::mem::size_of::<Vertex>()) as u64 {
            let new_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;

            vertex_buffer
                .resize(new_size)
                .expect("Failed to resize vertex buffer");
        }

        if index_buffer.size() < (indices.len() * std::mem::size_of::<u16>()) as u64 {
            let new_size = (indices.len() * std::mem::size_of::<u16>()) as u64;

            index_buffer
                .resize(new_size)
                .expect("Failed to resize index buffer");
        }

        {
            let inner = inner.pass.inner.borrow();
            let mut cmd = inner.cmd.borrow_mut();

            vertex_buffer.internal_write_raw_cmd_ref(&vertices, &mut cmd);
            index_buffer.internal_write_raw_cmd_ref(&indices, &mut cmd);
        }

        for queue in queues {
            inner.pass.set_scissor(queue.scissors);
            inner.pass.set_viewport(queue.viewport, 0.0, 1.0);

            let (texture, blend, sampler) = queue.texture.as_ref().unwrap();

            inner.pass.set_shader(queue.shader.as_ref());
            inner
                .pass
                .set_gpu_buffer(Some(&vertex_buffer), Some(&index_buffer));

            inner.pass.set_blend(Some(&blend));

            inner.pass.set_attachment_texture(0, 0, Some(&texture));
            inner.pass.set_attachment_sampler(0, 1, Some(sampler));

            inner
                .pass
                .draw_indexed(queue.start_index..queue.count, queue.start_vertex as i32, 1);
        }

        inner.vertices.clear();
        inner.indices.clear();
    }
}

impl Drop for DrawingContext {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }

        self.end();
    }
}

thread_local! {
    static INDICES_VEC: RefCell<Vec<u16>> = RefCell::new(Vec::new());
}

/// Helper to convert triangle fan to tringle list indices,
/// since the wgpu does not support triangle fans directly.
fn triangle_fan_to_list_indices_ref(param: &mut Vec<u16>) {
    if param.len() < 3 {
        return;
    }

    INDICES_VEC.with(|vec| {
        let mut vec = vec.borrow_mut();

        vec_clear(&mut vec);
        vec.resize((param.len() - 2) * 3, 0);

        for i in 1..(param.len() - 1) {
            vec[(i - 1) * 3] = param[0];
            vec[(i - 1) * 3 + 1] = param[i];
            vec[(i - 1) * 3 + 2] = param[i + 1];
        }

        vec_clear(param);
        param.extend_from_slice(&vec);
    });
}

fn vec_clear<T>(vec: &mut Vec<T>) {
    // SAFETY: Only used for clearing the vector of plain struct
    // no references or pointers to heap-allocated data
    unsafe {
        assert!(
            std::mem::needs_drop::<T>() == false,
            "Cannot clear vector of type that needs drop"
        );

        vec.set_len(0);
    }
}
