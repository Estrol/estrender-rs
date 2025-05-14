use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    io::Read,
    sync::{Arc, atomic::AtomicBool},
};

use wgpu::rwh::HasWindowHandle;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event,
    event_loop::ActiveEventLoop,
    platform::windows::{CornerPreference, WindowAttributesExtWindows},
    window::{CustomCursor, CustomCursorSource, Window, WindowAttributes, WindowId},
};

use crate::{
    math::{Point, Position, Size},
    utils::ArcRef,
};

use super::{CursorIcon, CustomCursorItem};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum WindowEvent {
    Create {
        ref_id: usize,
        parent_ref_id: Option<usize>,
        title: String,
        size: Point,
        pos: Option<Point>,
    },
    Close {
        ref_id: usize,
    },
    Title {
        ref_id: usize,
        title: String,
    },
    Cursor {
        ref_id: usize,
        cursor: Option<CursorIcon>,
    },
    Size {
        ref_id: usize,
        size: Size,
    },
    Position {
        ref_id: usize,
        pos: Position,
    },
    Visible {
        ref_id: usize,
        visible: bool,
    },
    Redraw {
        ref_id: usize,
    },
}

#[derive(Clone, Debug, Hash)]
pub enum CursorSource {
    String(&'static str),
    Buffer(Vec<u8>),
}

pub struct WindowHandle {
    pub window: Arc<Window>,
    pub events: ArcRef<Vec<event::WindowEvent>>,

    pub ref_id: usize,
    pub is_closed: bool,
}

impl Drop for WindowHandle {
    fn drop(&mut self) {
        dbg!(format!("WindowHandle dropped: {:?}", self.ref_id));
    }
}

pub struct WindowInner {
    pub handles: HashMap<WindowId, WindowHandle>,
    pub last_error: Option<String>,
    pub has_redraw_requested: AtomicBool,
    pub cursor_cache: HashMap<u64, CustomCursor>,
}

impl WindowInner {
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            last_error: None,
            has_redraw_requested: AtomicBool::new(false),
            cursor_cache: HashMap::new(),
        }
    }

    pub fn get_window_by_ref(&self, ref_id: usize) -> Option<&WindowHandle> {
        self.handles
            .iter()
            .find(|(_, handle)| handle.ref_id == ref_id)
            .map(|(_, handle)| handle)
    }

    pub fn get_window_by_ref_mut(&mut self, ref_id: usize) -> Option<&mut WindowHandle> {
        self.handles
            .iter_mut()
            .find(|(_, handle)| handle.ref_id == ref_id)
            .map(|(_, handle)| handle)
    }
}

