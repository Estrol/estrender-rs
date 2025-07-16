//! Drawing, an intermediate mode drawing for some 2D primitives.

use std::{cell::RefCell, collections::HashMap};
use super::RenderPass;

use crate::{
    font::{Font, FontManager}, math::{Color, Point2, RectF, Vector2, Vector3, Vertex}, utils::ArcRef
};

use super::{
    super::{
        GPUInner,
        texture::{
            atlas::TextureAtlas,
            Texture, 
            TextureBuilder, 
            TextureUsage, 
            TextureSampler,
            TextureFormat
        },
        shader::{GraphicsShader, GraphicsShaderBuilder},
    },
};

#[derive(Clone, Debug)]
pub(crate) struct DrawingGlobalState {
    pub texture: Texture,
    pub shader: GraphicsShader,
    pub font_manager: FontManager,
    pub font_textures: HashMap<String, Texture>,
}

impl DrawingGlobalState {
    pub fn new(gpu_inner: &ArcRef<GPUInner>) -> Option<Self> {
        let default_texture = TextureBuilder::new(ArcRef::clone(gpu_inner))
            .set_raw_image(&[255u8, 255, 255, 255], Point2::new(1, 1), TextureFormat::Bgra8Unorm)
            .set_usage(TextureUsage::Sampler)
            .build()
            .ok()?;

        let default_shader = GraphicsShaderBuilder::new(ArcRef::clone(gpu_inner))
            .set_source(include_str!("./resources/drawing_shader.wgsl"))
            .build()
            .ok()?;

        let font_manager = FontManager::new();

        Some(Self {
            texture: default_texture,
            shader: default_shader,
            font_manager,
            font_textures: HashMap::new(),
        })
    }
}

pub(crate) struct DrawingContextInner {
    pass: RenderPass,
    drawing_global_state: ArcRef<DrawingGlobalState>,

    vertices: Vec<Vertex>,
    indices: Vec<u16>,

    texture: Option<(Texture, TextureSampler)>,
    texture_uv: Option<RectF>,
    texture_atlas_uv: Option<RectF>,
    shader: Option<GraphicsShader>,
    scissor: Option<RectF>,
    viewport: Option<RectF>,
    current_queue: Option<DrawingQueue>,
    queue: Vec<DrawingQueue>,

    current_font: Option<Font>,
    current_font_texture: Option<Texture>,
}

impl DrawingContextInner {
    pub fn get_absolute_uv(&self) -> RectF {
        fn remap_uv(rect1: RectF, rect2: RectF) -> RectF {
            RectF {
                x: rect2.x + (rect2.w - rect2.x) * rect1.x,
                y: rect2.y + (rect2.h - rect2.y) * rect1.y,
                w: rect2.x + (rect2.w - rect2.x) * rect1.w,
                h: rect2.y + (rect2.h - rect2.y) * rect1.h,
            }
        }

        fn resolve_uv(uv1: Option<RectF>, uv2: Option<RectF>) -> RectF {
            match (uv1, uv2) {
                (Some(r1), Some(r2)) => remap_uv(r1, r2),
                (Some(r1), None) => r1,
                (None, Some(r2)) => r2,
                (None, None) => RectF::new(0.0, 0.0, 1.0, 1.0),
            }
        }

        resolve_uv(self.texture_uv.clone(), self.texture_atlas_uv.clone())
    }

    pub fn push_geometry(
        &mut self,
        vertices: &[Vertex],
        indices: &[u16],
        has_image: bool,
    ) {
        if vertices.is_empty() || indices.is_empty() {
            return;
        }

        let base_index = self.vertices.len() as u16;
        let indices: Vec<u16> = indices.iter().map(|i| i + base_index).collect();
        self.push_queue(indices.len() as u32, has_image);

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            for vertex in vertices {
                if vertex.position.z != 0.0 {
                    panic!("DrawingContext only supports 2D rendering with z = 0.0");
                }

                if vertex.texcoord.x < 0.0
                    || vertex.texcoord.y < 0.0
                    || vertex.texcoord.x > 1.0
                    || vertex.texcoord.y > 1.0
                {
                    panic!("Texture coordinates must be in the range [0, 1]");
                }
            }
        }

