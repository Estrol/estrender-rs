#![allow(dead_code)]

use std::sync::atomic::AtomicUsize;

use wgpu::{Extent3d, TextureDescriptor};

use crate::{
    dbg_log,
    gpu::{BufferBuilder, buffer::BufferUsage, gpu_inner::GPUInner},
    math::Rect,
    utils::ArcRef,
};

mod types;
pub use types::*;

pub enum TextureBuilderData<'a> {
    None,
    File(&'a str),
    Data(&'a [u8]),
    Raw(Rect, &'a [u8], TextureFormat),
    DepthStencil(Rect, Option<TextureFormat>),
    RenderTarget(Rect, Option<TextureFormat>),
}

pub struct TextureBuilder<'a> {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) sample_count: SampleCount,
    pub(crate) mip_level_count: u32,
    pub(crate) usage: TextureUsage,
    pub(crate) data: TextureBuilderData<'a>,
}

impl<'a> TextureBuilder<'a> {
    pub(crate) fn new(graphics: ArcRef<GPUInner>) -> Self {
        if graphics.borrow().is_invalid {
            panic!("Graphics context is invalid");
        }

        Self {
            graphics,
            sample_count: SampleCount::SampleCount1,
            mip_level_count: 1,
            usage: TextureUsage::None,
            data: TextureBuilderData::None,
        }
    }

    pub fn with_file(mut self, file_path: &'a str) -> Self {
        self.data = TextureBuilderData::File(file_path);
        self
    }

    pub fn with_data(mut self, data: &'a [u8]) -> Self {
        self.data = TextureBuilderData::Data(data);
        self
    }

    pub fn with_raw(mut self, data: &'a [u8], size: Rect, format: TextureFormat) -> Self {
        if format >= TextureFormat::Stencil8 && format <= TextureFormat::Depth32FloatStencil8 {
            panic!("Depth and stencil formats are not supported in raw data");
        }

        self.data = TextureBuilderData::Raw(size, data, format);
        self
    }

    /// Initializes a texture as a render target.
    ///
    /// This method sets the texture as a render target with the specified size and format.
    /// The size must be non-zero, and the format can be specified or defaulted to the swapchain format or RGBA8_UNORM_SRGB if the
    /// swapchain format is not available.
    pub fn with_render_target(mut self, size: Rect, format: Option<TextureFormat>) -> Self {
        if size.w == 0 || size.h == 0 {
            panic!("Render target texture must have a size");
        }

        self.data = TextureBuilderData::RenderTarget(size, format);
        self
    }

    /// Sets the sample count for the texture.
    ///
    /// This method allows you to specify the sample count for the texture. The default is 1.
    /// **NOTE:** Will panic! if the sample count is not supported by the GPU (such above 4x in wasm).
    pub fn with_sample_count(mut self, sample_count: SampleCount) -> Self {
        self.sample_count = sample_count.into();
        self
    }

    /// Initializes a texture as a depth stencil texture.
    pub fn with_depth_stencil(mut self, size: Rect, format: Option<TextureFormat>) -> Self {
        if size.w == 0 || size.h == 0 {
            panic!("Depth stencil texture must have a size");
        }

        self.data = TextureBuilderData::DepthStencil(
            size,
            Some(format.unwrap_or(TextureFormat::Depth32Float)),
        );
        self
    }

    pub fn with_mip_level_count(mut self, mip_level_count: u32) -> Self {
        self.mip_level_count = mip_level_count;
        self
    }

    /// Sets the usage of the texture.
    ///
    /// This method allows you to specify the usage of the texture. However it cannot set the texture as
    /// a render target, as that must be done using the `with_render_target` method.
    pub fn with_usage(mut self, usage: TextureUsage) -> Self {
        if usage.contains(TextureUsage::RenderAttachment) {
            panic!("Render attachment textures must be created with the render target method");
        }

        self.usage = usage;
        self
    }

    pub fn build(self) -> Result<Texture, TextureError> {
        Texture::from_builder(self)
    }
}

pub struct TextureInner {
    pub(crate) wgpu_texture: wgpu::Texture,
    pub(crate) wgpu_view: wgpu::TextureView,

    pub(crate) size: Rect,
    pub(crate) usages: TextureUsage,
    pub(crate) sample_count: SampleCount,
    pub(crate) blend: TextureBlend,
    pub(crate) sampler_info: TextureSampler,
    pub(crate) format: TextureFormat,

    pub(crate) mapped: bool,
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<TextureInner>,

    pub(crate) mapped_buffer: Vec<u8>,
    pub(crate) mapped_type: TextureMappedType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMappedType {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy)]
pub enum TextureError {
    InvalidGPUContext,
    InvalidTextureData,
    InvalidTextureSize,
    InvalidTextureFormat,
    FailedToWrite,
    FailedToRead,
    AlreadyMapped,
    NotMapped,
}

impl std::fmt::Display for TextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureError::InvalidGPUContext => write!(f, "Invalid GPU context"),
            TextureError::InvalidTextureData => write!(f, "Invalid texture data"),
            TextureError::InvalidTextureSize => write!(f, "Invalid texture size"),
            TextureError::InvalidTextureFormat => write!(f, "Invalid texture format"),
            TextureError::FailedToWrite => write!(f, "Failed to write to texture"),
            TextureError::FailedToRead => write!(f, "Failed to read from texture"),
            TextureError::AlreadyMapped => write!(f, "Texture is already mapped"),
            TextureError::NotMapped => write!(f, "Texture is not mapped"),
        }
    }
}

