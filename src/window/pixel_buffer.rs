#![cfg(feature = "software")]

use std::{num::NonZero, sync::Arc};

use softbuffer::{Context, Surface};
use winit::dpi::PhysicalSize;

use crate::{
    math::{Color, Vector2},
    utils::ArcRef,
};

use super::Window;

type SoftbufferSurface = Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>;
type SoftbufferContext = Context<Arc<winit::window::Window>>;

pub(crate) struct PixelBufferInner {
    pub _context: SoftbufferContext,
    pub surface: SoftbufferSurface,
    pub surface_size: Vector2,
}

impl PixelBufferInner {
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), String> {
        if size.width == 0 || size.height == 0 {
            return Err("Invalid size".to_string());
        }

        self.surface_size = Vector2::new(size.width as f32, size.height as f32);

        let width: NonZero<u32> = NonZero::new(size.width).ok_or("Width cannot be zero")?;
        let height: NonZero<u32> = NonZero::new(size.height).ok_or("Height cannot be zero")?;

        let result = self.surface.resize(width, height);
        if result.is_err() {
            return Err(format!(
                "Failed to resize softbuffer surface: {:?}",
                result.err()
            ));
        }

        Ok(())
    }
}

//// A wrapper around softbuffer to provide a soft buffer for pixel manipulation
pub struct PixelBuffer {
    pub(crate) inner: ArcRef<PixelBufferInner>,
}

#[derive(Clone, Copy, Debug)]
pub enum PixelWriteMode {
    // Append to the existing pixel value
    Copy,
    // Replace the existing pixel value with the new one
    Clear,
    // Blend the new pixel value with the existing one, such as alpha blending
    Blend,
}

#[derive(Clone, Copy, Debug)]
pub enum PixelBlendMode {
    // Alpha blending
    Alpha,
    // Additive blending
    Add,
    // Subtractive blending
    Subtract,
    // Multiplicative blending
    Multiply,
}

impl PixelBuffer {
    pub(crate) fn new(window: &Window) -> Result<Self, String> {
        let window_inner = window.inner.wait_borrow_mut();

        let window_handle = {
            let handle = window_inner.window_pointer
                .as_ref()
                .ok_or("Window pointer is not set")?
                .lock();

            let window_handle = handle
                .get_window();

            window_handle.clone()
        };

        let context = SoftbufferContext::new(window_handle.clone());
        
        if context.is_err() {
            return Err(format!(
                "Failed to create softbuffer context: {:?}",
                context.err()
            ));
        }

        let context = context.unwrap();
        let surface =
            SoftbufferSurface::new(&context, window_handle);

        if surface.is_err() {
            return Err(format!(
                "Failed to create softbuffer surface: {:?}",
                surface.err()
            ));
        }

        let surface = surface.unwrap();
        let softbuffer_inner = PixelBufferInner {
            _context: context,
            surface,
            surface_size: Vector2::new(0.0, 0.0),
        };

        let softbuffer_inner = ArcRef::new(softbuffer_inner);

        Ok(PixelBuffer {
            inner: softbuffer_inner,
        })
    }

    /// Get the size of the soft buffer surface
    /// Returns the size of the soft buffer surface in pixels
    pub fn get_size(&self) -> Vector2 {
        let inner = self.inner.wait_borrow();
        inner.surface_size
    }

    /// Write pixels to the soft buffer surface
    pub fn write_buffers(&self, pixels: &[u32], size: Vector2) -> Result<(), String> {
        let mut inner = self.inner.wait_borrow_mut();

        if pixels.len() != (size.x * size.y) as usize {
            return Err("Invalid pixel buffer size".to_string());
        }

        if inner.surface_size == Vector2::new(0.0, 0.0) {
            return Err("Pixel buffer surface size is zero".to_string());
        }

        let pixel_buffers = inner.surface.buffer_mut();
        if pixel_buffers.is_err() {
            return Err("Failed to get pixel buffer".to_string());
        }

        let mut pixel_buffers = pixel_buffers.unwrap();
        if pixel_buffers.len() < pixels.len() {
            return Err("Pixel buffer is too small".to_string());
        }

        for (i, pixel) in pixels.iter().enumerate() {
            pixel_buffers[i] = *pixel;
        }

        let res = pixel_buffers.present();
        if res.is_err() {
            return Err(format!("Failed to present pixel buffer: {:?}", res.err()));
        }

        Ok(())
    }

    pub fn begin_drawing(&self) -> Option<PixelBufferDrawing> {
        let size = self.get_size();
        PixelBufferDrawing::new(self, size)
    }
}

pub struct PixelBufferDrawing<'a> {
    pub instance: &'a PixelBuffer,
    pub pixel_buffer: Vec<u8>,
    pub size: Vector2,
}

impl<'a> PixelBufferDrawing<'a> {
    pub fn new(instance: &'a PixelBuffer, size: Vector2) -> Option<Self> {
        if size.x <= 0.0 || size.y <= 0.0 {
            return None;
        }

        let pixel_buffer = vec![0u8; (size.x * size.y * 4.0) as usize];

        Some(PixelBufferDrawing {
            instance,
            pixel_buffer,
            size,
        })
    }

    pub fn quad(&mut self, pos: Vector2, size: Vector2, color: Color) {
        self.line(pos, Vector2::new(pos.x + size.x, pos.y), color);
        self.line(
            Vector2::new(pos.x + size.x, pos.y),
            Vector2::new(pos.x + size.x, pos.y + size.y),
            color,
        );
        self.line(
            Vector2::new(pos.x + size.x, pos.y + size.y),
            Vector2::new(pos.x, pos.y + size.y),
            color,
        );
        self.line(Vector2::new(pos.x, pos.y + size.y), pos, color);
    }

