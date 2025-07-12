use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    io::{Read, Write},
};

use byteorder_lite::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::bufread::ZlibDecoder;

use crate::{
    gpu::{
        GPU,
        GPUInner,
        texture::{Texture, TextureBuilder, TextureError, TextureFormat, TextureUsage},
    },
    math::{Point2, Vector2},
    utils::ArcRef,
};


/// Creates a new [FontManager] instance.
///
/// This is useful for loading and managing fonts for text rendering.
pub fn new() -> FontManager {
    FontManager::new()
}

mod system;

#[derive(Clone, Copy, Debug)]
pub struct FontStyle(u8);

bitflags::bitflags! {
    impl FontStyle: u8 {
        /// The font is bold.
        const BOLD = 0b00000001;
        /// The font is italic.
        const ITALIC = 0b00000010;
    }
}

#[derive(Clone, Debug)]
pub struct FontInfo {
    pub name: String,
    pub path: std::path::PathBuf,
    pub style: FontStyle,
}

#[derive(Clone, Debug)]
pub struct FontInner {
    pub info: FontInfo,
    pub glyphs: HashMap<u32, Glyph>,
    pub texture_buffer: Vec<u8>,
    pub texture_width: u32,
    pub texture_height: u32,
    pub ascender: f32,
    pub descender: f32,
    pub line_height: f32,
    pub space_width: f32,
}

#[derive(Clone, Debug)]
pub struct Font {
    pub(crate) inner: ArcRef<FontInner>,
}

const FONT_CACHE_MAGIC: [u8; 5] = *b"eFONT";
const MAX_ATLAS_SIZE: usize = 2048; // 2048x2048

fn power_of_two(n: usize) -> usize {
    let mut power = 1;
    while power < n {
        power *= 2;
    }
    power
}

pub enum FontBakeFormat {
    GrayScale,
    Rgba,
}

pub enum FontError {
    InvalidFontData(String),
    GlyphNotFound(u32),
    IoError(std::io::Error),
}

impl Font {
    pub(crate) fn new(info: FontInfo, size: f32, glyph_range: &[(u32, u32)]) -> Self {
        let data = std::fs::read(&info.path).expect("Failed to read font file");
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
            .expect("Failed to parse font file");

        let line_metrics = font.horizontal_line_metrics(size);
        let pixel_gap = 2usize; // Add a pixel gap to avoid artifacts

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if line_metrics.is_none() {
            panic!(
                "Failed to get line metrics for font: {}",
                info.path.display()
            );
        }

        let line_metrics = line_metrics.unwrap();

        let ascender = line_metrics.ascent;
        let descender = line_metrics.descent;
        let line_height = line_metrics.ascent - line_metrics.descent + line_metrics.line_gap;
        let space_metrics = font.metrics(' ', size);

        // Calculate texture estimated width based on glyph range
        // to avoid very WIDE font atlas
        let tex_width = {
            let mut total_area = 0;
            
            for &(start, end) in glyph_range {
                for codepoint in start..=end {
                    let codepoint_char = std::char::from_u32(codepoint).unwrap_or_default();
                    let metrics = font.metrics(codepoint_char, size);

                    total_area += ((metrics.width + pixel_gap) * (metrics.height + pixel_gap)) as usize;
                }
            }

            power_of_two((total_area as f32).sqrt().ceil() as usize) as i32
        };

        if tex_width > MAX_ATLAS_SIZE as i32 {
            panic!(
                "Calculated texture area {} exceeds maximum atlas size {}",
                tex_width, MAX_ATLAS_SIZE
            );
        }

        let rect_config = rect_packer::Config {
            width: tex_width,
            height: tex_width,
            border_padding: 0,
            rectangle_padding: pixel_gap as i32,
        };

        let mut packer = rect_packer::Packer::new(rect_config);
        let mut raw_glyphs = Vec::new();
        let mut max_size = Point2::new(0, 0);

        for &(start, end) in glyph_range {
            for codepoint in start..=end {
                let codepoint_char = std::char::from_u32(codepoint).unwrap_or_default();
                let (metrics, bitmap) = font.rasterize(codepoint_char, size);
                if bitmap.is_empty() {
                    continue;
                }

                if let Some(rect) = packer.pack(metrics.width as i32, metrics.height as i32, false) {
                    raw_glyphs.push(
                        (rect, codepoint, metrics, bitmap)
                    );

                    max_size.x = max_size.x.max(rect.x + rect.width);
                    max_size.y = max_size.y.max(rect.y + rect.height);
                } else {
                    #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                    panic!(
                        "Failed to pack glyph: {} ({}x{}) with atlas size {}x{}",
                        codepoint_char,
                        metrics.width,
                        metrics.height,
                        tex_width,
                        tex_width
                    );
                }
            }
        }

