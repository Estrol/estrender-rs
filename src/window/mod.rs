use std::sync::Arc;

use winit::{event, event_loop::EventLoopProxy, window::Cursor};

use crate::{
    graphics::inner::GPUInner,
    math::{Point, Position, Size, Timing},
    utils::ArcRef,
};

mod inner;
pub mod input;
pub mod pixel_buffer;
pub mod runner;

pub use inner::*;
pub use input::*;
pub use runner::*;

#[cfg(feature = "software")]
pub use pixel_buffer::*;

#[cfg(feature = "software")]
use pixel_buffer::PixelBufferInner;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunMode {
    Poll,
    ReDraw,
}

// type RedrawCallback = Box<dyn FnMut()>;

pub struct InnerAttribute {
    pub window_id: usize,
    pub window_events: ArcRef<Vec<event::WindowEvent>>,
    pub window_pointer: Option<Arc<winit::window::Window>>,
    pub proxy: EventLoopProxy<WindowEvent>,

    pub(crate) graphics: Option<ArcRef<GPUInner>>,

    #[cfg(feature = "software")]
    pub(crate) pixelbuffer: Option<ArcRef<PixelBufferInner>>,
}

impl InnerAttribute {
    pub fn process_event(&mut self) {
        for event in self.window_events.wait_borrow_mut().iter() {
            match event {
                event::WindowEvent::CloseRequested => {
                    if let Some(gpu) = &self.graphics {
                        gpu.wait_borrow_mut().destroy();
                    }

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
                }
                _ => {}
            }
        }
    }
}

pub struct Window {
    pub(crate) inner: ArcRef<InnerAttribute>,
    pub(crate) timing: Timing,
}

impl Window {
    pub fn new(
        runner: &mut Runner,
        parent: Option<&Window>,
        title: String,
        size: Point,
        pos: Option<Point>,
    ) -> Result<Self, String> {
        let parent_id = if let Some(parent) = parent {
            Some(parent.inner.wait_borrow().window_id)
        } else {
            None
        };

        let result = runner.new_window(parent_id, title, size, pos);
        if result.is_err() {
            return Err(result.err().unwrap());
        }

        let (window_id, proxy) = result.unwrap();
        let window_events = runner.get_events_pointer(window_id);
        let window_pointer = runner.get_window_pointer(window_id);

        if window_events.is_none() || window_pointer.is_none() {
            return Err("Failed to create window!".to_string());
        }

        let window_events = window_events.unwrap();
        let window_pointer = window_pointer.unwrap();

        let inner = ArcRef::new(InnerAttribute {
            window_id,
            window_events,
            window_pointer: Some(window_pointer),
            proxy,
            graphics: None,

            #[cfg(feature = "software")]
            pixelbuffer: None,
        });

        runner.window_events_attributes.push(inner.clone());

        Ok(Self {
            inner,
            timing: Timing::new(60),
        })
    }

