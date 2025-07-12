use winit::{event, event_loop::EventLoopProxy};

#[cfg(feature = "software")]
use crate::software::PixelBufferInner;

use crate::{
    gpu::GPUInner, math::Point2, runner::{CursorIcon, Handle, Runner, RunnerError, WindowEvent}, utils::{ArcMut, ArcRef}
};

#[derive(Clone, Debug)]
pub struct Window {
    pub(crate) inner: ArcRef<WindowInner>,
}

impl Window {
    pub(crate) fn new(
        runner: &mut Runner,
        parent: Option<&Window>,
        title: String,
        size: Point2,
        pos: Option<Point2>,
    ) -> Result<Self, WindowError> {
        let parent_id = if let Some(parent) = parent {
            Some(parent.inner.wait_borrow().window_id)
        } else {
            None
        };

        let result = runner.internal_new_window(parent_id, title, size, pos);
        if result.is_err() {
            return Err(WindowError::RunnerError(result.unwrap_err()));
        }

        let (window_id, proxy) = result.unwrap();
        let window_events = runner.get_events_pointer(window_id);
        let window_pointer = runner.get_window_pointer(window_id);

        if window_events.is_none() || window_pointer.is_none() {
            return Err(WindowError::WindowNotFound);
        }

        let window_events = window_events.unwrap();
        let window_pointer = window_pointer.unwrap();

        let inner = ArcRef::new(WindowInner {
            window_id,
            window_events,
            window_pointer: Some(window_pointer),
            proxy,
            graphics: None,
            size: size.into(),

            #[cfg(feature = "software")]
            pixelbuffer: None,
        });

        runner.window_events_attributes.push(inner.clone());

        Ok(Self { inner })
    }

    /// Get the window ID of this window.
    ///
    /// This is a unique identifier for the window, useful
    /// for identifying the window in event handling and other operations.
    pub fn id(&self) -> usize {
        self.inner.wait_borrow().window_id
    }

    /// Get the size of the window.
    ///
    /// This useful for determining the dimensions of the window, such
    /// as when rendering content or handling layout.
    pub fn size(&self) -> Point2 {
        self.inner.wait_borrow().size
    }

    /// Send quit event to the runner to close the window.
    pub fn quit(&self) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Close {
            ref_id: inner.window_id,
        });
    }

    /// Set the title of the window.
    pub fn set_title(&mut self, title: &str) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Title {
            ref_id: inner.window_id,
            title: title.to_string(),
        });
    }

    /// Set the cursor icon for the window.
    pub fn set_cursor(&mut self, cursor: Option<CursorIcon>) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Cursor {
            ref_id: inner.window_id,
            cursor,
        });
    }

    /// Set the window size.
    pub fn set_size(&mut self, size: Point2) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Size {
            ref_id: inner.window_id,
            size: size.into(),
        });
    }

    /// Set the widnow position.
    pub fn set_position(&mut self, pos: Point2) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Position {
            ref_id: inner.window_id,
            pos: pos.into(),
        });
    }

    /// Request a redraw of the window.
    pub fn request_redraw(&mut self) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Redraw {
            ref_id: inner.window_id,
        });
    }
}

pub struct WindowBuilder<'a> {
    runner: &'a mut Runner,
    parent_window: Option<&'a Window>,
    title: String,
    size: Point2,
    pos: Option<Point2>,
}

impl<'a> WindowBuilder<'a> {
    pub(crate) fn new(runner: &'a mut Runner, title: &str, size: Point2) -> Self {
        WindowBuilder {
            runner,
            parent_window: None,
            title: title.to_string(),
            size,
            pos: None,
        }
    }

    /// Sets the title of the window.
    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    /// Sets the size of the window.
    pub fn size(mut self, size: Point2) -> Self {
        self.size = size;
        self
    }

    /// Sets the position of the window.
    pub fn pos(mut self, pos: Option<Point2>) -> Self {
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

    pub fn build(self) -> Result<Window, WindowError> {
        Window::new(
            self.runner,
            self.parent_window,
            self.title,
            self.size,
            self.pos,
        )
    }
}

pub(crate) struct WindowInner {
    pub window_id: usize,
    pub window_events: ArcRef<Vec<event::WindowEvent>>,
    pub window_pointer: Option<ArcMut<Handle>>,
    pub proxy: EventLoopProxy<WindowEvent>,
    pub size: Point2,

    pub(crate) graphics: Option<ArcRef<GPUInner>>,

    #[cfg(feature = "software")]
    pub(crate) pixelbuffer: Option<ArcRef<PixelBufferInner>>,
}

impl WindowInner {
    pub fn process_event(&mut self) {
        for event in self.window_events.wait_borrow_mut().iter() {
            match event {
                event::WindowEvent::CloseRequested => {
                    self.graphics = None;
                    self.window_pointer = None;
                }
                event::WindowEvent::Resized(size) => {
                    if let Some(gpu) = &self.graphics {
                        gpu.wait_borrow_mut().resize(*size);
                    }

                    #[cfg(feature = "software")]
                    if let Some(softbuffer) = &self.pixelbuffer {
                        _ = softbuffer.wait_borrow_mut().resize(*size);
                    }

                    self.size = Point2::from(*size);
                }
                _ => {}
            }
        }
    }
}


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunMode {
    Poll,
    ReDraw,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum WindowError {
    RunnerError(RunnerError),
    WindowNotFound,
}
