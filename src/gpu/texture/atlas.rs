use std::collections::HashMap;

use crate::{math::{Point2, RectF}, utils::ArcRef};

use super::{
    super::GPUInner,
    Texture,
    TextureError,
    TextureBuilder,
    TextureUsage,
    TextureFormat,
};

/// Represents a texture atlas containing multiple textures
/// and their UV coordinates
#[derive(Debug, Clone)]
pub struct TextureAtlas {
    pub(crate) texture: Texture,
    pub(crate) items: HashMap<String, TextureAtlasCoord>,
}

#[derive(Debug, Clone)]
pub(crate) struct TextureAtlasCoord {
    pub rect_uv: RectF,
    pub size: Point2,
}

impl TextureAtlas {
    pub(crate) fn new(texture: Texture, items: HashMap<String, TextureAtlasCoord>) -> Self {
        Self { texture, items }
    }

    /// Retrieves the UV rectangle and size for a given texture ID
    pub fn get_id(&self, id: &str) -> Option<(RectF, Point2)> {
        self.items.get(id).map(|coord| (coord.rect_uv, coord.size))
    }

    /// Get the texture associated with this atlas
    pub fn get_texture(&self) -> &Texture {
        &self.texture
    }

    /// Get the size of the texture atlas
    pub fn get_texture_size(&self) -> Point2 {
        let inner = self.texture.inner.borrow();

        Point2::new(inner.size.x as i32, inner.size.y as i32)
    }
}

const MAX_WIDTH_SIZE: i32 = 2048;

#[derive(Debug, Clone)]
pub struct TextureAtlasBuilder {
    pub(crate) gpu: ArcRef<GPUInner>,
    pub(crate) items: HashMap<String, ItemQueue>,
}

#[derive(Debug, Clone)]
pub(crate) enum ItemQueue {
    File(String),           // id, file path
    Memory(Vec<u8>),        // id, raw data
    Raw(Vec<u8>, u32, u32), // id, raw data, width, height
}

#[derive(Debug, Clone)]
pub enum TextureAtlasBuilderError {
    EmptyAtlas,
    ExceedsMaxSize(i32, i32),
    FileNotFound(String),
    InvalidData(String),
    TextureCreationError(TextureError),
}

impl std::fmt::Display for TextureAtlasBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureAtlasBuilderError::EmptyAtlas => write!(f, "Texture atlas cannot be empty"),
            TextureAtlasBuilderError::ExceedsMaxSize(width, height) => write!(
                f,
                "Texture atlas exceeds maximum size: {}x{}",
                width, height
            ),
            TextureAtlasBuilderError::FileNotFound(file) => {
                write!(f, "Texture file not found: {}", file)
            }
            TextureAtlasBuilderError::InvalidData(id) => {
                write!(f, "Invalid texture data for id: {}", id)
            }
            TextureAtlasBuilderError::TextureCreationError(err) => {
                write!(f, "Texture creation error: {}", err)
            }
        }
    }
}

impl TextureAtlasBuilder {
    pub(crate) fn new(gpu: ArcRef<GPUInner>) -> Self {
        Self {
            items: HashMap::new(),
            gpu,
        }
    }

    pub fn add_texture_file(mut self, id: &str, file: &str) -> Self {
        self.items
            .insert(id.to_string(), ItemQueue::File(file.to_string()));
        self
    }

    pub fn add_texture_file_buf(mut self, id: &str, data: &[u8]) -> Self {
        self.items
            .insert(id.to_string(), ItemQueue::Memory(data.to_vec()));
        self
    }

    pub fn add_texture_raw(mut self, id: &str, data: Vec<u8>, width: u32, height: u32) -> Self {
        self.items
            .insert(id.to_string(), ItemQueue::Raw(data, width, height));
        self
    }

