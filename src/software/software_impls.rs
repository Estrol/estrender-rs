use crate::{
    math::Vector2,
    software::{software_inner::{PixelBufferInner, SoftbufferContext, SoftbufferSurface}, PixelBufferBuilderError, PixelBufferError},
    utils::ArcRef,
    window::Window,
};

//// A wrapper around softbuffer to provide a soft buffer for pixel manipulation
pub struct PixelBuffer {
    pub(crate) inner: ArcRef<PixelBufferInner>,
}

impl PixelBuffer {
    pub(crate) fn new(window: &Window) -> Result<Self, PixelBufferError> {
        let window_inner = window.inner.wait_borrow_mut();

        let window_handle = {
            let handle = window_inner
                .window_pointer
                .as_ref();

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

        let pixel_buffer = PixelBuffer::new(window)
            .map_err(|e| PixelBufferBuilderError::PixelBufferError(e))?;

        let mut window_inner = window.inner.borrow_mut();
        window_inner.pixelbuffer = Some(pixel_buffer.inner.clone());

        Ok(pixel_buffer)
    }
}
