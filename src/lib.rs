pub mod graphics;
pub mod math;
pub mod prelude;
pub mod utils;
pub mod window;

use graphics::{GPU, GPUAdapter};
use math::Point;
pub use window::Window;
use window::{input::Input, runner::Runner};

#[cfg(feature = "software")]
use window::pixel_buffer::PixelBuffer;

pub struct RenderEngine;

impl RenderEngine {
    /// Create a EventLoop instance, required for creating one or more windows. \
    /// This will make the thread caller the main thread.
    ///
    /// # Example
    /// ```rs
    /// use engine::prelude::*;
    ///
    /// let mut event_loop = Engine::make_event_loop();
    /// ```
    pub fn make_runner() -> Result<Runner, String> {
        Runner::new()
    }

    /// Creates a new window with the given title, size, and position. \
    /// **NOTE:** This function will make the thread caller the main thread.
    /// It will panic if called from a different thread after the first call.
    ///
    /// # Example
    /// ```rs
    /// use engine::prelude::*;
    ///
    /// let mut window = Engine::make_window("Hello", Size::new(800, 600), Position::new(100, 100))
    ///    .build();
    /// ```
    pub fn make_window(title: &str, size: Point, pos: Option<Point>) -> WindowBuilder {
        WindowBuilder::new(title, size, pos)
    }

    /// Creates a new GPU instance. \
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
    pub fn make_gpu() -> GpuBuilder<'static> {
        GpuBuilder::new()
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
    pub fn make_pixel_buffer() -> PixelBufferBuilder<'static> {
        PixelBufferBuilder::new()
    }

    /// Queries the available GPU adapters. \
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

    /// Creates a new Input instance. \
    /// This is not thread-safe and must be called from the same thread as the window.
    ///
    /// # Example
    /// ```rs
    /// use engine::prelude::*;
    ///
    /// let input = Engine::make_input()
    ///   .with_runner(&mut runner)
    ///   .with_window(&mut window)
    ///   .build()?;
    ///
    /// if input.is_key_pressed("A") {
    ///    println!("Key A is pressed");
    /// }
    /// ```
    pub fn make_input() -> InputBuilder<'static> {
        InputBuilder::new()
    }
}

pub struct WindowBuilder<'a> {
    parent_window: Option<&'a Window>,
    title: String,
    size: Point,
    pos: Option<Point>,
}

impl<'a> WindowBuilder<'a> {
    pub(crate) fn new(title: &str, size: Point, pos: Option<Point>) -> WindowBuilder {
        WindowBuilder {
            parent_window: None,
            title: title.to_string(),
            size,
            pos,
        }
    }

    /// Sets the title of the window.
    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    /// Sets the size of the window.
    pub fn size(mut self, size: Point) -> Self {
        self.size = size;
        self
    }

    /// Sets the position of the window.
    pub fn pos(mut self, pos: Option<Point>) -> Self {
        self.pos = pos;
        self
    }

    /// Sets the parent window for this window. \
    /// This is useful for creating child windows or popups.
    /// The parent window must be created before this window.
    pub fn with_parent_window(mut self, parent: &'a Window) -> Self {
        self.parent_window = Some(parent);
        self
    }

    pub fn build(self, runner: &mut Runner) -> Result<Window, String> {
        Window::new(runner, self.parent_window, self.title, self.size, self.pos)
    }
}

pub struct GpuBuilder<'a> {
    window: Option<&'a mut Window>,
    adapter: Option<&'a GPUAdapter>,
}

impl<'a> GpuBuilder<'a> {
    pub(crate) fn new() -> Self {
        GpuBuilder {
            window: None,
            adapter: None,
        }
    }

    /// Sets the window for this GPU instance. \
    /// This is useful for creating a GPU instance that is bound to a specific window.
    /// The window must be created before this GPU instance.
    pub fn with_window(mut self, window: &'a mut Window) -> Self {
        self.window = Some(window);
        self
    }

    /// Sets the GPU adapter for this GPU instance. \
    /// This is useful for creating a GPU instance that uses a specific GPU adapter.
    /// The adapter can be queried using the `Engine::query_gpu_adapter` function.
    pub fn with_adapter(mut self, adapter: &'a GPUAdapter) -> Self {
        self.adapter = Some(adapter);
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

            gpu = futures::executor::block_on(GPU::new(window_cloned, self.adapter))?;

            window_inner.graphics = Some(gpu.inner.clone());
        } else {
            gpu = futures::executor::block_on(GPU::new_headless(self.adapter))?;
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

pub struct InputBuilder<'a> {
    runner: Option<&'a mut Runner>,
    window: Option<&'a mut Window>,
}

impl<'a> InputBuilder<'a> {
    pub(crate) fn new() -> Self {
        InputBuilder {
            runner: None,
            window: None,
        }
    }

    /// Sets the runner for this Input instance.
    pub fn with_runner(mut self, runner: &'a mut Runner) -> Self {
        self.runner = Some(runner);
        self
    }

    /// Sets the window for this Input instance.
    pub fn with_window(mut self, window: &'a mut Window) -> Self {
        self.window = Some(window);
        self
    }

    pub fn build(self) -> Result<Input, String> {
        if self.runner.is_none() {
            return Err("Input must be created with a runner".to_string());
        }

        if self.window.is_none() {
            return Err("Input must be created with a window".to_string());
        }

        let input = Input::new(self.runner.unwrap(), self.window.unwrap());

        Ok(input)
    }
}