        self.vertices.extend_from_slice(&vertices);
        self.indices.extend_from_slice(&indices);
    }

    pub fn push_queue(
        &mut self,
        count: u32,
        has_image: bool,
    ) {
        let mut push_new_queue = false;

        if self.current_queue.is_some() {
            let ref_queue = self.current_queue.as_ref().unwrap();

            // Check if current queue has the same texture, if not push the queue
            let current_texture = if has_image { &self.texture } else { &None };

            let texture_changed = match (&ref_queue.texture, current_texture) {
                (None, None) => false,
                (Some(_), None) | (None, Some(_)) => true,
                (
                    Some((old_texture, old_sampler)),
                    Some((new_texture, new_sampler)),
                ) => {
                    old_texture != new_texture
                        || old_sampler != new_sampler
                }
            };
            
            if texture_changed {
                push_new_queue = true;
            }

            let blend_states_changed = {
                let renderpass_inner = self.pass.inner.borrow();
                let ref_queue_blend_states = &ref_queue.blend_states;

                if renderpass_inner.render_targets.len() != ref_queue_blend_states.len() {
                   panic!("Render targets count mismatch: expected {}, got {}", renderpass_inner.render_targets.len(), ref_queue_blend_states.len());
                } else {
                    renderpass_inner.render_targets.iter().enumerate().any(|(i, render_target)| {
                        let (state, color_write) = &ref_queue_blend_states[i];
                        render_target.blend != *state || render_target.write_mask != *color_write
                    })
                }
            };

            if blend_states_changed {
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

            // check if current queue has the same shader, if not push the queue
            if ref_queue.shader != self.shader {
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

            let blend_state = {
                let renderpass_inner = self.pass.inner.borrow();
                renderpass_inner.render_targets.iter()
                    .map(|rt| (rt.blend.clone(), rt.write_mask))
                    .collect::<Vec<_>>()
            };

            self.current_queue = Some(DrawingQueue {
                texture: self.texture.clone(),
                shader: None,
                scissors: self.scissor.clone(),
                viewport: self.viewport.clone(),
                start_index: self.indices.len() as u32,
                start_vertex: 0, // TODO: Fix this
                count,
                blend_states: blend_state,
            });
        } else {
            let queue = self.current_queue.as_mut().unwrap();
            queue.count += count;
        }
    }

    pub fn load_font(&mut self, font_path: &str, range: Option<&[(u32, u32)]>, size: f32) {
        let mut state = self.drawing_global_state.borrow_mut();
        if let Ok(font) = state.font_manager.load_font(font_path, range, size) {
            if !state.font_textures.contains_key(font_path) {
                let texture = font.create_texture_inner(&self.pass.graphics)
                    .expect("Failed to create font texture");

                state.font_textures.insert(font_path.to_string(), texture);
            }

            self.current_font = Some(font);
            self.current_font_texture = state.font_textures.get(font_path).cloned();
        } else {
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            {
                crate::dbg_log!("Failed to load font: {}", font_path);
            }
        }
    }

    pub fn set_font(&mut self, font: &Font) {
        let name = {
            let font_inner = font.inner.borrow();
            font_inner.info.path.clone().into_os_string().into_string()
                .ok()
        };

        if name.is_none() {
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            {
                crate::dbg_log!("Font path is None, cannot set font");
            }
            return;
        }

        let name = name.unwrap();

        let mut state = self.drawing_global_state.borrow_mut();
        if !state.font_textures.contains_key(&name) {
            let texture = font.create_texture_inner(&self.pass.graphics)
                .expect("Failed to create font texture");

            state.font_textures.insert(name.to_string(), texture);
        }

        self.current_font = Some(font.clone());
        self.current_font_texture = state.font_textures.get(&name).cloned();
    }
}

pub(crate) struct DrawingQueue {
    pub texture: Option<(Texture, TextureSampler)>,
    pub shader: Option<GraphicsShader>,

    pub scissors: Option<RectF>,
    pub viewport: Option<RectF>,

    pub start_index: u32,
    pub start_vertex: u32,
    pub count: u32,

    pub blend_states: Vec<(Option<wgpu::BlendState>, Option<wgpu::ColorWrites>)>,
}

/// DrawingContext is an intermediate mode for drawing 2D primitives.
///
/// It provides methods to draw rectangles, lines, triangles, circles, and images with various options for colors and textures.
pub struct DrawingContext {
    pub(crate) inner: ArcRef<DrawingContextInner>,

    pub(crate) vertex_cache: Vec<Vertex>,
    pub(crate) index_cache: Vec<u16>,
}

impl DrawingContext {
    pub(crate) fn new(pass: RenderPass) -> Option<Self> {
        if pass.graphics.borrow().drawing_state.is_none() {
            let state = DrawingGlobalState::new(&pass.graphics)?;

            let mut gpu_inner = pass.graphics.borrow_mut();
            gpu_inner.drawing_state = Some(ArcRef::new(state));
        }

        let drawing_state = ArcRef::clone(
            &pass.graphics.borrow().drawing_state.as_ref().unwrap()
        );

        let inner = DrawingContextInner {
            pass: pass,
            drawing_global_state: drawing_state,

            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
            texture_uv: None,
            texture_atlas_uv: None,
            shader: None,
            scissor: None,
            viewport: None,
            current_queue: None,
            queue: Vec::new(),
            
            current_font: None,
            current_font_texture: None,
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

    /// Load a font from the specified path with an optional range of codepoints and size.
    pub fn load_font(&mut self, font_path: &str, range: Option<&[(u32, u32)]>, size: f32) {
        let mut inner = self.inner.borrow_mut();
        inner.load_font(font_path, range, size);
    }

    /// Set the current font to be used for drawing text.
    pub fn set_font(&mut self, font: &Font) {
        let mut inner = self.inner.borrow_mut();
        inner.set_font(font);
    }

    /// Get the current font, loading it if it hasn't been set yet.
    pub fn get_font(&self) -> Font {
        let mut inner = self.inner.borrow_mut();
        if inner.current_font.is_none() {
            inner.load_font("Arial", None, 16.0);
        }

        inner.current_font.clone().unwrap_or_else(|| {
            panic!("Fatal: No font loaded, this shouldn't happen as default Font is loaded on demand.");
        })
    }

    /// Draw text with a specified position, color, and font.
    pub fn draw_text(&mut self, text: &str, pos: Vector2, color: Color) {
        let mut inner = self.inner.borrow_mut();
        if inner.current_font.is_none() {
            inner.load_font("Arial", None, 16.0);
        }

        vec_clear(&mut self.vertex_cache);
        vec_clear(&mut self.index_cache);

        let font = inner.current_font.as_ref().unwrap();
        let texture_size = font.texture_size();
        let line_height = font.line_height();
        let ascender = font.ascender();
        let space_width = font.space_width();

        // Calculate the minimum Y offset for the text
        let mut pen_y = 0.0;
        let mut min_y = f32::MAX;
        for c in text.chars() {
            let codepoint = c as u32;
            if codepoint == 0 {
                continue;
            }

            if codepoint == '\n' as u32 {
                pen_y += line_height;
                continue;
            }

            if let Ok(glyph) = font.get_glyph(codepoint) {
                min_y = f32::min(min_y, pen_y + ascender - (glyph.bearing_y + glyph.height));
            }
        }

        let mut pen = pos;
        for c in text.chars() {
            let codepoint = c as u32;
            if codepoint == 0 {
                continue;
            }

            if codepoint == '\n' as u32 {
                pen.x = pos.x;
                pen.y += line_height;
                continue;
            }

            if codepoint == ' ' as u32 {
                pen.x += space_width;
                continue;
            }

            if let Ok(glyph) = font.get_glyph(codepoint) {
                let x0 = pen.x + glyph.bearing_x;
                let y0 = pen.y + ascender - (glyph.bearing_y + glyph.height) - min_y;
                let x1 = x0 + glyph.width;
                let y1 = y0 + glyph.height;

                let uv_x0 = glyph.atlas_start_offset.x as f32 / texture_size.x as f32;
                let uv_y0 = glyph.atlas_start_offset.y as f32 / texture_size.y as f32;
                let uv_x1 = (glyph.atlas_start_offset.x + glyph.width) as f32 / texture_size.x as f32;
                let uv_y1 = (glyph.atlas_start_offset.y + glyph.height) as f32 / texture_size.y as f32;

                let vertices = [
                    Vertex::new(Vector3::new(x0, y0, 0.0), color, Vector2::new(uv_x0, uv_y0)),
                    Vertex::new(Vector3::new(x1, y0, 0.0), color, Vector2::new(uv_x1, uv_y0)),
                    Vertex::new(Vector3::new(x1, y1, 0.0), color, Vector2::new(uv_x1, uv_y1)),
                    Vertex::new(Vector3::new(x0, y1, 0.0), color, Vector2::new(uv_x0, uv_y1)),
                ];

                let base_index = self.vertex_cache.len() as u16;
                let indices = [
                    base_index + 0,
                    base_index + 1,
                    base_index + 2,
                    base_index + 0,
                    base_index + 2,
                    base_index + 3,
                ];

                self.vertex_cache.extend_from_slice(&vertices);
                self.index_cache.extend_from_slice(&indices);

                pen.x += glyph.advance_x;
            }
        }

        if self.index_cache.is_empty() {
            return;
        }

        let all_vertices = &self.vertex_cache;
        let all_indices = &self.index_cache;

        let current_texture = inner.texture.clone();
        let font_texture = inner.current_font_texture.clone();
        inner.texture = Some((
            font_texture.unwrap(),
            TextureSampler::DEFAULT,
        ));

        inner.push_geometry(&all_vertices, &all_indices, true);

        inner.texture = current_texture;
    }

    /// Draw hollow rectangle with a specified position, size, thickness, and color.
    pub fn draw_rect(&mut self, pos: Vector2, size: Vector2, thickness: f32, color: Color) {
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
            let vertices = vertices
                .iter()
                .map(|v| {
                    Vertex::new(
                        Vector3::new(v.x, v.y, 0.0),
                        color,
                        Vector2::ZERO,
                    )
                })
                .collect::<Vec<_>>();

            indices.iter_mut().for_each(|idx| *idx += index_offset);
            index_offset += vertices.len() as u16;

            all_vertices.extend(vertices);
            all_indices.extend(indices);
        }

        self.inner.borrow_mut()
            .push_geometry(&all_vertices, &all_indices, false);
    }

    /// Draw line between two points with a specified thickness and color.
    pub fn draw_line(&mut self, a: Vector2, b: Vector2, thickness: f32, color: Color) {
        let line = Self::construct_line(a, b, thickness);
        if line.is_none() {
            return;
        }

        let (vertices, indices) = line.unwrap();
        let vertices = vertices
            .iter()
            .map(|v| {
                Vertex::new(
                    Vector3::new(v.x, v.y, 0.0),
                    color,
                    Vector2::ZERO,
                )
            })
            .collect::<Vec<_>>();

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    /// Draw rectangle filled with a specified position, size, and color.
    pub fn draw_rect_filled(&mut self, pos: Vector2, size: Vector2, color: Color) {
        let vertices = [
            Vertex::new(
                Vector3::new(pos.x, pos.y, 0.0),
                color,
                Vector2::ZERO,
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y, 0.0),
                color,
                Vector2::ZERO,
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y + size.y, 0.0),
                color,
                Vector2::ZERO,
            ),
            Vertex::new(
                Vector3::new(pos.x, pos.y + size.y, 0.0),
                color,
                Vector2::ZERO,
            ),
        ];

        let indices = [0, 1, 2, 0, 2, 3];

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    /// Draw rectangle filled with specified colors for each corner.
    pub fn draw_rect_filled_colors(
        &mut self,
        pos: Vector2,
        size: Vector2,
        color_tl: Color,
        color_tr: Color,
        color_br: Color,
        color_bl: Color,
    ) {
        let vertices = [
            Vertex::new(
                Vector3::new(pos.x, pos.y, 0.0),
                color_tl.into_srgb(),
                Vector2::ZERO,
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y, 0.0),
                color_tr.into_srgb(),
                Vector2::ZERO,
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y + size.y, 0.0),
                color_br.into_srgb(),
                Vector2::ZERO,
            ),
            Vertex::new(
                Vector3::new(pos.x, pos.y + size.y, 0.0),
                color_bl.into_srgb(),
                Vector2::ZERO,
            ),
        ];

        let indices = [
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    /// Draw triangle with specified vertices, thickness, and color.
    pub fn draw_triangle(
        &mut self,
        a: Vector2,
        b: Vector2,
        c: Vector2,
        thickness: f32,
        color: Color,
    ) {
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
            let vertices = vertices
                .iter()
                .map(|v| Vertex::new(Vector3::new(v.x, v.y, 0.0), color, Vector2::ZERO))
                .collect::<Vec<_>>();

            indices.iter_mut().for_each(|idx| *idx += index_offset);
            index_offset += vertices.len() as u16;

            all_vertices.extend(vertices);
            all_indices.extend(indices);
        }

        if all_indices.is_empty() {
            return;
        }

        self.inner.borrow_mut()
            .push_geometry(&all_vertices, &all_indices, false);
    }

    /// Draw triangle filled with specified vertices and color.
    pub fn draw_triangle_filled(&mut self, a: Vector2, b: Vector2, c: Vector2, color: Color) {
        let vertices = [
            Vertex::new(Vector3::new(a.x, a.y, 0.0), color, Vector2::ZERO),
            Vertex::new(Vector3::new(b.x, b.y, 0.0), color, Vector2::ZERO),
            Vertex::new(Vector3::new(c.x, c.y, 0.0), color, Vector2::ZERO),
        ];

        let indices = [0, 1, 2];

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    pub fn draw_triangle_filled_colors(
        &mut self,
        a: Vector2,
        b: Vector2,
        c: Vector2,
        color_a: Color,
        color_b: Color,
        color_c: Color,
    ) {
        let vertices = [
            Vertex::new(Vector3::new(a.x, a.y, 0.0), color_a, Vector2::ZERO),
            Vertex::new(Vector3::new(b.x, b.y, 0.0), color_b, Vector2::ZERO),
            Vertex::new(Vector3::new(c.x, c.y, 0.0), color_c, Vector2::ZERO),
        ];

        let indices = [0, 1, 2];

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    /// Draw circle with a specified center, radius, number of segments, thickness, and color.
    pub fn draw_circle(
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
            let line_vertices: Vec<Vertex> = line_vertices
                .iter()
                .map(|v| Vertex::new(Vector3::new(v.x, v.y, 0.0), color, Vector2::ZERO))
                .collect();

            vertices.extend(line_vertices);
            indices.extend(line_indices.into_iter().map(|i| i + base_index));
        }

        if indices.is_empty() {
            return;
        }

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    pub fn draw_circle_filled(
        &mut self,
        center: Vector2,
        radius: f32,
        segments: u32,
        color: Color,
    ) {
        if segments < 3 {
            return;
        }

        let angle_step = std::f32::consts::PI * 2.0 / segments as f32;
        let vertices = &mut self.vertex_cache;
        let indices = &mut self.index_cache;

        vec_clear(vertices);
        vec_clear(indices);

        vertices.push(Vertex::new(
            Vector3::new(center.x, center.y, 0.0),
            color,
            Vector2::ZERO,
        ));

        for i in 0..segments {
            let angle = angle_step * i as f32;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();

            vertices.push(Vertex::new(Vector3::new(x, y, 0.0), color, Vector2::ZERO));
            indices.push(i as u16 + 1);
        }

        triangle_fan_to_list_indices_ref(&mut *indices);

        if indices.is_empty() {
            return;
        }

        self.inner.borrow_mut()
            .push_geometry(&vertices, &indices, false);
    }

    pub fn draw_rect_image(&mut self, pos: Vector2, size: Vector2, color: Color) {
        let mut inner = self.inner.borrow_mut();
        let uv: RectF = inner.get_absolute_uv();

        let vertices = [
            Vertex::new(
                Vector3::new(pos.x, pos.y, 0.0),
                color,
                Vector2::new(uv.x, uv.y),
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y, 0.0),
                color,
                Vector2::new(uv.w, uv.y),
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y + size.y, 0.0),
                color,
                Vector2::new(uv.w, uv.h),
            ),
            Vertex::new(
                Vector3::new(pos.x, pos.y + size.y, 0.0),
                color,
                Vector2::new(uv.x, uv.h),
            ),
        ];

        let indices = [0, 1, 2, 0, 2, 3];
        inner.push_geometry(&vertices, &indices, true);
    }

    pub fn draw_rect_image_colors(
        &mut self,
        pos: Vector2,
        size: Vector2,
        color_tl: Color,
        color_tr: Color,
        color_br: Color,
        color_bl: Color,
    ) {
        let mut inner = self.inner.borrow_mut();
        let uv = inner.get_absolute_uv();

        let vertices = [
            Vertex::new(
                Vector3::new(pos.x, pos.y, 0.0),
                color_tl,
                Vector2::new(uv.x, uv.y),
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y, 0.0),
                color_tr,
                Vector2::new(uv.w, uv.y),
            ),
            Vertex::new(
                Vector3::new(pos.x + size.x, pos.y + size.y, 0.0),
                color_br,
                Vector2::new(uv.w, uv.h),
            ),
            Vertex::new(
                Vector3::new(pos.x, pos.y + size.y, 0.0),
                color_bl,
                Vector2::new(uv.x, uv.h),
            ),
        ];

        let indices = [0, 1, 2, 0, 2, 3];
        inner.push_geometry(&vertices, &indices, true);
    }

    pub fn draw_triangle_image(&mut self, a: Vector2, b: Vector2, c: Vector2, color: Color) {
        let mut inner = self.inner.borrow_mut();
        let uv = inner.get_absolute_uv();

        let vertices = [
            Vertex::new(Vector3::new(a.x, a.y, 0.0), color, Vector2::new(uv.x, uv.y)),
            Vertex::new(Vector3::new(b.x, b.y, 0.0), color, Vector2::new(uv.w, uv.y)),
            Vertex::new(
                Vector3::new(c.x, c.y, 0.0),
                color,
                Vector2::new(uv.w * 0.5, uv.h),
            ),
        ];

        let indices = [0, 1, 2];
        inner.push_geometry(&vertices, &indices, true);
    }

    pub fn draw_triangle_image_colors(
        &mut self,
        a: Vector2,
        b: Vector2,
        c: Vector2,
        color_a: Color,
        color_b: Color,
        color_c: Color,
    ) {
        let mut inner = self.inner.borrow_mut();
        let uv = inner.get_absolute_uv();

        let vertices = [
            Vertex::new(Vector3::new(a.x, a.y, 0.0), color_a, Vector2::new(uv.x, uv.y)),
            Vertex::new(Vector3::new(b.x, b.y, 0.0), color_b, Vector2::new(uv.w, uv.y)),
            Vertex::new(Vector3::new(c.x, c.y, 0.0), color_c, Vector2::new(uv.w * 0.5, uv.h)),
        ];

        let indices = [0, 1, 2];
        inner.push_geometry(&vertices, &indices, true);
    }

    pub fn draw_circle_image(&mut self, center: Vector2, radius: f32, segments: u32, color: Color) {
        if segments < 3 {
            return;
        }

        let mut inner = self.inner.borrow_mut();
        let uv = inner.get_absolute_uv();

        let angle_step = std::f32::consts::PI * 2.0 / segments as f32;
        let mut vertices = Vec::with_capacity(segments as usize);
        let mut indices = Vec::with_capacity(segments as usize * 3);

        for i in 0..segments {
            let angle = angle_step * i as f32;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();

            let u = uv.x + (uv.w - uv.x) * (angle.cos() * 0.5 + 0.5);
            let v = uv.y + (uv.h - uv.y) * (angle.sin() * 0.5 + 0.5);

            vertices.push(Vertex::new(
                Vector3::new(x, y, 0.0),
                color,
                Vector2::new(u, v),
            ));

            indices.push(i as u16);
        }

        triangle_fan_to_list_indices_ref(&mut indices);

        if indices.is_empty() {
            return;
        }

        inner.push_geometry(&vertices, &indices, true);
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
        self.set_texture_ex(texture, None);
    }

    pub fn set_texture_ex(
        &mut self,
        texture: Option<&Texture>,
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

                let default_sampler = TextureSampler::DEFAULT;
                let sampler = sampler.unwrap_or(default_sampler);

                inner.texture = Some((texture.clone(), sampler));
            }
            None => {
                inner.texture = None;
            }
        }
    }

    pub fn set_texture_uv(&mut self, texture_uv: Option<RectF>) {
        let mut inner = self.inner.borrow_mut();

        match texture_uv {
            Some(uv) => {
                inner.texture_uv = Some(uv);
            }
            None => {
                inner.texture_uv = None;
            }
        }
    }

    pub fn set_texture_atlas(&mut self, atlas: Option<(&TextureAtlas, &str)>) {
        self.set_texture_atlas_ex(atlas);
    }

    pub fn set_texture_atlas_ex(
        &mut self,
        atlas: Option<(&TextureAtlas, &str)>,
    ) {
        match atlas {
            Some((atlas, id)) => {
                let tex_coord = atlas.get_id(id);

                #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                if tex_coord.is_none() {
                    panic!("Texture atlas does not contain the specified id: {}", id);
                }

                let (tex_coord, _) = tex_coord.unwrap();
                let texture = atlas.get_texture();

                let mut inner = self.inner.borrow_mut();

                let default_sampler = TextureSampler::DEFAULT;
                inner.texture_atlas_uv = Some(tex_coord);
                inner.texture = Some((
                    texture.clone(),
                    default_sampler,
                ));
            }
            None => {
                self.inner.borrow_mut().texture_atlas_uv = None;
                return;
            }
        };
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
                        use super::super::shader::types::{ShaderReflect, ShaderBindingType};

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

    pub(crate) fn end(&mut self) {
        let mut inner = self.inner.borrow_mut();

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if inner.vertices.is_empty() {
            crate::dbg_log!(
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

        {
            let graphics_inner = inner.pass.graphics.borrow();
            let drawing = graphics_inner.drawing_state.as_ref().unwrap().borrow();
            
            let swapchain_size = {
                let renderpass_inner = inner.pass.inner.borrow_mut();

                Vector2::new(
                    renderpass_inner.surface_size.x as f32,
                    renderpass_inner.surface_size.y as f32,
                )
            };

            for vertex in vertices.iter_mut() {
                vertex.position.x = vertex.position.x / swapchain_size.x * 2.0 - 1.0;
                vertex.position.y = 1.0 - (vertex.position.y / swapchain_size.y * 2.0);
            }

            for queue in queues.iter_mut() {
                if queue.texture.is_none() {
                    let default_texture = drawing
                        .texture
                        .clone();

                    let sampler = TextureSampler::DEFAULT;
                    queue.texture = Some((default_texture, sampler));
                }

                if queue.shader.is_none() {
                    let default_shader = drawing
                        .shader
                        .clone();

                    queue.shader = Some(default_shader);
                }
            }
        };

        let (vertex_buffer, index_buffer) = {
            let mut graphics_inner = inner.pass.graphics.borrow_mut();
            
            let vertex_buffer = graphics_inner
                .create_staging_buffer(bytemuck::cast_slice(&vertices), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);

            let index_buffer = graphics_inner
                .create_staging_buffer(bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST);

            (vertex_buffer, index_buffer)
        };

        for queue in queues {
            let pass = &mut inner.pass;

            if let Some(mut pass_inner) = pass.inner.try_borrow_mut() {
                for (i, blend_state) in queue.blend_states.iter().enumerate() {
                    if i >= pass_inner.render_targets.len() {
                        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                        {
                            crate::dbg_log!("DrawingContext::end: Blend state index {} out of bounds for render_targets", i);
                        }
                        continue;
                    }

                    let attachment = &mut pass_inner.render_targets[i];
                    attachment.blend = blend_state.0.clone();
                    attachment.write_mask = blend_state.1.clone();
                }
            }

            pass.set_scissor(queue.scissors);
            pass.set_viewport(queue.viewport, 0.0, 1.0);

            let (texture, sampler) = queue.texture.as_ref().unwrap();

            pass.set_shader(queue.shader.as_ref());
            pass
                .set_gpu_buffer_wgpu(Some(vertex_buffer.clone()), Some(index_buffer.clone()));

            pass.set_attachment_texture(0, 0, Some(&texture));
            pass.set_attachment_sampler(0, 1, Some(sampler));

            pass
                .draw_indexed(queue.start_index..(queue.start_index + queue.count), queue.start_vertex as i32, 1);
        }
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

// Memory optimization purpose.
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

/// Quick and dirty way to clear a vector without dropping its elements.
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
