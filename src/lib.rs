//! Easy to use winit, softbuffer & wgpu abstractions

/// Font rendering and text layout utilities
pub mod font;
/// GPU graphics rendering abstractions
pub mod gpu;
/// Mathematical utilities and types
pub mod math;
/// Predefined types and traits for easy access
pub mod prelude;
/// Utility functions and types for common tasks
pub mod utils;
/// Window management and event handling abstractions
pub mod window;
use gpu::{GPU, GPUAdapter};
use window::{Runner, Window};

#[cfg(feature = "software")]
use window::pixel_buffer::PixelBuffer;

use crate::prelude::Limits;

/// Create a EventLoop instance, required for creating one or more windows.
///
/// **NOTE:** When calling this function, the thread will be made the main thread,
/// future calls to this function will panic if called from a different thread.
///
/// # Example
/// ```rs
/// use engine::prelude::*;
///
/// let mut event_loop = Engine::make_event_loop();
/// ```
pub fn create_runner() -> Result<Runner, String> {
    Runner::new()
}

/// Creates a new GPU instance.
///
/// This is thread-safe and can be called from any thread, except when using
/// the `with_window` method, which binds the GPU to the window's thread.
///
/// # Example
/// ```rs
/// use engine::prelude::*;
///
/// let gpu = Engine::make_gpu()
///    .build();
/// ```
pub fn create_gpu<'a>(window: Option<&'a mut Window>) -> GPUBuilder<'a> {
    let builder = GPUBuilder::new();

    if let Some(window) = window {
        builder.set_window(window)
    } else {
        builder
    }
}

/// Creates a new PixelBuffer instance. \
/// This is not thread-safe and must be called from the same thread as the window.
///
/// # Example
/// ```rs
/// use engine::prelude::*;
///
/// let pixel_buffer = Engine::make_pixel_buffer()
///   .with_window(&mut window)
///   .build()?;
///
/// let pixels = vec![0u32; (800 * 600) as usize];
/// pixel_buffer.write_buffers(&pixels, Vector2::new(800.0, 600.0))?;
/// ```
#[cfg(feature = "software")]
pub fn create_pixel_buffer() -> PixelBufferBuilder<'static> {
    PixelBufferBuilder::new()
}

/// Queries the available GPU adapters.
///
/// This is useful for checking the available GPU adapters on the system and the supported \
/// graphics APIs.
///
/// This function can be called from any thread.
///
/// # Example
/// ```rs
/// use engine::prelude::*;
///
/// let adapters = Engine::query_gpu_adapter(None);
/// if adapters.is_empty() {
///    println!("No GPU adapters found");
/// } else {
///    println!("Found {} GPU adapters", adapters.len());
/// }
/// ```
pub fn query_gpu_adapter(window: Option<&Window>) -> Vec<GPUAdapter> {
    let mut window_arc = None;
    if let Some(window) = window {
        window_arc = Some(
            window
                .inner
                .borrow()
                .window_pointer
                .as_ref()
                .unwrap()
                .clone(),
        );
    }

    GPU::query_gpu(window_arc)
}

pub struct GPUBuilder<'a> {
    window: Option<&'a mut Window>,
    adapter: Option<&'a GPUAdapter>,
    limits: Option<Limits>,
}

impl<'a> GPUBuilder<'a> {
    pub(crate) fn new() -> Self {
        GPUBuilder {
            window: None,
            adapter: None,
            limits: None,
        }
    }

    /// Sets the window for this GPU instance.
    ///
    /// This is useful for creating a GPU instance that is bound to a specific window.
    /// The window must be created before this GPU instance.
    pub fn set_window(mut self, window: &'a mut Window) -> Self {
        self.window = Some(window);
        self
    }

    /// Sets the GPU adapter for this GPU instance.
    ///
    /// This is useful for creating a GPU instance that uses a specific GPU adapter.
    /// The adapter can be queried using the `Engine::query_gpu_adapter` function.
    pub fn set_adapter(mut self, adapter: &'a GPUAdapter) -> Self {
        self.adapter = Some(adapter);
        self
    }

    pub fn set_limits(mut self, limits: Limits) -> Self {
        self.limits = Some(limits);
        self
    }

    pub fn build(self) -> Result<GPU, String> {
        let gpu;

        if self.window.is_some() {
            let window_ref = self.window.unwrap();
            let mut window_inner = window_ref.inner.borrow_mut();

            #[cfg(feature = "software")]
            if window_inner.pixelbuffer.is_some() {
                return Err(
                    "GPU cannot be created along side PixelBuffer (software rendering)".to_string(),
                );
            }

            let window_cloned = window_inner.window_pointer.as_ref().unwrap().clone();

            gpu = futures::executor::block_on(GPU::new(window_cloned, self.adapter, self.limits))?;

            window_inner.graphics = Some(gpu.inner.clone());
        } else {
            gpu = futures::executor::block_on(GPU::new_headless(self.adapter, self.limits))?;
        }

        Ok(gpu)
    }
}

#[cfg(feature = "software")]
pub struct PixelBufferBuilder<'a> {
    window: Option<&'a mut Window>,
}

#[cfg(feature = "software")]
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

    pub fn build(self) -> Result<PixelBuffer, String> {
        if self.window.is_none() {
            return Err("PixelBuffer must be created with a window".to_string());
        }

        let window = self.window.unwrap();
        let window_inner = window.inner.borrow_mut();
        if window_inner.graphics.is_some() {
            return Err(
                "PixelBuffer cannot be created along side GPU (hardware rendering)".to_string(),
            );
        }

        drop(window_inner);

        let pixel_buffer = PixelBuffer::new(window)?;

        let mut window_inner = window.inner.borrow_mut();
        window_inner.pixelbuffer = Some(pixel_buffer.inner.clone());

        Ok(pixel_buffer)
    }
}