    pub fn quad_filled(&mut self, pos: Vector2, size: Vector2, color: Color) {
        let contents = vec![
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        self.set_pixel_bytes(pos, size, &contents);
    }

    pub fn triangle(&mut self, t0: Vector2, t1: Vector2, t2: Vector2, color: Color) {
        self.line(t0, t1, color);
        self.line(t1, t2, color);
        self.line(t2, t0, color);
    }

    fn edge_function(a: Vector2, b: Vector2, c: Vector2) -> f32 {
        (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
    }

    pub fn triangle_filled(&mut self, a: Vector2, b: Vector2, c: Vector2, color: Color) {
        let abc = Self::edge_function(a, b, c);
        if abc < 0.0 {
            return;
        }

        let min_x = a.x.min(b.x).min(c.x).floor() as i32;
        let max_x = a.x.max(b.x).max(c.x).ceil() as i32;
        let min_y = a.y.min(b.y).min(c.y).floor() as i32;
        let max_y = a.y.max(b.y).max(c.y).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = Vector2::new(x as f32, y as f32);
                let ab = Self::edge_function(a, b, p);
                let bc = Self::edge_function(b, c, p);
                let ca = Self::edge_function(c, a, p);

                if ab >= 0.0 && bc >= 0.0 && ca >= 0.0 {
                    self.set_pixel(p, bytemuck::cast_slice(&[color]));
                }
            }
        }
    }

    pub fn circle(&mut self, center: Vector2, radius: f32, amount: usize, color: Color) {
        let angle_step = 2.0 * std::f32::consts::PI / amount as f32;
        let mut previous_point = Vector2::new(
            center.x + radius * (0.0_f32).cos(),
            center.y + radius * (0.0_f32).sin(),
        );

        for i in 1..=amount {
            let angle = i as f32 * angle_step;
            let current_point = Vector2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );

            // Draw a line between the previous point and the current point
            self.line(previous_point, current_point, color);

            // Update the previous point
            previous_point = current_point;
        }
    }

    pub fn circle_filled(&mut self, center: Vector2, radius: f32, amount: usize, color: Color) {
        let angle_step = 2.0 * std::f32::consts::PI / amount as f32;
        let mut prev = Vector2::new(
            center.x + radius * 0.0_f32.cos(),
            center.y + radius * 0.0_f32.sin(),
        );

        for i in 1..=amount {
            let angle = i as f32 * angle_step;
            let curr = Vector2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );

            self.triangle_filled(center, prev, curr, color);
            prev = curr;
        }
    }

    pub fn line(&mut self, start: Vector2, end: Vector2, color: Color) {
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        let steps = dx.max(dy).ceil() as usize; // Use ceil to ensure at least one step

        if steps == 0 {
            self.set_pixel(start, bytemuck::cast_slice(&[color]));
            return;
        }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = start.x + (end.x - start.x) * t;
            let y = start.y + (end.y - start.y) * t;
            let pos = Vector2::new(x, y);
            self.set_pixel(pos, bytemuck::cast_slice(&[color]));
        }
    }

    pub fn set_pixel(&mut self, pos: Vector2, contents: &[u8]) {
        let index = (pos.y as usize * self.size.x as usize + pos.x as usize) * 4;
        if index + 3 < self.pixel_buffer.len() {
            let r = contents[0];
            let g = contents[1];
            let b = contents[2];
            let a = contents[3];

            self.pixel_buffer[index] = b; // Blue
            self.pixel_buffer[index + 1] = g; // Green
            self.pixel_buffer[index + 2] = r; // Red
            self.pixel_buffer[index + 3] = a; // Alpha
        }
    }

    pub fn set_pixel_bytes(&mut self, pos: Vector2, size: Vector2, contents: &[u8]) {
        let width = size.x as usize;
        let height = size.y as usize;
        let dest_width = self.size.x as usize;

        for y in 0..height {
            for x in 0..width {
                let src_index = (y * width + x) * 4;
                let dest_index = ((pos.y as usize + y) * dest_width + (pos.x as usize + x)) * 4;

                if dest_index + 3 < self.pixel_buffer.len() && src_index + 3 < contents.len() {
                    // Read source pixel
                    let r = contents[src_index];
                    let g = contents[src_index + 1];
                    let b = contents[src_index + 2];
                    let a = contents[src_index + 3];

                    // Write to destination pixel in BGRA format
                    self.pixel_buffer[dest_index] = b; // Blue
                    self.pixel_buffer[dest_index + 1] = g; // Green
                    self.pixel_buffer[dest_index + 2] = r; // Red
                    self.pixel_buffer[dest_index + 3] = a; // Alpha
                }
            }
        }
    }

    pub(crate) fn end(&self) {
        let u32_pixels = bytemuck::cast_slice::<u8, u32>(&self.pixel_buffer);
        let size = Vector2::new(self.pixel_buffer.len() as f32 / 4.0, 1.0);

        let result = self.instance.write_buffers(u32_pixels, size);
        if result.is_err() {
            eprintln!("Failed to write pixel buffer: {:?}", result.err());
        }
    }
}

impl<'a> Drop for PixelBufferDrawing<'a> {
    fn drop(&mut self) {
        self.end();
    }
}
