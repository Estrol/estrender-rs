//! Implementation of the software renderer using softbuffer crate.
//!
//! This module provides a software renderer that can be used for rendering graphics without relying on a GPU.
//! Does not provided any high-level abstractions such drawing quad or image, but rather low-level access to the softbuffer crate. \
//! Provided as it, without any guarantees of performance or correctness.

use std::{num::NonZero, sync::Arc};

use winit::dpi::PhysicalSize;

use crate::{math::Point2, utils::ArcRef, window::Window};

/// Creates a new [software::PixelBuffer] instance. \
/// This is not thread-safe and must be called from the same thread as the window.
pub fn new<'a>(window: Option<&'a mut super::window::Window>) -> PixelBufferBuilder<'a> {
    let builder = PixelBufferBuilder::new();

    if let Some(window) = window {
        builder.with_window(window)
    } else {
        builder
    }
}

/// A wrapper around softbuffer to provide a soft buffer for pixel manipulation
#[derive(Clone, Debug)]
pub struct PixelBuffer {
    pub(crate) inner: ArcRef<PixelBufferInner>,
}

impl PixelBuffer {
    pub(crate) fn new(window: &Window) -> Result<Self, PixelBufferError> {
        let window_inner = window.inner.wait_borrow_mut();

        let window_handle = {
            let handle = window_inner.window_pointer.as_ref();

            if handle.is_none() {
                return Err(PixelBufferError::WindowPointerIsNull);
            }

            let handle = handle.unwrap();
            let window_handle = handle.lock().get_window().clone();

            window_handle
        };

        let context = SoftbufferContext::new(window_handle.clone());

        if context.is_err() {
            return Err(PixelBufferError::ContextCreationFailed);
        }

        let context = context.unwrap();
        let surface = SoftbufferSurface::new(&context, window_handle);

        if surface.is_err() {
            return Err(PixelBufferError::SurfaceCreationFailed);
        }

        let surface = surface.unwrap();
        let softbuffer_inner = PixelBufferInner {
            _context: context,
            surface,
            surface_size: Point2::new(0.0, 0.0),
        };

        let softbuffer_inner = ArcRef::new(softbuffer_inner);

        Ok(PixelBuffer {
            inner: softbuffer_inner,
        })
    }

    /// Get the size of the soft buffer surface
    /// Returns the size of the soft buffer surface in pixels
    pub fn size(&self) -> Point2 {
        let inner = self.inner.wait_borrow();
        inner.surface_size
    }

    /// Write pixels to the soft buffer surface
    pub fn write_buffers(&mut self, pixels: &[u32], size: Point2) -> Result<(), PixelBufferError> {
        let mut inner = self.inner.wait_borrow_mut();

        if pixels.len() != (size.x * size.y) as usize {
            return Err(PixelBufferError::InvalidSize(size.x as u32, size.y as u32));
        }

        if inner.surface_size == Point2::new(0.0, 0.0) {
            return Err(PixelBufferError::InvalidSurfaceSize);
        }

        let pixel_buffers = inner.surface.buffer_mut();
        if pixel_buffers.is_err() {
            return Err(PixelBufferError::BufferFetchFailed);
        }

        let mut pixel_buffers = pixel_buffers.unwrap();
        if pixel_buffers.len() < pixels.len() {
            return Err(PixelBufferError::BufferTooSmall);
        }

        for (i, pixel) in pixels.iter().enumerate() {
            pixel_buffers[i] = *pixel;
        }

        let res = pixel_buffers.present();
        if res.is_err() {
            return Err(PixelBufferError::PresentFailed);
        }

        Ok(())
    }
}

pub type SoftbufferSurface = softbuffer::Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>;
pub type SoftbufferContext = softbuffer::Context<Arc<winit::window::Window>>;

pub(crate) struct PixelBufferInner {
    pub _context: SoftbufferContext,
    pub surface: SoftbufferSurface,
    pub surface_size: Point2,
}

impl PixelBufferInner {
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), String> {
        if size.width == 0 || size.height == 0 {
            return Err("Invalid size".to_string());
        }

        self.surface_size = Point2::new(size.width as f32, size.height as f32);

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


pub struct PixelBufferBuilder<'a> {
    window: Option<&'a mut Window>,
}

impl<'a> PixelBufferBuilder<'a> {
    pub(crate) fn new() -> Self {
        PixelBufferBuilder { window: None }
    }

    /// Sets the window for this PixelBuffer instance. \
    /// This is useful for creating a PixelBuffer instance that is bound to a specific window.
    pub fn with_window(mut self, window: &'a mut Window) -> Self {
        self.window = Some(window);
        self
    }

    pub fn build(self) -> Result<PixelBuffer, PixelBufferBuilderError> {
        if self.window.is_none() {
            return Err(PixelBufferBuilderError::WindowIsNull);
        }

        let window = self.window.unwrap();

        let is_graphics_exist = {
            let window_inner = window.inner.borrow();
            window_inner.graphics.is_some()
        };

        if is_graphics_exist {
            return Err(PixelBufferBuilderError::CannotUseWithGPUWindow);
        }

        let pixel_buffer =
            PixelBuffer::new(window).map_err(|e| PixelBufferBuilderError::PixelBufferError(e))?;

        let mut window_inner = window.inner.borrow_mut();
        window_inner.pixelbuffer = Some(pixel_buffer.inner.clone());

        Ok(pixel_buffer)
    }
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

#[derive(Clone, Copy, Debug)]
pub enum PixelBufferError {
    WindowPointerIsNull,
    ContextCreationFailed,
    SurfaceCreationFailed,
    InvalidSize(u32, u32),
    InvalidSurfaceSize,
    BufferFetchFailed,
    BufferTooSmall,
    PresentFailed,
}

impl std::fmt::Display for PixelBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelBufferError::WindowPointerIsNull => write!(f, "Window pointer is null"),
            PixelBufferError::ContextCreationFailed => {
                write!(f, "Failed to create pixel buffer context")
            }
            PixelBufferError::SurfaceCreationFailed => {
                write!(f, "Failed to create pixel buffer surface")
            }
            PixelBufferError::InvalidSize(width, height) => {
                write!(f, "Invalid size: {}x{}", width, height)
            }
            PixelBufferError::InvalidSurfaceSize => write!(f, "Pixel buffer surface size is zero"),
            PixelBufferError::BufferFetchFailed => write!(f, "Failed to fetch pixel buffer"),
            PixelBufferError::BufferTooSmall => write!(f, "Pixel buffer is too small"),
            PixelBufferError::PresentFailed => write!(f, "Failed to present pixel buffer"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PixelBufferBuilderError {
    WindowIsNull,
    CannotUseWithGPUWindow,
    PixelBufferError(PixelBufferError),
}

impl std::fmt::Display for PixelBufferBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelBufferBuilderError::WindowIsNull => {
                write!(f, "PixelBuffer must be created with a window")
            }
            PixelBufferBuilderError::CannotUseWithGPUWindow => write!(
                f,
                "PixelBuffer cannot be created alongside GPU (hardware rendering)"
            ),
            PixelBufferBuilderError::PixelBufferError(e) => write!(f, "PixelBuffer error: {}", e),
        }
    }
}
