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
    pub(crate) fn new(info: &Font, gpu: &mut GPU) -> Result<Self, String> {
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

        unimplemented!("GPUFont::new is not fully implemented yet");
    }
}
