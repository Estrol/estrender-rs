use std::collections::HashMap;

use crate::{
    math::{Color, Rect, Vector2, Vertex},
    prelude::{Buffer, GPU, RenderPass, Texture, TextureFormat, TextureUsage},
    utils::ArcRef,
};

use super::{Font, Glyph};

#[allow(dead_code)]
pub(crate) struct GPUFontInner {
    pub texture: Texture,
    pub glyphs: HashMap<u32, Glyph>,

    pub index_buffer: Option<Buffer>,
    pub vertex_buffer: Option<Buffer>,
}

pub struct GPUFont {
    pub(crate) inner: ArcRef<GPUFontInner>,
}

impl GPUFont {
    pub(crate) fn new(info: &Font, gpu: &GPU) -> Result<Self, String> {
        let (texture_data, width, height) = info.get_image_data();

        let mut texture_data_rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for i in 0..(width as usize * height as usize) {
            texture_data_rgba.push(texture_data[i]); // R
            texture_data_rgba.push(texture_data[i]); // G
            texture_data_rgba.push(texture_data[i]); // B
            texture_data_rgba.push(255); // A
        }

        let texture = gpu
            .create_texture()
            .with_raw(
                &texture_data_rgba,
                Rect::new(0, 0, width as i32, height as i32),
                TextureFormat::Bgra8Unorm,
            )
            .with_usage(TextureUsage::Sampler)
            .build();

        if texture.is_err() {
            return Err("Failed to create texture".to_string());
        }

        let texture = texture.unwrap();

        let info_inner = info.inner.borrow();

        Ok(GPUFont {
            inner: ArcRef::new(GPUFontInner {
                texture,
                glyphs: info_inner.glyphs.clone(),
                index_buffer: None,
                vertex_buffer: None,
            }),
        })
    }

    pub fn bind_texture(&self, graphics_pass: &mut RenderPass, group: u32, binding: u32) {
        let inner = self.inner.borrow();

        // graphics_pass.set_attachment_texture(group, binding, Some(&inner.texture));
    }

    pub fn bind_sampler(&self, graphics_pass: &mut RenderPass, group: u32, binding: u32) {
        let inner = self.inner.borrow();

        let texture_inner = inner.texture.inner.borrow();

        // graphics_pass.set_attachment_sampler(group, binding, Some(&texture_inner.sampler_info));
    }

    /// Draw directly to GPU's Graphics Pass
    ///
    /// You need set the group and binding according to your shader layout with
    /// [GPUFont::bind_texture] and [GPUFont::bind_sampler] before calling this function.
    ///
    /// If you incorrectly set the group and binding, it may cause a panic or undefined behavior.
    /// You also need to set the pipeline and bind group before calling this function.
    ///
    /// This functio internally use own vertex buffer and index buffer, so you may need to reset your own
    /// vertex buffer and index buffer after calling this function.
    pub fn draw_gpu_cmd(
        &self,
        _graphics_pass: &mut RenderPass,
        position: Vector2,
        color: Color,
        text: &str,
    ) {
        let inner = self.inner.borrow();
        let texture_inner = inner.texture.inner.borrow();

        let texture_size = texture_inner.size;

        let chars = text.chars();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let mut pen_position = position.clone();

        for c in chars {
            let glyph = {
                // if glyph is not found, use space instead
                let codepoint = c as u32;

                if codepoint == '\n' as u32 {
                    pen_position.x = position.x;
                    pen_position.y -= texture_inner.size.y as f32;
                    continue;
                }

                if let Some(glyph) = inner.glyphs.get(&codepoint) {
                    glyph
                } else {
                    continue;
                    // inner.glyphs.get(&(b' ' as u32)).unwrap()
                }
            };

            let start_uv_x = glyph.atlas_start_offset.x / texture_size.w as f32;
            let start_uv_y = glyph.atlas_start_offset.y / texture_size.h as f32;
            let end_uv_x = glyph.atlas_end_offset.x / texture_size.w as f32;
            let end_uv_y = glyph.atlas_end_offset.y / texture_size.h as f32;

            let start_x = pen_position.x + glyph.bearing_x;
            let start_y = pen_position.y - glyph.bearing_y;
            let end_x = start_x + glyph.width;
            let end_y = start_y + glyph.height;

            let vertex_list = [
                Vertex::new_slice(
                    [start_x, start_y, 0.0],
                    [color.r, color.g, color.b, color.a],
                    [start_uv_x, start_uv_y],
                ),
                Vertex::new_slice(
                    [end_x, start_y, 0.0],
                    [color.r, color.g, color.b, color.a],
                    [end_uv_x, start_uv_y],
                ),
                Vertex::new_slice(
                    [end_x, end_y, 0.0],
                    [color.r, color.g, color.b, color.a],
                    [end_uv_x, end_uv_y],
                ),
                Vertex::new_slice(
                    [start_x, end_y, 0.0],
                    [color.r, color.g, color.b, color.a],
                    [start_uv_x, end_uv_y],
                ),
            ];

            let index_list = [0, 1, 2, 0, 2, 3];

            let start_index = vertices.len() as u32;
            vertices.extend_from_slice(&vertex_list);
            indices.extend_from_slice(&index_list.map(|i| i + start_index));

            pen_position.x += glyph.advance_x;
        }

        // if let Some(index_buffer) = inner.index_buffer.clone() {
        //     if index_buffer.size < (indices.len() * size_of::<u16>() as u64) {

        //     }
        // }

        // let (vertex, index) = {
        //     let mut gpu_inner = graphics_pass.graphics.borrow_mut();

        //     let vertex_u8: &[u8] = bytemuck::cast_slice(&vertices);
        //     let index_u8: &[u8] = bytemuck::cast_slice(&indices);

        //     let vertex = gpu_inner.insert_buffer(vertex_u8, BufferUsages::VERTEX);
        //     let index = gpu_inner.insert_buffer(index_u8, BufferUsages::INDEX);

        //     (vertex, index)
        // };

        // graphics_pass.internal_set_gpu_buffer(Some(&vertex), Some(&index));
        // graphics_pass.draw_indexed(
        //     0,
        //     indices.len() as u32,
        //     0,
        // );
    }
}