        let mut texture_buffer = vec![0; (max_size.x * max_size.y) as usize];
        let mut glyphs = HashMap::new();

        for (rect, codepoint, metrics, bitmap) in raw_glyphs {
            let advance = metrics.advance_width as f32;
            let glyph_width = metrics.width as usize;
            let glyph_height = metrics.height as usize;

            for j in 0..glyph_height {
                for i in 0..glyph_width {
                    let src_index = j * glyph_width + i;
                    let dest_x = rect.x as usize + i;
                    let dest_y = rect.y as usize + j;
                    let dest_index = dest_y * max_size.x as usize + dest_x;

                    if dest_index < texture_buffer.len() && src_index < bitmap.len() {
                        texture_buffer[dest_index] = bitmap[src_index];
                    }
                }
            }

            let start_offset = Vector2::new(rect.x as f32, rect.y as f32);
            let end_offset = Vector2::new(
                rect.x + glyph_width as i32,
                rect.y + glyph_height as i32,
            );

            let glyph = Glyph {
                codepoint,
                advance,
                atlas_start_offset: start_offset,
                atlas_end_offset: end_offset,

                width: glyph_width as f32,
                height: glyph_height as f32,
                bearing_x: metrics.xmin as f32,
                bearing_y: metrics.ymin as f32,
                advance_x: metrics.advance_width as f32,
                advance_y: metrics.advance_height as f32,
                ascender: -metrics.bounds.ymin.max(0.0) as f32,
                descender: (metrics.bounds.ymin + metrics.bounds.height) as f32,
            };

            glyphs.insert(codepoint, glyph);
        }

        let inner = FontInner {
            info,
            glyphs,
            texture_buffer,
            texture_width: max_size.x as u32,
            texture_height: max_size.y as u32,
            ascender,
            descender,
            line_height,
            space_width: space_metrics.advance_width as f32,
        };

        let inner = ArcRef::new(inner);
        