    pub fn build(self) -> Result<TextureAtlas, TextureAtlasBuilderError> {
        if self.items.is_empty() {
            return Err(TextureAtlasBuilderError::EmptyAtlas);
        }

        let mut texture_items = HashMap::new();

        for (id, item) in self.items {
            use image::GenericImageView;

            let (texture_data, size) = match item {
                ItemQueue::File(file) => {
                    if !std::path::Path::new(&file).exists() {
                        return Err(TextureAtlasBuilderError::FileNotFound(file));
                    }

                    let canonical_path = std::fs::canonicalize(&file)
                        .map_err(|_| TextureAtlasBuilderError::FileNotFound(file.clone()))?;

                    let image = image::open(&canonical_path)
                        .map_err(|_| TextureAtlasBuilderError::InvalidData(file.clone()))?;

                    let (width, height) = image.dimensions();
                    let data = image.to_rgba8();

                    (data.to_vec(), Point2::new(width as i32, height as i32))
                }
                ItemQueue::Memory(data) => {
                    let image = image::load_from_memory(&data)
                        .map_err(|_| TextureAtlasBuilderError::InvalidData(id.clone()))?;

                    let (width, height) = image.dimensions();
                    let data = image.to_rgba8();

                    (data.to_vec(), Point2::new(width as i32, height as i32))
                }
                ItemQueue::Raw(data, width, height) => {
                    if data.len() != (width * height * 4) as usize {
                        return Err(TextureAtlasBuilderError::InvalidData(id.clone()));
                    }

                    let size = Point2::new(width as i32, height as i32);
                    (data, size)
                }
            };

            texture_items.insert(id.to_string(), (texture_data, size));
        }

        let rect_config = rect_packer::Config {
            width: MAX_WIDTH_SIZE as i32,
            height: MAX_WIDTH_SIZE as i32,
            border_padding: 1,
            rectangle_padding: 1,
        };

        let mut packer = rect_packer::Packer::new(rect_config);
        let mut placemenets = HashMap::new();
        let mut atlas_size = Point2::new(0, 0);

        for (id, (_, size)) in &texture_items {
            if size.x > MAX_WIDTH_SIZE || size.y > MAX_WIDTH_SIZE {
                return Err(TextureAtlasBuilderError::ExceedsMaxSize(
                    size.x,
                    size.y,
                ));
            }

            let rect = packer.pack(size.x, size.y, false)
                .ok_or_else(|| {
                TextureAtlasBuilderError::InvalidData(format!(
                    "Failed to pack texture with id: {}",
                    id
                ))
            })?;

            placemenets.insert(id.clone(), rect);
            atlas_size.x = atlas_size.x.max(rect.x + rect.width);
            atlas_size.y = atlas_size.y.max(rect.y + rect.height);
        }

        if atlas_size.x > MAX_WIDTH_SIZE || atlas_size.y > MAX_WIDTH_SIZE {
            return Err(TextureAtlasBuilderError::ExceedsMaxSize(atlas_size.x, atlas_size.y));
        }

        let mut texture_data = vec![0; (atlas_size.x * atlas_size.y * 4) as usize];
        let mut items = HashMap::new();
        for (id, rect) in placemenets {
            let (data, size) = texture_items.get(&id).ok_or_else(|| {
                TextureAtlasBuilderError::InvalidData(format!("Missing data for id: {}", id))
            })?;

            let atlas_w = atlas_size.x as f32;
            let atlas_h = atlas_size.y as f32;
            let half_texel_x = 0.5 / atlas_w;
            let half_texel_y = 0.5 / atlas_h;

            let rect_uv = RectF::new(
                (rect.x as f32 + half_texel_x) / atlas_w,
                (rect.y as f32 + half_texel_y) / atlas_h,
                (rect.x as f32 + rect.width as f32 - half_texel_x) / atlas_w,
                (rect.y as f32 + rect.height as f32 - half_texel_y) / atlas_h,
            );

            let size = Point2::new(size.x, size.y);

            for j in 0..size.y {
                for i in 0..size.x {
                    let src_index = ((j * size.x + i) * 4) as usize;
                    let dst_index = (((rect.y + j) * atlas_size.x + (rect.x + i)) * 4) as usize;

                    texture_data[dst_index..dst_index + 4]
                        .copy_from_slice(&data[src_index..src_index + 4]);
                }
            }

            items.insert(
                id,
                TextureAtlasCoord {
                    rect_uv,
                    size,
                },
            );
        }

        let format = if self.gpu.borrow().is_srgb() {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        };

        let texture = TextureBuilder::new(self.gpu)
            .set_raw_image(&texture_data, atlas_size, format)
            .set_usage(TextureUsage::Sampler)
            .build()
            .map_err(TextureAtlasBuilderError::TextureCreationError)?;

        Ok(TextureAtlas::new(texture, items))
    }
}