    pub fn quit(&self) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Close {
            ref_id: inner.window_id,
        });
    }

    pub fn set_fps(&mut self, fps: u32) {
        self.timing.set_fps(fps);
    }

    pub fn get_fps(&self) -> u32 {
        self.timing.get_fps()
    }

    pub fn get_frame_time(&self) -> f64 {
        self.timing.get_frame_time()
    }

    pub fn set_title(&mut self, title: &str) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Title {
            ref_id: inner.window_id,
            title: title.to_string(),
        });
    }

    pub fn set_cursor(&mut self, cursor: Option<CursorIcon>) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Cursor {
            ref_id: inner.window_id,
            cursor,
        });
    }

    /// Set the size of the window. \
    /// This will resize the window to the specified size.
    pub fn set_size(&mut self, size: Size) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Size {
            ref_id: inner.window_id,
            size: size.into(),
        });
    }

    /// Set the position of the window. \
    /// This will move the window to the specified position.
    pub fn set_position(&mut self, pos: Position) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Position {
            ref_id: inner.window_id,
            pos: pos.into(),
        });
    }

    /// Request a redraw of the window. \
    /// This is useful when you want to update the window's content.
    pub fn request_redraw(&mut self) {
        let inner = self.inner.wait_borrow();

        _ = inner.proxy.send_event(WindowEvent::Redraw {
            ref_id: inner.window_id,
        });
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CursorIcon {
    Default,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,

    Custom(CustomCursorItem),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CustomCursorItem {
    Path(String),
    Image(Vec<u8>),
}

impl Into<Cursor> for CursorIcon {
    fn into(self) -> Cursor {
        match self {
            CursorIcon::Default => Cursor::Icon(winit::window::CursorIcon::Default),
            CursorIcon::ContextMenu => Cursor::Icon(winit::window::CursorIcon::ContextMenu),
            CursorIcon::Help => Cursor::Icon(winit::window::CursorIcon::Help),
            CursorIcon::Pointer => Cursor::Icon(winit::window::CursorIcon::Pointer),
            CursorIcon::Progress => Cursor::Icon(winit::window::CursorIcon::Progress),
            CursorIcon::Wait => Cursor::Icon(winit::window::CursorIcon::Wait),
            CursorIcon::Cell => Cursor::Icon(winit::window::CursorIcon::Cell),
            CursorIcon::Crosshair => Cursor::Icon(winit::window::CursorIcon::Crosshair),
            CursorIcon::Text => Cursor::Icon(winit::window::CursorIcon::Text),
            CursorIcon::VerticalText => Cursor::Icon(winit::window::CursorIcon::VerticalText),
            CursorIcon::Alias => Cursor::Icon(winit::window::CursorIcon::Alias),
            CursorIcon::Copy => Cursor::Icon(winit::window::CursorIcon::Copy),
            CursorIcon::Move => Cursor::Icon(winit::window::CursorIcon::Move),
            CursorIcon::NoDrop => Cursor::Icon(winit::window::CursorIcon::NoDrop),
            CursorIcon::NotAllowed => Cursor::Icon(winit::window::CursorIcon::NotAllowed),
            CursorIcon::Grab => Cursor::Icon(winit::window::CursorIcon::Grab),
            CursorIcon::Grabbing => Cursor::Icon(winit::window::CursorIcon::Grabbing),
            CursorIcon::EResize => Cursor::Icon(winit::window::CursorIcon::EResize),
            CursorIcon::NResize => Cursor::Icon(winit::window::CursorIcon::NResize),
            CursorIcon::NeResize => Cursor::Icon(winit::window::CursorIcon::NeResize),
            CursorIcon::NwResize => Cursor::Icon(winit::window::CursorIcon::NwResize),
            CursorIcon::SResize => Cursor::Icon(winit::window::CursorIcon::SResize),
            CursorIcon::SeResize => Cursor::Icon(winit::window::CursorIcon::SeResize),
            CursorIcon::SwResize => Cursor::Icon(winit::window::CursorIcon::SwResize),
            CursorIcon::WResize => Cursor::Icon(winit::window::CursorIcon::WResize),
            CursorIcon::EwResize => Cursor::Icon(winit::window::CursorIcon::EwResize),
            CursorIcon::NsResize => Cursor::Icon(winit::window::CursorIcon::NsResize),
            CursorIcon::NeswResize => Cursor::Icon(winit::window::CursorIcon::NeswResize),
            CursorIcon::NwseResize => Cursor::Icon(winit::window::CursorIcon::NwseResize),
            CursorIcon::ColResize => Cursor::Icon(winit::window::CursorIcon::ColResize),
            CursorIcon::RowResize => Cursor::Icon(winit::window::CursorIcon::RowResize),
            CursorIcon::AllScroll => Cursor::Icon(winit::window::CursorIcon::AllScroll),
            CursorIcon::ZoomIn => Cursor::Icon(winit::window::CursorIcon::ZoomIn),
            CursorIcon::ZoomOut => Cursor::Icon(winit::window::CursorIcon::ZoomOut),
            CursorIcon::Custom(_) => panic!("This should not handled here!"),
        }
    }
}