        Font {
            inner,
        }
    }

    pub fn line_height(&self) -> f32 {
        self.inner.borrow().line_height
    }

    pub fn ascender(&self) -> f32 {
        self.inner.borrow().ascender
    }

    pub fn descender(&self) -> f32 {
        self.inner.borrow().descender
    }

    pub fn space_width(&self) -> f32 {
        self.inner.borrow().space_width
    }

    pub fn texture_size(&self) -> Point2 {
        let inner = self.inner.borrow();
        Point2::new(inner.texture_width as i32, inner.texture_height as i32)
    }

    pub fn calculate_text_size(&self, text: &str) -> Vector2 {
        let inner = self.inner.borrow();

        let mut width = 0.0f32;
        let mut height = inner.line_height;

        let mut pen_x = 0.0;

        for c in text.chars() {
            let codepoint = c as u32;
            if codepoint == '\n' as u32 {
                width = width.max(pen_x);
                pen_x = 0.0;
                height += inner.line_height;
                continue;
            }

            if codepoint == ' ' as u32 {
                pen_x += inner.space_width;
                continue;
            }

            if let Some(glyph) = inner.glyphs.get(&codepoint) {
                pen_x += glyph.advance_x;
            }
        }

        width = width.max(pen_x);

        Vector2::new(width, height)
    }

    /// Bakes the text into a texture data buffer.
    ///
    /// This is useful for rendering static text without needing to render each glyph individually.
    pub fn create_baked_text_raw(
        &self,
        text: &str,
        format: FontBakeFormat,
    ) -> Result<(Vec<u8>, u32, u32), String> {
        let inner = self.inner.borrow();

        let mut pen = Vector2::new(0.0, 0.0);

        // Track bounding box
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        // let mut max_bearing_y = f32::MIN;

        for c in text.chars() {
            let codepoint = c as u32;
            if codepoint == '\n' as u32 {
                pen.x = 0.0;
                pen.y += inner.line_height as f32;
                continue;
            }

            if codepoint == ' ' as u32 {
                pen.x += inner.space_width;
                continue;
            }

            if let Some(glyph) = inner.glyphs.get(&codepoint) {
                let x0 = pen.x + glyph.bearing_x;
                let y0 = pen.y + inner.ascender - (glyph.height + glyph.bearing_y);
                let x1 = x0 + glyph.width;
                let y1 = y0 + glyph.height;

                min_x = min_x.min(x0);
                min_y = min_y.min(y0);
                max_x = max_x.max(x1);
                max_y = max_y.max(y1);

                pen.x += glyph.advance_x;
            }
        }

        // If no glyphs, return empty buffer
        if min_x == f32::MAX || min_y == f32::MAX {
            return Err("No glyphs found".to_string());
        }

        let width = (max_x - min_x).ceil().max(1.0) as usize;
        let height = (max_y - min_y).ceil().max(1.0) as usize;
        let mut buffer = vec![0; width * height];

        let mut pen2 = Vector2::new(0.0, 0.0);

        for c in text.chars() {
            let codepoint = c as u32;
            if codepoint == '\n' as u32 {
                pen2.x = 0.0;
                pen2.y += inner.line_height as f32;
                continue;
            }

            if codepoint == ' ' as u32 {
                pen2.x += inner.space_width;
                continue;
            }

            if let Some(glyph) = inner.glyphs.get(&codepoint) {
                let x0 = pen2.x + glyph.bearing_x - min_x;
                let y0 = pen2.y + inner.ascender - (glyph.height + glyph.bearing_y) - min_y;

                let atlas_offset_x = glyph.atlas_start_offset.x as usize;
                let atlas_offset_y = glyph.atlas_start_offset.y as usize;
                let atlas_width = inner.texture_width as usize;
                let atlas_height = inner.texture_height as usize;

                for y in 0..glyph.height as usize {
                    let src_start = (atlas_offset_y + y) * atlas_width + atlas_offset_x;
                    let dest_start = (y0 as usize + y) * width + x0 as usize;

                    for x in 0..glyph.width as usize {
                        let src_index = src_start + x;
                        let dest_index = dest_start + x;

                        if src_index < atlas_width * atlas_height && dest_index < buffer.len() {
                            buffer[dest_index] = inner.texture_buffer[src_index];
                        }
                    }
                }

                pen2.x += glyph.advance_x;
            }
        }

        match format {
            FontBakeFormat::GrayScale => Ok((buffer, width as u32, height as u32)),
            FontBakeFormat::Rgba => {
                let mut rgba_buffer = Vec::with_capacity(width * height * 4);
                for byte in buffer.iter() {
                    let is_transparent = *byte == 0;

                    rgba_buffer.push(*byte);
                    rgba_buffer.push(*byte);
                    rgba_buffer.push(*byte);
                    rgba_buffer.push(if is_transparent { 0 } else { 255 });
                }

                Ok((rgba_buffer, width as u32, height as u32))
            }
        }
    }

    pub(crate) fn new_cached(path: &str) -> Result<Self, std::io::Error> {
        let data = std::fs::read(path)?;
        let mut reader = std::io::Cursor::new(data);

        let mut magic = [0; 5];
        reader.read_exact(&mut magic)?;
        if magic != FONT_CACHE_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid font cache file",
            ));
        }

        let compressed_size = reader.read_u32::<LittleEndian>()?;
        let uncompressed_size = reader.read_u32::<LittleEndian>()?;

        let mut compressed_data = vec![0; compressed_size as usize];
        reader.read_exact(&mut compressed_data)?;

        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed_data = Vec::with_capacity(uncompressed_size as usize);
        decoder.read_to_end(&mut decompressed_data)?;

        let mut reader = std::io::Cursor::new(decompressed_data);

        let font_family_name_len = reader.read_u32::<LittleEndian>()?;
        let mut font_family_name = vec![0; font_family_name_len as usize];
        reader.read_exact(&mut font_family_name)?;
        let font_family_name = String::from_utf8(font_family_name).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid UTF-8 in font family name",
            )
        })?;
        let font_style = reader.read_u8()?;
        let font_style = FontStyle::from_bits(font_style).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid font style")
        })?;

        let info = FontInfo {
            name: font_family_name,
            path: std::path::PathBuf::from(path),
            style: font_style,
        };

        let num_glyphs = reader.read_u32::<LittleEndian>()?;
        let mut glyphs = HashMap::new();
        for _ in 0..num_glyphs {
            let codepoint = reader.read_u32::<LittleEndian>()?;
            let advance = reader.read_f32::<LittleEndian>()?;
            let atlas_start_offset = Vector2::new(
                reader.read_f32::<LittleEndian>()?,
                reader.read_f32::<LittleEndian>()?,
            );
            let atlas_end_offset = Vector2::new(
                reader.read_f32::<LittleEndian>()?,
                reader.read_f32::<LittleEndian>()?,
            );
            let width = reader.read_f32::<LittleEndian>()?;
            let height = reader.read_f32::<LittleEndian>()?;
            let bearing_x = reader.read_f32::<LittleEndian>()?;
            let bearing_y = reader.read_f32::<LittleEndian>()?;
            let advance_x = reader.read_f32::<LittleEndian>()?;
            let advance_y = reader.read_f32::<LittleEndian>()?;
            let ascender = reader.read_f32::<LittleEndian>()?;
            let descender = reader.read_f32::<LittleEndian>()?;

            let glyph = Glyph {
                codepoint,
                advance,
                atlas_start_offset,
                atlas_end_offset,
                width,
                height,
                bearing_x,
                bearing_y,
                advance_x,
                advance_y,
                ascender,
                descender,
            };

            glyphs.insert(codepoint, glyph);
        }

        let texture_buffer_width = reader.read_u32::<LittleEndian>()?;
        let texture_buffer_height = reader.read_u32::<LittleEndian>()?;

        if texture_buffer_width > MAX_ATLAS_SIZE as u32
            || texture_buffer_height > MAX_ATLAS_SIZE as u32
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid texture buffer size",
            ));
        }

        let mut texture_buffer = vec![0; (texture_buffer_width * texture_buffer_height) as usize];
        reader.read_exact(&mut texture_buffer)?;

        let ascender = reader.read_f32::<LittleEndian>()?;
        let descender = reader.read_f32::<LittleEndian>()?;
        let line_height = reader.read_f32::<LittleEndian>()?;
        let space_width = reader.read_f32::<LittleEndian>()?;

        let inner = FontInner {
            info,
            glyphs,
            texture_buffer,
            texture_width: texture_buffer_width,
            texture_height: texture_buffer_height,
            ascender,
            descender,
            line_height,
            space_width,
        };

        let inner = ArcRef::new(inner);

        Ok(Font {
            inner: ArcRef::clone(&inner),
        })
    }

    /// Saves the font cache to a file.
    ///
    /// This will create a binary file that can be loaded later using [FontManager::load_font_cached].
    pub fn save_font_cache(&self, path: &str) -> Result<(), std::io::Error> {
        let mut writer = std::fs::File::create(path)?;
        writer.write_all(&FONT_CACHE_MAGIC)?;

        let inner = self.inner.borrow();

        let mut writer2 = std::io::Cursor::new(Vec::<u8>::new());

        writer2.write_u32::<LittleEndian>(inner.info.name.len() as u32)?;
        writer2.write_all(inner.info.name.as_bytes())?;
        writer2.write_u8(inner.info.style.bits())?;

        writer2.write_u32::<LittleEndian>(inner.glyphs.len() as u32)?;
        for (_index, glyph) in inner.glyphs.iter() {
            writer2.write_u32::<LittleEndian>(glyph.codepoint)?;
            writer2.write_f32::<LittleEndian>(glyph.advance)?;
            writer2.write_f32::<LittleEndian>(glyph.atlas_start_offset.x)?;
            writer2.write_f32::<LittleEndian>(glyph.atlas_start_offset.y)?;
            writer2.write_f32::<LittleEndian>(glyph.atlas_end_offset.x)?;
            writer2.write_f32::<LittleEndian>(glyph.atlas_end_offset.y)?;
            writer2.write_f32::<LittleEndian>(glyph.width)?;
            writer2.write_f32::<LittleEndian>(glyph.height)?;
            writer2.write_f32::<LittleEndian>(glyph.bearing_x)?;
            writer2.write_f32::<LittleEndian>(glyph.bearing_y)?;
            writer2.write_f32::<LittleEndian>(glyph.advance_x)?;
            writer2.write_f32::<LittleEndian>(glyph.advance_y)?;
            writer2.write_f32::<LittleEndian>(glyph.ascender)?;
            writer2.write_f32::<LittleEndian>(glyph.descender)?;
        }

        writer2.write_u32::<LittleEndian>(inner.texture_width)?;
        writer2.write_u32::<LittleEndian>(inner.texture_height)?;
        writer2.write_all(&inner.texture_buffer)?;

        writer2.write_f32::<LittleEndian>(inner.ascender)?;
        writer2.write_f32::<LittleEndian>(inner.descender)?;
        writer2.write_f32::<LittleEndian>(inner.line_height)?;
        writer2.write_f32::<LittleEndian>(inner.space_width)?;

        let uncompressed_data: Vec<u8> = writer2.into_inner();
        let uncompressed_size = uncompressed_data.len() as u32;

        let mut compressed_data =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        compressed_data.write_all(&uncompressed_data)?;

        let compressed_data = compressed_data.finish()?;

        writer.write_u32::<LittleEndian>(compressed_data.len() as u32)?;
        writer.write_u32::<LittleEndian>(uncompressed_size as u32)?;
        writer.write_all(&compressed_data)?;

        Ok(())
    }

    /// Returns the image data of the font texture atlas.
    pub fn get_image_data(&self) -> (Vec<u8>, u32, u32) {
        let inner = self.inner.borrow();
        (
            inner.texture_buffer.clone(),
            inner.texture_width,
            inner.texture_height,
        )
    }

    /// Returns the font's glyph for the given codepoint.
    pub fn get_glyph(&self, codepoint: u32) -> Result<Glyph, FontError> {
        let inner = self.inner.borrow();

        inner
            .glyphs
            .get(&codepoint)
            .cloned()
            .ok_or(FontError::GlyphNotFound(codepoint))
    }

    /// Create a texture from the baked text.
    /// 
    /// This is useful for rendering static text without needing to render each glyph individually.
    pub fn create_baked_text(
        &self,
        gpu: &mut GPU,
        text: &str,
    ) -> Result<Texture, TextureError> {
        let (image_data, width, height) = self.create_baked_text_raw(text, FontBakeFormat::Rgba)
            .map_err(|_| TextureError::InvalidTextureData)?;

        let format = {
            let gpu_inner = gpu.inner.borrow();

            if gpu_inner.is_srgb() {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        };

        let texture = gpu
            .create_texture()
            .set_raw_image(&image_data, Point2::new(width as i32, height as i32), format)
            .set_usage(TextureUsage::Sampler)
            .build()?;

        Ok(texture)
    }

    /// Creates a texture from the font's glyph atlas.
    pub fn create_texture(&self, gpu: &mut GPU) -> Result<Texture, TextureError> {
        let gpu_inner = &gpu.inner;

        self.create_texture_inner(&gpu_inner)
    }

    pub(crate) fn create_texture_inner(
        &self,
        gpu: &ArcRef<GPUInner>,
    ) -> Result<Texture, TextureError> {
        let (image_data, width, height) = self.get_image_data();

        let format = {
            let gpu_inner = gpu.borrow();

            if gpu_inner.is_srgb() {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        };

        let image_data = {
            let mut data = Vec::with_capacity(image_data.len() * 4);
            for &pixel in &image_data {
                let is_transparent_pixel = pixel == 0;
                data.push(pixel);
                data.push(pixel);
                data.push(pixel);
                data.push(if is_transparent_pixel { 0 } else { 255 });
            }

            data
        };

        let texture = TextureBuilder::new(ArcRef::clone(gpu))
            .set_raw_image(
                &image_data,
                Point2::new(width as i32, height as i32),
                format,
            )
            .set_usage(TextureUsage::Sampler)
            .build()?;

        Ok(texture)
    }
}

#[derive(Clone, Debug)]
pub struct Glyph {
    pub codepoint: u32,
    pub advance: f32,
    pub atlas_start_offset: Vector2,
    pub atlas_end_offset: Vector2,

    // Metrics
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance_x: f32,
    pub advance_y: f32,
    pub ascender: f32,
    pub descender: f32,
}

impl Eq for Glyph {}

impl PartialEq for Glyph {
    fn eq(&self, other: &Self) -> bool {
        self.codepoint == other.codepoint
    }
}

#[derive(Clone, Debug)]
pub struct FontManager {
    fonts: Vec<FontInfo>,
    cached_font: HashMap<u64, Font>,
}

const DEFAULT_GLYPH_RANGE: [(u32, u32); 1] = [(0x20, 0x7E)]; // ASCII range

impl FontManager {
    /// Creates a new FontManager instance.
    /// 
    /// This will search for system fonts and cache them.
    /// It will also initialize an empty cache for loaded fonts.
    pub fn new() -> Self {
        let fonts = system::search_system_font();
        FontManager {
            fonts,
            cached_font: HashMap::new(),
        }
    }

    /// Loads a font by name and size, optionally specifying a glyph range.
    ///
    /// If the font is already cached, it will return the cached version.
    /// If the font is not found, it will return `None`.
    pub fn load_font(
        &mut self,
        font_name: &str,
        glyph_range: Option<&[(u32, u32)]>,
        size: f32,
    ) -> Option<Font> {
        let glyph_range = glyph_range.unwrap_or(&DEFAULT_GLYPH_RANGE);

        let hashed_name = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            font_name.hash(&mut hasher);
            for (start, end) in glyph_range {
                start.hash(&mut hasher);
                end.hash(&mut hasher);
            }
            size.to_bits().hash(&mut hasher);
            hasher.finish()
        };

        if self.cached_font.contains_key(&hashed_name) {
            return self.cached_font.get(&hashed_name).cloned();
        }

        if std::path::Path::new(font_name).exists() {
            let path = std::path::Path::new(font_name);

            let font_info = system::get_font_info(path);
            if font_info.is_none() {
                return None;
            }

            let font_info = font_info.unwrap();
            let font = Font::new(font_info, size, glyph_range);

            self.cached_font.insert(hashed_name, font.clone());

            return Some(font);
        } else {
            for font in &self.fonts {
                if font.name == font_name {
                    let font = Font::new(font.clone(), size, glyph_range);
                    self.cached_font.insert(hashed_name, font.clone());
                    return Some(font);
                }
            }
        }

        None
    }

    /// Loads a font from a cached file.
    ///
    /// This will load the font from a binary file created by [Font::save_font_cache].
    /// If the font is already cached, it will return the cached version.
    pub fn load_font_cached(&mut self, path: &str) -> Option<Font> {
        let hash_id = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hasher);
            hasher.finish()
        };

        if self.cached_font.contains_key(&hash_id) {
            return self.cached_font.get(&hash_id).cloned();
        }

        match Font::new_cached(path) {
            Ok(font) => {
                self.cached_font.insert(hash_id, font.clone());
                Some(font)
            }
            Err(_) => None,
        }
    }
}