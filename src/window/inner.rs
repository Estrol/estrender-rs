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
    window::{CustomCursor, CustomCursorSource, Window, WindowAttributes, WindowId},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};

use crate::{
    dbg_log,
    math::{Point2, Position, Size},
    utils::{ArcMut, ArcRef},
};

use super::{CursorIcon, CustomCursorItem};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum WindowEvent {
    Create {
        ref_id: usize,
        parent_ref_id: Option<usize>,
        title: String,
        size: Point2,
        pos: Option<Point2>,
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

// #[derive(Clone, Debug, Hash)]
// pub enum CursorSource {
//     String(&'static str),
//     Buffer(Vec<u8>),
// }

#[derive(Clone, Debug)]
pub struct Handle {
    pub window: Option<Arc<Window>>,
    pub is_closed: bool,
    pub is_pinned: bool,
}

#[allow(dead_code)]
impl Handle {
    pub fn new(window: Arc<Window>) -> Self {
        Self {
            window: Some(window),
            is_closed: false,
            is_pinned: false,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    pub fn close(&mut self) {
        self.window = None;
        self.is_closed = true;
    }

    pub fn set_window(&mut self, window: Option<Arc<Window>>) {
        self.window = window;
    }

    pub fn get_window(&self) -> &Arc<Window> {
        if self.is_closed {
            panic!("Window is closed");
        }

        self.window.as_ref().unwrap()
    }

    pub fn get_window_id(&self) -> WindowId {
        if self.is_closed {
            panic!("Window is closed");
        }

        self.window.as_ref().unwrap().id()
    }

    pub fn is_pinned(&self) -> bool {
        self.is_pinned
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        self.is_pinned = pinned;
    }
}

pub struct WindowHandle {
    pub window: ArcMut<Handle>,
    pub events: ArcRef<Vec<event::WindowEvent>>,

    pub ref_id: usize,
}

impl Drop for WindowHandle {
    fn drop(&mut self) {
        dbg_log!("WindowHandle dropped: {:?}", self.ref_id);
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

    pub fn get_window_handle_by_ref(&self, ref_id: usize) -> Option<ArcMut<Handle>> {
        self.handles
            .iter()
            .find(|(_, handle)| handle.ref_id == ref_id)
            .map(|(_, handle)| handle.window.clone())
    }

    pub fn get_window_events_by_ref(
        &self,
        ref_id: usize,
    ) -> Option<ArcRef<Vec<event::WindowEvent>>> {
        self.handles
            .iter()
            .find(|(_, handle)| handle.ref_id == ref_id)
            .map(|(_, handle)| handle.events.clone())
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
                    .with_min_inner_size(size);

                #[cfg(target_os = "windows")]
                {
                    window_attributes = window_attributes.with_corner_preference(
                        CornerPreference::DoNotRound,
                    );
                }

                if let Some(pos) = pos {
                    let pos: PhysicalPosition<i32> =
                        PhysicalPosition::new(pos.x as i32, pos.y as i32);
                    window_attributes = window_attributes.with_position(pos);
                }

                if let Some(parent_ref_id) = parent_ref_id {
                    if let Some(parent_window) = self.get_window_handle_by_ref(parent_ref_id) {
                        let parent_window = parent_window.lock();

                        // SAFETY: We are using the `window_handle` method to get the raw window handle,
                        // which is safe as long as the window is valid and not dropped.
                        unsafe {
                            if parent_window.is_closed() {
                                self.last_error = Some(format!(
                                    "Parent window is None for ref_id: {}",
                                    parent_ref_id
                                ));
                                return;
                            }

                            let parent_window = parent_window.get_window().window_handle();

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
                    let window_id = window.id();
                    let handle = Handle::new(Arc::new(window));

                    let window_handle = WindowHandle {
                        window: ArcMut::new(handle),
                        events: ArcRef::new(Vec::new()),
                        ref_id,
                    };

                    dbg_log!("Window {} created", ref_id);
                    self.handles.insert(window_id, window_handle);
                } else {
                    dbg_log!("Failed to create window: {:?}", window);
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
                        handle.window.lock().close();
                    }

                    dbg_log!("Window {} closed", ref_id);
                    self.handles.remove(&window_id);
                }

                if self.handles.is_empty() {
                    dbg_log!("All windows closed, exiting event loop");
                    event_loop.exit();
                }
            }
            WindowEvent::Title { ref_id, title } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let window = handle.lock();
                    let window = window.get_window();

                    dbg_log!("Window {} title: {}", ref_id, title);

                    window.set_title(title.as_str());
                }
            }
            WindowEvent::Size { ref_id, size } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let size: PhysicalSize<u32> = size.into();

                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    dbg_log!("Window {} size: {:?}", ref_id, size);

                    window.set_max_inner_size(Some(size));
                    window.set_min_inner_size(Some(size));
                    _ = window.request_inner_size(size);
                }
            }
            WindowEvent::Position { ref_id, pos } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let pos: PhysicalPosition<i32> = pos.into();
                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    dbg_log!("Window {} position: {:?}", ref_id, pos);
                    window.set_outer_position(pos);
                }
            }
            WindowEvent::Visible { ref_id, visible } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    dbg_log!("Window {} visible: {}", ref_id, visible);
                    window.set_visible(visible);
                }
            }
            WindowEvent::Redraw { ref_id } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    window.request_redraw();
                }
            }
            WindowEvent::Cursor { ref_id, cursor } => {
                if let Some(CursorIcon::Custom(cursor)) = cursor {
                    let mut hash = std::collections::hash_map::DefaultHasher::new();
                    cursor.hash(&mut hash);
                    let hash = hash.finish();

                    if let Some(cached_cursor) = self.cursor_cache.get(&hash).cloned() {
                        if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                            let handle_ref = handle.lock();
                            let window = handle_ref.get_window();

                            window.set_cursor(cached_cursor.clone());
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

                    if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                        let handle_ref = handle.lock();
                        let window = handle_ref.get_window();

                        window.set_cursor(cursor.clone());
                    }
                } else {
                    if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                        let handle_ref = handle.lock();
                        let window = handle_ref.get_window();

                        window.set_cursor(cursor.clone().unwrap());
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