impl ApplicationHandler<WindowEvent> for WindowInner {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: event::WindowEvent,
    ) {
        if self.handles.is_empty() {
            return;
        }

        let mut to_remove = None;

        if let Some((_ref_id, _)) = self
            .handles
            .iter()
            .find(|(ref_id, _)| **ref_id == window_id)
        {
            match event {
                event::WindowEvent::CloseRequested => {
                    if self.handles.is_empty() {
                        event_loop.exit();
                        return;
                    }

                    to_remove = Some(window_id);
                }
                event::WindowEvent::RedrawRequested => {
                    if self
                        .has_redraw_requested
                        .load(std::sync::atomic::Ordering::SeqCst)
                    {
                        return;
                    }

                    self.has_redraw_requested
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
                _ => {}
            }

            if let Some(handle) = self.handles.get_mut(&window_id) {
                handle.events.borrow_mut().push(event.clone());
            }
        }

        if let Some(window_id) = to_remove {
            self.handles.remove(&window_id);
            if self.handles.is_empty() {
                event_loop.exit();
            }
        }
    }

    #[allow(unused_variables, unreachable_patterns)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::Create {
                ref_id,
                parent_ref_id,
                title,
                size,
                pos,
            } => {
                let size: PhysicalSize<u32> = PhysicalSize::new(size.x as u32, size.y as u32);
                let mut window_attributes = WindowAttributes::default()
                    .with_title(title)
                    .with_visible(true)
                    .with_inner_size(size)
                    .with_resizable(false)
                    .with_max_inner_size(size)
                    .with_min_inner_size(size)
                    .with_corner_preference(CornerPreference::DoNotRound);

                if let Some(pos) = pos {
                    let pos: PhysicalPosition<i32> =
                        PhysicalPosition::new(pos.x as i32, pos.y as i32);
                    window_attributes = window_attributes.with_position(pos);
                }

                if let Some(parent_ref_id) = parent_ref_id {
                    if let Some(parent_window) = self.get_window_by_ref(parent_ref_id) {
                        // SAFETY: We are using the `window_handle` method to get the raw window handle,
                        // which is safe as long as the window is valid and not dropped.
                        unsafe {
                            let parent_window = parent_window.window.window_handle();
                            if let Err(e) = parent_window {
                                self.last_error =
                                    Some(format!("Failed to set parent window: {:?}", e));

                                return;
                            }

                            let parent_window_handle = parent_window.unwrap().as_raw();
                            window_attributes =
                                window_attributes.with_parent_window(Some(parent_window_handle));
                        }
                    }
                }

                let window = event_loop.create_window(window_attributes);

                if let Ok(window) = window {
                    self.handles.insert(
                        window.id(),
                        WindowHandle {
                            window: Arc::new(window),
                            events: ArcRef::new(Vec::new()),
                            ref_id,
                            is_closed: false,
                        },
                    );
                } else {
                    self.last_error = Some(format!("Failed to create window: {:?}", window));
                }
            }
            WindowEvent::Close { ref_id } => {
                if self.handles.is_empty() {
                    event_loop.exit();

                    return;
                }

                let mut to_remove = None;

                for (window_id, handle) in &self.handles {
                    if handle.ref_id == ref_id {
                        to_remove = Some(*window_id);
                        break;
                    }
                }

                if let Some(window_id) = to_remove {
                    if let Some(handle) = self.handles.get_mut(&window_id) {
                        handle.is_closed = true;
                        handle.window.set_visible(false);
                    }

                    self.handles.remove(&window_id);
                }

                if self.handles.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Title { ref_id, title } => {
                let window = self.get_window_by_ref_mut(ref_id);
                if let Some(window) = window {
                    window.window.set_title(&title);
                }
            }
            WindowEvent::Size { ref_id, size } => {
                let window = self.get_window_by_ref_mut(ref_id);
                if let Some(window) = window {
                    let size: PhysicalSize<u32> = size.into();

                    window.window.set_max_inner_size(Some(size));
                    window.window.set_min_inner_size(Some(size));
                    _ = window.window.request_inner_size(size);
                }
            }
            WindowEvent::Position { ref_id, pos } => {
                let window = self.get_window_by_ref_mut(ref_id);
                if let Some(window) = window {
                    let pos: PhysicalPosition<i32> = pos.into();
                    window.window.set_outer_position(pos);
                }
            }
            WindowEvent::Visible { ref_id, visible } => {
                let window = self.get_window_by_ref_mut(ref_id);
                if let Some(window) = window {
                    window.window.set_visible(visible);
                }
            }
            WindowEvent::Redraw { ref_id } => {
                let window = self.get_window_by_ref_mut(ref_id);
                if let Some(window) = window {
                    window.window.request_redraw();
                }
            }
            WindowEvent::Cursor { ref_id, cursor } => {
                if let Some(CursorIcon::Custom(cursor)) = cursor {
                    let mut hash = std::collections::hash_map::DefaultHasher::new();
                    cursor.hash(&mut hash);
                    let hash = hash.finish();

                    if let Some(cached_cursor) = self.cursor_cache.get(&hash).cloned() {
                        if let Some(window) = self.get_window_by_ref_mut(ref_id) {
                            window.window.set_cursor(cached_cursor);
                        }
                        return;
                    }

                    let cursor = decode_cursor(cursor);
                    if let Err(e) = cursor {
                        self.last_error = Some(format!("Failed to decode cursor: {:?}", e));
                        return;
                    }

                    let cursor_src = cursor.unwrap();
                    let cursor = event_loop.create_custom_cursor(cursor_src);

                    self.cursor_cache.insert(hash, cursor.clone());

                    if let Some(window) = self.get_window_by_ref_mut(ref_id) {
                        window.window.set_cursor(cursor);
                    }
                } else {
                    if let Some(window) = self.get_window_by_ref_mut(ref_id) {
                        window.window.set_cursor(cursor.clone().unwrap());
                    }
                }
            }
            _ => {
                println!("Unhandled event: {:?}", event);
            }
        }
    }
}

fn decode_cursor(cursor: CustomCursorItem) -> Result<CustomCursorSource, String> {
    let image_src = match cursor {
        CustomCursorItem::Path(s) => {
            let file = std::fs::File::open(s).unwrap();
            let mut reader = std::io::BufReader::new(file);
            let mut buffer = Vec::new();
            let result = reader.read_to_end(&mut buffer);
            if let Err(e) = result {
                return Err(format!("Failed to read cursor file: {:?}", e));
            }

            buffer
        }
        CustomCursorItem::Image(b) => b,
    };

    let image = image::load_from_memory(&image_src);
    if let Err(e) = image {
        return Err(format!("Failed to load image: {:?}", e));
    }

    let image = image.unwrap();
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    let w = width as u16;
    let h = height as u16;

    let result = CustomCursor::from_rgba(image.into_raw(), w, h, w / 2, h / 2);

    if let Err(e) = result {
        return Err(format!("Failed to create custom cursor: {:?}", e));
    }

    let cursor = result.unwrap();
    Ok(cursor)
}
