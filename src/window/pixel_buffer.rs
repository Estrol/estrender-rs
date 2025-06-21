#![cfg(feature = "software")]

use std::{num::NonZero, sync::Arc};

use softbuffer::{Context, Surface};
use winit::dpi::PhysicalSize;

use crate::{
    math::Vector2,
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
    pub fn write_buffers(&mut self, pixels: &[u32], size: Vector2) -> Result<(), String> {
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
}