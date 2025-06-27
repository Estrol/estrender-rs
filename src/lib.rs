//! Easy to use winit, softbuffer & wgpu abstractions

/// Font rendering and text layout utilities
#[cfg(feature = "font")]
pub mod font;
/// GPU graphics rendering abstractions
pub mod gpu;
/// Mathematical utilities and types
pub mod math;
/// Predefined types and traits for easy access
pub mod prelude;
/// Runner for managing the main event loop and window lifecycle
pub mod runner;
/// Software rendering utilities
#[cfg(feature = "software")]
pub mod software;
/// Utility functions and types for common tasks
pub mod utils;
/// Window management
pub mod window;

use gpu::{GPU, GPUAdapter};
use runner::Runner;
use window::Window;

#[cfg(feature = "font")]
use crate::font::FontManager;
use crate::gpu::GPUBuilder;

#[cfg(feature = "software")]
use crate::software::PixelBufferBuilder;

/// Create a [Runner] instance, required for creating one or more windows.
///
/// **NOTE:** When calling this function, the thread will be made the main thread,
/// future calls to this function will panic if called from a different thread.
pub fn create_runner() -> Result<Runner, String> {
    Runner::new()
}

/// Creates a new [GPU] instance.
///
/// This is thread-safe and can be called from any thread, except when using
/// the [GPUBuilder::set_window] method, which binds the GPU to the window's thread.
pub fn create_gpu<'a>(window: Option<&'a mut Window>) -> GPUBuilder<'a> {
    let builder = GPUBuilder::new();

    if let Some(window) = window {
        builder.set_window(window)
    } else {
        builder
    }
}

/// Creates a new [software::PixelBuffer] instance. \
/// This is not thread-safe and must be called from the same thread as the window.
#[cfg(feature = "software")]
pub fn create_pixel_buffer<'a>(window: Option<&'a mut Window>) -> PixelBufferBuilder<'a> {
    let builder = PixelBufferBuilder::new();

    if let Some(window) = window {
        builder.with_window(window)
    } else {
        builder
    }
}

/// Queries the available GPU's [GPUAdapter].
///
/// This is useful for checking the available GPU adapters on the system and the supported \
/// graphics APIs, allowing you to choose the best GPU and graphics API for your application.
///
/// This function can be called from any thread.
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

/// Creates a new [FontManager] instance.
///
/// This is useful for loading and managing fonts for text rendering.
#[cfg(feature = "font")]
pub fn create_font_manager() -> FontManager {
    FontManager::new()
}