static TEXTURE_REF_ID: AtomicUsize = AtomicUsize::new(0);

impl Texture {
    pub(crate) fn from_builder(builder: TextureBuilder) -> Result<Self, TextureError> {
        if builder.graphics.borrow().is_invalid {
            // return Err("Graphics context is invalid".to_string());
            return Err(TextureError::InvalidGPUContext);
        }

        let texture = match builder.data {
            TextureBuilderData::Data(data) => {
                let image = image::load_from_memory(data).map_err(|e| e.to_string());
                if image.is_err() {
                    dbg_log!(
                        "Failed to load image from memory: {}",
                        image.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                let image = image.unwrap();

                let rgba = image.to_rgba8();
                let dimensions = rgba.dimensions();
                let size = Rect::new(0, 0, dimensions.0 as i32, dimensions.1 as i32);

                let texture = Self::create_texture(
                    builder.graphics,
                    size,
                    builder.sample_count,
                    builder.mip_level_count,
                    wgpu::TextureDimension::D2,
                    TextureFormat::Rgba8Unorm,
                    builder.usage,
                );

                if texture.is_err() {
                    dbg_log!(
                        "Failed to create texture: {}",
                        texture.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                let mut texture = texture.unwrap();

                if let Err(e) = texture.write::<u8>(&rgba) {
                    return Err(e);
                }

                Ok(texture)
            }

            TextureBuilderData::File(file_path) => {
                let image = image::open(file_path).map_err(|e| e.to_string());
                if image.is_err() {
                    dbg_log!(
                        "Failed to load image from file: {}",
                        image.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                let image = image.unwrap();

                let rgba = image.to_rgba8();
                let dimensions = rgba.dimensions();
                let size = Rect::new(0, 0, dimensions.0 as i32, dimensions.1 as i32);

                let texture = Self::create_texture(
                    builder.graphics,
                    size,
                    builder.sample_count,
                    builder.mip_level_count,
                    wgpu::TextureDimension::D2,
                    TextureFormat::Rgba8Unorm,
                    builder.usage,
                );

                if texture.is_err() {
                    dbg_log!(
                        "Failed to create texture: {}",
                        texture.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                let mut texture = texture.unwrap();

                if let Err(e) = texture.write::<u8>(&rgba) {
                    dbg_log!("Failed to write texture data: {}", e);
                    return Err(e);
                }

                Ok(texture)
            }

            TextureBuilderData::Raw(size, data, format) => {
                let texture = Self::create_texture(
                    builder.graphics,
                    size,
                    builder.sample_count,
                    builder.mip_level_count,
                    wgpu::TextureDimension::D2,
                    format,
                    builder.usage,
                );

                if texture.is_err() {
                    dbg_log!(
                        "Failed to create texture: {}",
                        texture.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                let mut texture = texture.unwrap();
                if let Err(e) = texture.write::<u8>(data) {
                    dbg_log!("Failed to write texture data: {}", e);
                    return Err(e);
                }

                Ok(texture)
            }

            TextureBuilderData::DepthStencil(size, format) => {
                let texture = Self::create_texture(
                    builder.graphics,
                    size,
                    builder.sample_count,
                    builder.mip_level_count,
                    wgpu::TextureDimension::D2,
                    format.unwrap(),
                    builder.usage | TextureUsage::RenderAttachment,
                );

                if texture.is_err() {
                    dbg_log!(
                        "Failed to create depth stencil texture: {}",
                        texture.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                texture
            }

            TextureBuilderData::RenderTarget(size, format) => {
                let format = {
                    if format.is_none() {
                        let graphics_ref = builder.graphics.borrow();

                        if graphics_ref.config.is_none() {
                            dbg_log!(
                                "Using default format (RGBA8_UNORM_SRGB) for render target texture"
                            );
                            TextureFormat::Rgba8UnormSrgb
                        } else {
                            let config = graphics_ref.config.as_ref().unwrap();
                            dbg_log!(
                                "Using swapchain format ({:?}) for render target texture",
                                config.format
                            );
                            config.format.into()
                        }
                    } else {
                        format.unwrap()
                    }
                };

                let texture = Self::create_texture(
                    builder.graphics,
                    size,
                    builder.sample_count,
                    builder.mip_level_count,
                    wgpu::TextureDimension::D2,
                    TextureFormat::from(format),
                    builder.usage | TextureUsage::RenderAttachment,
                );

                if texture.is_err() {
                    dbg_log!(
                        "Failed to create render target texture: {}",
                        texture.as_ref().err().unwrap()
                    );
                    return Err(TextureError::InvalidTextureData);
                }

                texture
            }

            _ => {
                return Err(TextureError::InvalidTextureData);
            }
        };

        texture
    }

    fn create_texture(
        graphics: ArcRef<GPUInner>,
        size: Rect,
        sample_count: SampleCount,
        mip_level_count: u32,
        dimension: wgpu::TextureDimension,
        format: TextureFormat,
        usages: TextureUsage,
    ) -> Result<Self, TextureError> {
        if size.w == 0 || size.h == 0 {
            return Err(TextureError::InvalidTextureSize);
        }

        let texture_size = Extent3d {
            width: size.w as u32,
            height: size.h as u32,
            depth_or_array_layers: 1,
        };

        let ref_id_label = TEXTURE_REF_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let tex_label = format!("Texture {}", ref_id_label);
        let view_label = format!("Texture View {}", ref_id_label);

        let texture_create_info = TextureDescriptor {
            size: texture_size,
            mip_level_count,
            sample_count: sample_count.clone().into(),
            dimension,
            format: format.clone().into(),
            usage: (wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC)
                | usages.clone().into(),
            label: Some(tex_label.as_str()),
            view_formats: &[],
        };

        let graphics_ref = graphics.borrow();
        let texture = graphics_ref
            .get_device()
            .create_texture(&texture_create_info);

        let sampler = TextureSampler::DEFAULT;
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(view_label.as_str()),
            ..Default::default()
        });

        let inner = TextureInner {
            wgpu_texture: texture,
            wgpu_view: view,

            sample_count,
            usages,
            blend: TextureBlend::NONE,
            sampler_info: sampler,
            size,
            format,

            mapped: false,
        };

        Ok(Self {
            graphics: ArcRef::clone(&graphics),
            inner: ArcRef::new(inner),
            mapped_buffer: vec![],
            mapped_type: TextureMappedType::Write,
        })
    }

    pub fn get_sampler(&self) -> TextureSampler {
        self.inner.borrow().sampler_info.clone()
    }

    pub fn set_sampler(&mut self, sampler: TextureSampler) {
        let mut inner = self.inner.borrow_mut();
        inner.sampler_info = sampler.clone();
    }

    pub fn write<T: bytemuck::Pod>(&mut self, data: &[T]) -> Result<(), TextureError> {
        if data.is_empty() {
            return Err(TextureError::InvalidTextureData);
        }

        let inner = self.inner.borrow();

        let data: Vec<u8> = bytemuck::cast_slice(data).to_vec();
        let bytes_per_pixel = inner.format.get_size();
        let unpadded_bytes_per_row = bytes_per_pixel * inner.size.w as u32;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;

        let mut padded_data =
            Vec::with_capacity((padded_bytes_per_row * inner.size.h as u32) as usize);
        for row in 0..inner.size.h as usize {
            let start = row * unpadded_bytes_per_row as usize;
            let end = start + unpadded_bytes_per_row as usize;
            padded_data.extend_from_slice(&data[start..end]);
            padded_data.extend(vec![
                0;
                (padded_bytes_per_row - unpadded_bytes_per_row) as usize
            ]);
        }

        let buffer = BufferBuilder::<u8>::new(self.graphics.clone())
            .set_data_vec(padded_data)
            .set_usage(BufferUsage::COPY_SRC)
            .build();

        if buffer.is_err() {
            return Err(TextureError::FailedToWrite);
        }

        let buffer = buffer.unwrap();

        let mut encoder = self.graphics.borrow().get_device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("texture write encoder"),
            },
        );

        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfoBase {
                buffer: &buffer.inner.borrow().buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(inner.size.h as u32),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture: &inner.wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: inner.size.w as u32,
                height: inner.size.h as u32,
                depth_or_array_layers: 1,
            },
        );

        self.graphics
            .borrow()
            .get_queue()
            .submit(Some(encoder.finish()));
        _ = self
            .graphics
            .borrow()
            .get_device()
            .poll(wgpu::PollType::Wait);

        Ok(())
    }

    pub fn read<T: bytemuck::Pod>(&self) -> Result<Vec<T>, TextureError> {
        if self.inner.borrow().size.w == 0 || self.inner.borrow().size.h == 0 {
            return Err(TextureError::InvalidTextureSize);
        }

        let inner = self.inner.borrow();
        let inner_graphics = self.graphics.borrow();

        let bytes_per_pixel = 4; // For RGBA8/BGRA8, etc. Adjust if needed.
        let unpadded_bytes_per_row = bytes_per_pixel * inner.size.w as u32;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;

        let buffer = BufferBuilder::<u8>::new(self.graphics.clone())
            .set_data_empty((padded_bytes_per_row * inner.size.h as u32) as usize)
            .set_usage(BufferUsage::COPY_DST | BufferUsage::MAP_READ)
            .build();

        if buffer.is_err() {
            return Err(TextureError::FailedToRead);
        }

        let buffer = buffer.unwrap();

        let mut encoder =
            inner_graphics
                .get_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("texture read encoder"),
                });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &inner.wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer.inner.borrow().buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(inner.size.h as u32),
                },
            },
            inner.size.into(),
        );

        inner_graphics.get_queue().submit(Some(encoder.finish()));
        _ = inner_graphics.get_device().poll(wgpu::PollType::Wait);

        drop(inner_graphics);

        // Remove row padding
        let raw = buffer.read::<u8>();

        if raw.is_err() {
            return Err(TextureError::FailedToRead);
        }

        let raw = raw.unwrap();

        let height = inner.size.h as u32;
        let padded_bytes_per_row = padded_bytes_per_row as u32;

        let mut result = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
        for row in 0..height as usize {
            let start = row * padded_bytes_per_row as usize;
            let end = start + unpadded_bytes_per_row as usize;
            result.extend_from_slice(&raw[start..end]);
        }

        // Cast to T
        let ptr = result.as_ptr();
        let len = result.len() / std::mem::size_of::<T>();
        let mut out = Vec::with_capacity(len);
        unsafe {
            out.set_len(len);
            std::ptr::copy_nonoverlapping(ptr as *const T, out.as_mut_ptr(), len);
        }
        Ok(out)
    }

    pub fn map(&mut self, map_type: TextureMappedType) -> Result<&mut Vec<u8>, TextureError> {
        let mut inner = self.inner.borrow_mut();
        if inner.mapped {
            dbg_log!("Texture is already mapped");
            return Err(TextureError::AlreadyMapped);
        }

        match map_type {
            TextureMappedType::Read => {
                inner.mapped = true;
                drop(inner);

                self.mapped_type = TextureMappedType::Read;
                self.mapped_buffer = self.read::<u8>()?;

                return Ok(&mut self.mapped_buffer);
            }
            TextureMappedType::Write => {
                inner.mapped = true;
                drop(inner);

                self.mapped_type = TextureMappedType::Write;
                self.mapped_buffer =
                    vec![0; (self.inner.borrow().size.w * self.inner.borrow().size.h * 4) as usize];

                return Ok(&mut self.mapped_buffer);
            }
        }
    }

    pub fn unmap(&mut self) -> Result<(), TextureError> {
        let mut inner = self.inner.borrow_mut();
        if !inner.mapped {
            dbg_log!("Texture is not mapped");
            return Err(TextureError::NotMapped);
        }

        match self.mapped_type {
            TextureMappedType::Read => {
                inner.mapped = false;
                self.mapped_buffer.clear();
            }
            TextureMappedType::Write => {
                inner.mapped = false;

                drop(inner);
                let buffer = self.mapped_buffer.clone();

                if let Err(e) = self.write::<u8>(&buffer) {
                    dbg_log!("Failed to write texture data: {}", e);
                    return Err(e);
                }

                self.mapped_buffer = vec![];
            }
        }

        Ok(())
    }
}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Texture {}

impl PartialEq for TextureInner {
    fn eq(&self, other: &Self) -> bool {
        self.wgpu_texture == other.wgpu_texture &&
        self.wgpu_view == other.wgpu_view &&
        self.size == other.size &&
        self.usages == other.usages &&
        self.sample_count == other.sample_count &&
        // self.blend == other.blend &&
        // self.sampler_info == other.sampler_info &&
        self.format == other.format
    }
}

impl Eq for TextureInner {}
