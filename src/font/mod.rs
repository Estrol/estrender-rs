use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    io::{Read, Write},
};

use byteorder_lite::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::bufread::ZlibDecoder;
// use gpu::GPUFont;

use crate::{math::Vector2, utils::ArcRef};

// mod gpu;
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

pub struct FontInner {
    pub info: FontInfo,
    pub glyphs: HashMap<u32, Glyph>,
    pub texture_buffer: Vec<u8>,
    pub texture_width: u32,
    pub texture_height: u32,
    pub ascender: f32,
    pub descender: f32,
    pub line_height: f32,
}

#[derive(Clone)]
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

        let mut total_area = 0;
        let mut max_glyph_width = 0;
        let mut max_glyph_height = 0;
        let mut raw_glyph = Vec::new();
        for &(start, end) in glyph_range {
            for codepoint in start..=end {
                let codepoint_char = std::char::from_u32(codepoint).unwrap_or_default();
                let (metrics, bitmap) = font.rasterize(codepoint_char, size);
                if bitmap.is_empty() {
                    continue;
                }

                total_area += (metrics.width * metrics.height) as u32;
                max_glyph_width = max_glyph_width.max(metrics.width);
                max_glyph_height = max_glyph_height.max(metrics.height);

                raw_glyph.push((codepoint, metrics, bitmap));
            }
        }

        let estimated_side = (total_area as f32).sqrt().ceil() as usize;
        let atlas_width = power_of_two(estimated_side);
        let atlas_height = power_of_two(estimated_side);

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if atlas_width > MAX_ATLAS_SIZE || atlas_height > MAX_ATLAS_SIZE {
            panic!(
                "Font texture atlas is too large: {}x{} (max: {}x{})",
                atlas_width, atlas_height, MAX_ATLAS_SIZE, MAX_ATLAS_SIZE
            );
        }

        let mut x = 0;
        let mut y = 0;
        let mut row_height = 0;

        let mut texture_buffer = vec![0; atlas_width * atlas_height];
        let mut glyphs = HashMap::new();

        for (codepoint, metrics, bitmap) in raw_glyph {
            let advance = metrics.width as f32;

            let glyph_width = metrics.width as usize;
            let glyph_height = metrics.height as usize;

            if x + glyph_width > atlas_width {
                x = 0;
                y += row_height;
                row_height = 0;
            }

            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            if y + glyph_height > atlas_height {
                // panic!("Font texture atlas is too small, try reducing the number of glyphs.");
                panic!(
                    "Font texture atlas is too small: {}x{} (max: {}x{}), trying to fit glyph: {} at ({}, {})",
                    atlas_width, atlas_height, MAX_ATLAS_SIZE, MAX_ATLAS_SIZE, codepoint, x, y
                );
            }

            for row in 0..glyph_height {
                let dest_start = (x + (y + row) * atlas_width) as usize;
                let src_start = row * glyph_width;
                let src_end = src_start + glyph_width;
                texture_buffer[dest_start..dest_start + glyph_width]
                    .copy_from_slice(&bitmap[src_start..src_end]);
            }

            if glyph_height > row_height {
                row_height = glyph_height;
            }

            let start_offset = Vector2::new(x as f32, y as f32);
            let end_offset = Vector2::new((x + glyph_width) as f32, (y + glyph_height) as f32);

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

            x += glyph_width;
        }

        let inner = FontInner {
            info,
            glyphs,
            texture_buffer,
            texture_width: atlas_width as u32,
            texture_height: atlas_height as u32,
            ascender,
            descender,
            line_height,
        };

        let inner = ArcRef::new(inner);
        let font = Font { inner };

        Font {
            inner: ArcRef::clone(&font.inner),
        }
    }

    pub fn bake_text(
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

        let mut pen = Vector2::new(0.0, 0.0);

        for c in text.chars() {
            let codepoint = c as u32;
            if codepoint == '\n' as u32 {
                pen.x = 0.0;
                pen.y += inner.line_height as f32;
                continue;
            }

            if let Some(glyph) = inner.glyphs.get(&codepoint) {
                let x0 = pen.x + glyph.bearing_x - min_x;
                let y0 = pen.y + inner.ascender - (glyph.height + glyph.bearing_y) - min_y;

                println!("Drawing glyph: {} at ({}, {})", codepoint, x0, y0);

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

                pen.x += glyph.advance_x;
            }
        }

        match format {
            FontBakeFormat::GrayScale => Ok((buffer, width as u32, height as u32)),
            FontBakeFormat::Rgba => {
                let mut rgba_buffer = vec![0; width * height * 4];
                for (i, pixel) in buffer.iter().enumerate() {
                    rgba_buffer[i * 4] = *pixel; // R
                    rgba_buffer[i * 4 + 1] = *pixel; // G
                    rgba_buffer[i * 4 + 2] = *pixel; // B
                    rgba_buffer[i * 4 + 3] = 255; // A
                }

                Ok((rgba_buffer, width as u32, height as u32))
            }
        }
    }

    pub fn new_cached(path: &str) -> Result<Self, std::io::Error> {
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

        let inner = FontInner {
            info,
            glyphs,
            texture_buffer,
            texture_width: texture_buffer_width,
            texture_height: texture_buffer_height,
            ascender,
            descender,
            line_height,
        };

        let inner = ArcRef::new(inner);

        Ok(Font {
            inner: ArcRef::clone(&inner),
        })
    }

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

    pub fn get_image_data(&self) -> (Vec<u8>, u32, u32) {
        let inner = self.inner.borrow();
        (
            inner.texture_buffer.clone(),
            inner.texture_width,
            inner.texture_height,
        )
    }

    // pub fn create_gpu(&self, gpu: &mut GPU) -> Result<GPUFont, String> {
    //     GPUFont::new(self, gpu)
    // }
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

pub struct FontManager {
    fonts: Vec<FontInfo>,
    cached_font: HashMap<u64, Font>,
}

const DEFAULT_GLYPH_RANGE: [(u32, u32); 1] = [(0x20, 0x7E)]; // ASCII range

impl FontManager {
    pub fn new() -> Self {
        let fonts = system::search_system_font();
        FontManager {
            fonts,
            cached_font: HashMap::new(),
        }
    }

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
}
