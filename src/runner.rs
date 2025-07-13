use std::{
    collections::HashMap, hash::{Hash, Hasher}, io::Read, sync::{atomic::{AtomicBool, AtomicUsize}, Arc, Mutex}, thread::ThreadId, time::Duration
};

use crate::{math::{Point2, Timing}, utils::{ArcMut, ArcRef}, window::{WindowInner, WindowBuilder}};

use smol_str::SmolStr;
use wgpu::rwh::HasWindowHandle;
use winit::{
    application::ApplicationHandler, dpi::{PhysicalPosition, PhysicalSize}, event, event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy}, keyboard::{Key, NamedKey, NativeKey}, platform::pump_events::{EventLoopExtPumpEvents, PumpStatus}, window::{Cursor, CustomCursor, CustomCursorSource, Window as WinitWindow, WindowAttributes, WindowId}
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

#[cfg(all(not(feature = "x11"), target_os = "linux"))]
use winit::platform::wayland::EventLoopBuilderExtWayland;

#[cfg(all(feature = "x11", target_os = "linux"))]
use winit::platform::x11::EventLoopBuilderExtX11;

/// Create a [Runner] instance, required for creating one or more windows.
///
/// **NOTE:** When calling this function, the thread will be made the main thread,
/// future calls to this function will panic if called from a different thread.
pub fn new() -> Result<Runner, RunnerError> {
    Runner::new()
}

// This is the most laziest workaround to able construct multiple event loops
// in the same process, but only one at a time.
//
// However, this will lock the event loop to the thread that created it, and
// will panic if called from a different thread after the first call.
lazy_static::lazy_static! {
    static ref CURRENT_LOOP_THREAD_ID: Mutex<Option<ThreadId>> = Mutex::new(None);
    static ref CURRENT_LOOP: Mutex<Option<EventLoopWrapper>> = Mutex::new(None);
    static ref CURRENT_WINDOW_ID: AtomicUsize = AtomicUsize::new(0);
}

pub(crate) struct EventLoopWrapper {
    pub event_loop: ArcRef<EventLoop<WindowEvent>>,
}

// This needed for global access to the event loop
// But the actual uses, still limit it to the callers thread
// This is a workaround for the fact that winit's EventLoop is not Send or Sync
unsafe impl Sync for EventLoopWrapper {}
unsafe impl Send for EventLoopWrapper {}

/// Provide almost cross-platform event loop for the application.
///
/// This wrap winit's [EventLoop] and provides a way to create windows and handle events.
/// But with some limitations:
/// - No support for iOS and WASM platforms.
/// - macOS platform have to use [PollMode::WaitDraw] or drawing at event [Event::RedrawRequested] because
/// how winit setup the window drawing on macOS.
#[allow(dead_code)]
pub struct Runner {
    pub(crate) app_runner: RunnerInner,
    pub(crate) event_loop: ArcRef<EventLoop<WindowEvent>>,
    pub(crate) event_loop_proxy: EventLoopProxy<WindowEvent>,
    pub(crate) window_events_attributes: Vec<ArcRef<WindowInner>>,
    pub(crate) rate_timing: Timing,
    pub(crate) pending_events: Vec<Event>,
}

impl Runner {
    pub(crate) fn new() -> Result<Self, RunnerError> {
        let thread_id = std::thread::current().id();

        if CURRENT_LOOP_THREAD_ID.lock().unwrap().is_none() {
            *CURRENT_LOOP_THREAD_ID.lock().unwrap() = Some(thread_id);
        } else if CURRENT_LOOP_THREAD_ID.lock().unwrap().as_ref() != Some(&thread_id) {
            return Err(RunnerError::ThreadMissmatch);
        }

        let event_loop = if let Some(current_loop) = CURRENT_LOOP.lock().unwrap().as_ref() {
            current_loop.event_loop.clone()
        } else {
            let event_loop_result = std::panic::catch_unwind(|| {
                let mut event_loop_builder = EventLoop::<WindowEvent>::with_user_event();

                #[cfg(any(target_os = "windows", target_os = "linux"))]
                {
                    event_loop_builder.with_any_thread(true);
                }

                event_loop_builder.build()
            });

            // Winit panic if the event loop is already created in another? thread.
            if event_loop_result.is_err() {
                *CURRENT_LOOP_THREAD_ID.lock().unwrap() = None;

                return Err(RunnerError::WinitEventLoopPanic);
            }

            // If the event loop creation failed, we return an error.
            let event_loop_result = event_loop_result.unwrap();
            if event_loop_result.is_err() {
                *CURRENT_LOOP_THREAD_ID.lock().unwrap() = None;

                return Err(RunnerError::WinitEventLoopFailed);
            }

            let event_loop_result = ArcRef::new(event_loop_result.unwrap());
            *CURRENT_LOOP.lock().unwrap() = Some(EventLoopWrapper {
                event_loop: event_loop_result.clone(),
            });

            event_loop_result
        };

        let event_loop_proxy = {
            let event_loop = event_loop.wait_borrow_mut();
            event_loop.create_proxy()
        };

        Ok(Self {
            app_runner: RunnerInner::new(),
            event_loop,
            event_loop_proxy,
            window_events_attributes: Vec::new(),
            rate_timing: Timing::new(0),
            pending_events: Vec::new(),
        })
    }

    /// Returns the pending events that have been processed by the event loop in [Runner::pool_events].
    pub fn get_events(&self) -> &Vec<Event> {
        &self.pending_events
    }

    /// Creates a new [WindowBuilder] instance to build a new window.
    pub fn create_window(&mut self, title: &str, size: Point2) -> WindowBuilder {
        WindowBuilder::new(self, title, size)
    }

    /// This called from [WindowBuilder] to create a new window.
    pub(crate) fn internal_new_window(
        &mut self,
        parent: Option<usize>,
        title: String,
        size: Point2,
        pos: Option<Point2>,
    ) -> Result<(usize, EventLoopProxy<WindowEvent>), RunnerError> {
        let mut event_loop = self.event_loop.wait_borrow_mut();
        let event_loop_proxy = event_loop.create_proxy();

        let window_id = CURRENT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if window_id >= 1000 {
            // return Err("Maximum window reached!".to_string());
            return Err(RunnerError::MaximumWindowReached);
        }

        let res = event_loop_proxy.send_event(WindowEvent::Create {
            ref_id: window_id,
            parent_ref_id: parent,
            title,
            size,
            pos,
        });

        if res.is_err() {
            let err = self
                .app_runner
                .last_error
                .clone()
                .unwrap_or_else(|| "Failed to create window!".to_string());

            return Err(RunnerError::FailedToCreateWindow(err));
        }

        event_loop.pump_app_events(Some(Duration::ZERO), &mut self.app_runner);

        let mut found = false;
        for (_id, handle) in self.app_runner.handles.iter() {
            if handle.ref_id == window_id {
                found = true;
                break;
            }
        }

        if !found {
            let err = self
                .app_runner
                .last_error
                .clone()
                .unwrap_or_else(|| "Failed to create window!".to_string());

            return Err(RunnerError::FailedToCreateWindow(err));
        }

        Ok((window_id, event_loop_proxy))
    }

    /// Pump the event loop and process events.
    ///
    /// This method will block based on the provided `mode`.
    /// - [PollMode::Poll] will return immediately if there are no events.
    /// - [PollMode::Wait] will block until an event is available.
    /// - [PollMode::WaitDraw] will block until a redraw is requested (Recommended for MacOS platform).
    ///
    /// You can also pass [None] to use the default behavior, which is equivalent to `PollMode::Poll`.
    ///
    /// After calling this method, you can access the processed events using the [Runner::get_events] method.
    ///
    /// # Incompatible platforms
    /// - iOS: This method is not supported on iOS due to platform limitations.
    /// - WASM: This method is not supported on WASM due to how the browser handles events, unless
    /// you using the emscripten event loop.
    pub fn pool_events<T>(&mut self, mode: T) -> bool
    where
        T: Into<Option<PollMode>>,
    {
        let mut event_loop = self.event_loop.wait_borrow_mut();
        let mode = mode.into();

        let duration = match mode {
            Some(PollMode::Poll) => Some(Duration::ZERO),
            Some(PollMode::Wait) => None,
            Some(PollMode::WaitDraw) => None,
            None => Some(Duration::ZERO),
        };

        let wait_for_redraw = match mode {
            Some(PollMode::WaitDraw) => true,
            _ => false,
        };

        self.pending_events.clear();

        loop {
            match event_loop.pump_app_events(duration, &mut self.app_runner) {
                PumpStatus::Continue => {
                    for window in self.window_events_attributes.iter() {
                        if let Some(mut window) = window.try_borrow_mut() {
                            window.process_event();

                            {
                                let window_events = window.window_events.wait_borrow_mut();
                                for event in window_events.iter() {
                                    match event {
                                        event::WindowEvent::CloseRequested => {
                                            self.pending_events.push(Event::WindowClosed {
                                                window_id: window.window_id,
                                            });
                                        }
                                        event::WindowEvent::Resized(size) => {
                                            self.pending_events.push(Event::WindowResized {
                                                window_id: window.window_id,
                                                size: Point2::new(size.width, size.height),
                                            });
                                        }
                                        event::WindowEvent::Moved(pos) => {
                                            self.pending_events.push(Event::WindowMoved {
                                                window_id: window.window_id,
                                                pos: Point2::new(pos.x, pos.y),
                                            });
                                        }
                                        event::WindowEvent::RedrawRequested => {
                                            self.pending_events.push(Event::RedrawRequested {
                                                window_id: window.window_id,
                                            });
                                        }
                                        event::WindowEvent::KeyboardInput {
                                            event,
                                            is_synthetic,
                                            ..
                                        } => {
                                            if *is_synthetic {
                                                continue;
                                            }

                                            let is_pressed =
                                                event.state == event::ElementState::Pressed;

                                            match event.logical_key {
                                                Key::Character(ref smol_str) => {
                                                    let smol_key = smol_str.clone();

                                                    self.pending_events.push(Event::KeyboardInput {
                                                        window_id: window.window_id,
                                                        key: smol_key,
                                                        pressed: is_pressed,
                                                    });
                                                }
                                                Key::Named(ref named_key) => {
                                                    let smol_key = named_key_to_str(named_key);
                                                    if smol_key.is_none() {
                                                        continue;
                                                    }

                                                    let smol_key = smol_key.unwrap();

                                                    self.pending_events.push(Event::KeyboardInput {
                                                        window_id: window.window_id,
                                                        key: smol_key,
                                                        pressed: is_pressed,
                                                    });
                                                }
                                                Key::Unidentified(NativeKey::Windows(virtual_key)) => {
                                                    let fmt = format!("virtual-key:{:?}", virtual_key);
                                                    let smol_key = SmolStr::new(fmt);

                                                    self.pending_events.push(Event::KeyboardInput {
                                                        window_id: window.window_id,
                                                        key: smol_key,
                                                        pressed: is_pressed,
                                                    });
                                                }
                                                _ => {
                                                    // ignore
                                                }
                                            }
                                        }
                                        event::WindowEvent::MouseWheel {
                                            delta, phase: _, ..
                                        } => {
                                            let delta = match delta {
                                                event::MouseScrollDelta::LineDelta(
                                                    delta_x,
                                                    delta_y,
                                                ) => MouseScrollDelta::LineDelta {
                                                    delta_x: *delta_x,
                                                    delta_y: *delta_y,
                                                },
                                                event::MouseScrollDelta::PixelDelta(delta_pos) => {
                                                    MouseScrollDelta::PixelDelta {
                                                        delta_x: delta_pos.x as f32,
                                                        delta_y: delta_pos.y as f32,
                                                    }
                                                }
                                            };

                                            self.pending_events.push(Event::MouseWheel {
                                                window_id: window.window_id,
                                                delta,
                                            });
                                        }
                                        event::WindowEvent::MouseInput {
                                            device_id: _,
                                            state,
                                            button,
                                        } => {
                                            let is_pressed = *state == event::ElementState::Pressed;
                                            let smoll_str = match button {
                                                event::MouseButton::Left => SmolStr::new("Left"),
                                                event::MouseButton::Right => SmolStr::new("Right"),
                                                event::MouseButton::Middle => SmolStr::new("Middle"),
                                                event::MouseButton::Back => SmolStr::new("Back"),
                                                event::MouseButton::Forward => SmolStr::new("Forward"),
                                                event::MouseButton::Other(_) => continue, // Ignore other buttons
                                            };

                                            self.pending_events.push(Event::MouseInput {
                                                window_id: window.window_id,
                                                button: smoll_str,
                                                pressed: is_pressed,
                                            });
                                        }
                                        event::WindowEvent::CursorEntered { device_id: _ } => {
                                            self.pending_events.push(Event::CursorEntered {
                                                window_id: window.window_id,
                                            });
                                        }
                                        event::WindowEvent::CursorLeft { device_id: _ } => {
                                            self.pending_events.push(Event::CursorLeft {
                                                window_id: window.window_id,
                                            });
                                        }
                                        event::WindowEvent::CursorMoved {
                                            device_id: _,
                                            position,
                                        } => {
                                            self.pending_events.push(Event::CursorMoved {
                                                window_id: window.window_id,
                                                pos: Point2::new(position.x, position.y),
                                            });
                                        }
                                        event::WindowEvent::Focused(focused) => {
                                            self.pending_events.push(Event::WindowFocused {
                                                window_id: window.window_id,
                                                focused: *focused,
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            window.cycle();
                        }
                    }
                }
                PumpStatus::Exit(_code) => {
                    // Exit the event loop
                    crate::dbg_log!("Event loop exited with code: {}", _code);

                    return false;
                }
            }

            for window in self.window_events_attributes.iter() {
                window.wait_borrow().window_events.wait_borrow_mut().clear();
            }

            if wait_for_redraw {
                if self
                    .app_runner
                    .has_redraw_requested
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    break;
                }
            } else {
                break;
            }
        }

        drop(event_loop);

        self.rate_timing.sleep();

        true
    }

    /// Set the rate (frame rate) for the event loop.
    ///
    /// This only useful if you want to control the frame rate of the event loop.
    /// Not effective if you use `PollMode::Wait` or `PollMode::WaitDraw`, or multi
    /// window mode, or multiple threads.
    pub fn set_rate(&mut self, rate: Option<Duration>) {
        let rate = {
            if let Some(rate) = rate {
                1.0 / (rate.as_secs_f64() * 1000.0)
            } else {
                0.0
            }
        };

        self.rate_timing.set_fps(rate as u32);
    }

    /// Set the target frames per second (FPS) for the event loop.
    ///
    /// This only useful if you want to control the frame rate of the event loop.
    /// Not effective if you use `PollMode::Wait` or `PollMode::WaitDraw`, or multi
    /// window mode, or multiple threads.
    pub fn set_target_fps(&mut self, fps: u32) {
        self.rate_timing.set_fps(fps);
    }

    /// Get the current frame rate (FPS) of the event loop.
    ///
    /// This only useful if you want to control the frame rate of the event loop.
    /// Not effective if you use `PollMode::Wait` or `PollMode::WaitDraw`, or multi
    /// window mode, or multiple threads.
    pub fn get_target_fps(&self) -> u32 {
        self.rate_timing.get_fps()
    }

    /// Get the time taken for each frame in milliseconds.
    ///
    /// This only useful if you want to control the frame rate of the event loop.
    /// Not effective if you use `PollMode::Wait` or `PollMode::WaitDraw`, or multi
    /// window mode, or multiple threads.
    pub fn get_frame_time(&self) -> f64 {
        self.rate_timing.get_frame_time()
    }

    pub(crate) fn get_events_pointer(
        &self,
        window_id: usize,
    ) -> Option<ArcRef<Vec<event::WindowEvent>>> {
        self.app_runner.get_window_events_by_ref(window_id)
    }

    pub(crate) fn get_window_pointer(&self, window_id: usize) -> Option<ArcMut<Handle>> {
        self.app_runner.get_window_handle_by_ref(window_id)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Handle {
    pub window: Option<Arc<WinitWindow>>,
    pub is_closed: bool,
    pub is_pinned: bool,
}

#[allow(dead_code)]
impl Handle {
    pub fn new(window: Arc<WinitWindow>) -> Self {
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

    pub fn set_window(&mut self, window: Option<Arc<WinitWindow>>) {
        self.window = window;
    }

    pub fn get_window(&self) -> &Arc<WinitWindow> {
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

pub(crate) struct WindowHandle {
    pub window: ArcMut<Handle>,
    pub events: ArcRef<Vec<event::WindowEvent>>,

    pub ref_id: usize,
}

impl Drop for WindowHandle {
    fn drop(&mut self) {
        crate::dbg_log!("WindowHandle dropped: {:?}", self.ref_id);
    }
}

pub(crate) struct RunnerInner {
    pub handles: HashMap<WindowId, WindowHandle>,
    pub last_error: Option<String>,
    pub has_redraw_requested: AtomicBool,
    pub cursor_cache: HashMap<u64, CustomCursor>,
}

impl RunnerInner {
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

impl ApplicationHandler<WindowEvent> for RunnerInner {
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
                    use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};

                    window_attributes =
                        window_attributes.with_corner_preference(CornerPreference::DoNotRound);
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

                    crate::dbg_log!("Window {} created", ref_id);
                    self.handles.insert(window_id, window_handle);
                } else {
                    crate::dbg_log!("Failed to create window: {:?}", window);
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

                    crate::dbg_log!("Window {} closed", ref_id);
                    self.handles.remove(&window_id);
                }

                if self.handles.is_empty() {
                    crate::dbg_log!("All windows closed, exiting event loop");
                    event_loop.exit();
                }
            }
            WindowEvent::Title { ref_id, title } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let window = handle.lock();
                    let window = window.get_window();

                    crate::dbg_log!("Window {} title: {}", ref_id, title);

                    window.set_title(title.as_str());
                }
            }
            WindowEvent::Size { ref_id, size } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let size: PhysicalSize<u32> = size.into();

                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    crate::dbg_log!("Window {} size: {:?}", ref_id, size);

                    window.set_max_inner_size(Some(size));
                    window.set_min_inner_size(Some(size));
                    _ = window.request_inner_size(size);
                }
            }
            WindowEvent::Position { ref_id, pos } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let pos = PhysicalPosition::new(pos.x as i32, pos.y as i32);

                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    crate::dbg_log!("Window {} position: {:?}", ref_id, pos);
                    window.set_outer_position(pos);
                }
            }
            WindowEvent::Visible { ref_id, visible } => {
                if let Some(handle) = self.get_window_handle_by_ref(ref_id) {
                    let handle_ref = handle.lock();
                    let window = handle_ref.get_window();

                    crate::dbg_log!("Window {} visible: {}", ref_id, visible);
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

pub(crate) fn named_key_to_str(key: &NamedKey) -> Option<SmolStr> {
    match key {
        NamedKey::Alt => Some(SmolStr::new("Alt")),
        NamedKey::AltGraph => Some(SmolStr::new("AltGraph")),
        NamedKey::CapsLock => Some(SmolStr::new("CapsLock")),
        NamedKey::Control => Some(SmolStr::new("Control")),
        NamedKey::Fn => Some(SmolStr::new("Fn")),
        NamedKey::FnLock => Some(SmolStr::new("FnLock")),
        NamedKey::NumLock => Some(SmolStr::new("NumLock")),
        NamedKey::ScrollLock => Some(SmolStr::new("ScrollLock")),
        NamedKey::Shift => Some(SmolStr::new("Shift")),
        NamedKey::Symbol => Some(SmolStr::new("Symbol")),
        NamedKey::SymbolLock => Some(SmolStr::new("SymbolLock")),
        NamedKey::Meta => Some(SmolStr::new("Meta")),
        NamedKey::Hyper => Some(SmolStr::new("Hyper")),
        NamedKey::Super => Some(SmolStr::new("Super")),
        NamedKey::Enter => Some(SmolStr::new("Enter")),
        NamedKey::Tab => Some(SmolStr::new("Tab")),
        NamedKey::Space => Some(SmolStr::new("Space")),
        NamedKey::ArrowDown => Some(SmolStr::new("ArrowDown")),
        NamedKey::ArrowLeft => Some(SmolStr::new("ArrowLeft")),
        NamedKey::ArrowRight => Some(SmolStr::new("ArrowRight")),
        NamedKey::ArrowUp => Some(SmolStr::new("ArrowUp")),
        NamedKey::End => Some(SmolStr::new("End")),
        NamedKey::Home => Some(SmolStr::new("Home")),
        NamedKey::PageDown => Some(SmolStr::new("PageDown")),
        NamedKey::PageUp => Some(SmolStr::new("PageUp")),
        NamedKey::Backspace => Some(SmolStr::new("Backspace")),
        NamedKey::Clear => Some(SmolStr::new("Clear")),
        NamedKey::Delete => Some(SmolStr::new("Delete")),
        NamedKey::Insert => Some(SmolStr::new("Insert")),
        NamedKey::Escape => Some(SmolStr::new("Escape")),
        NamedKey::Pause => Some(SmolStr::new("Pause")),
        NamedKey::F1 => Some(SmolStr::new("F1")),
        NamedKey::F2 => Some(SmolStr::new("F2")),
        NamedKey::F3 => Some(SmolStr::new("F3")),
        NamedKey::F4 => Some(SmolStr::new("F4")),
        NamedKey::F5 => Some(SmolStr::new("F5")),
        NamedKey::F6 => Some(SmolStr::new("F6")),
        NamedKey::F7 => Some(SmolStr::new("F7")),
        NamedKey::F8 => Some(SmolStr::new("F8")),
        NamedKey::F9 => Some(SmolStr::new("F9")),
        NamedKey::F10 => Some(SmolStr::new("F10")),
        NamedKey::F11 => Some(SmolStr::new("F11")),
        NamedKey::F12 => Some(SmolStr::new("F12")),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PollMode {
    /// The event loop will poll for events and return immediately.
    Poll,
    /// The event loop will wait for events and return when an event is available.
    Wait,
    /// The event loop will wait for events and return when the window needs to be redrawn.
    /// Unless calling the [Window::request_redraw] method.
    WaitDraw,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseScrollDelta {
    LineDelta { delta_x: f32, delta_y: f32 },
    PixelDelta { delta_x: f32, delta_y: f32 },
}

impl PartialEq for MouseScrollDelta {
    fn eq(&self, other: &Self) -> bool {
        // use near equality for floating point comparison

        match (self, other) {
            (
                MouseScrollDelta::LineDelta { delta_x, delta_y },
                MouseScrollDelta::LineDelta {
                    delta_x: other_x,
                    delta_y: other_y,
                },
            ) => {
                (delta_x - other_x).abs() < f32::EPSILON && (delta_y - other_y).abs() < f32::EPSILON
            }
            (
                MouseScrollDelta::PixelDelta { delta_x, delta_y },
                MouseScrollDelta::PixelDelta {
                    delta_x: other_x,
                    delta_y: other_y,
                },
            ) => {
                (delta_x - other_x).abs() < f32::EPSILON && (delta_y - other_y).abs() < f32::EPSILON
            }
            _ => false,
        }
    }
}

impl PartialOrd for MouseScrollDelta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (
                MouseScrollDelta::LineDelta { delta_x, delta_y },
                MouseScrollDelta::LineDelta {
                    delta_x: other_x,
                    delta_y: other_y,
                },
            ) => Some(
                delta_x
                    .partial_cmp(other_x)?
                    .then(delta_y.partial_cmp(other_y)?),
            ),
            (
                MouseScrollDelta::PixelDelta { delta_x, delta_y },
                MouseScrollDelta::PixelDelta {
                    delta_x: other_x,
                    delta_y: other_y,
                },
            ) => Some(
                delta_x
                    .partial_cmp(other_x)?
                    .then(delta_y.partial_cmp(other_y)?),
            ),
            _ => None,
        }
    }
}

impl Ord for MouseScrollDelta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (
                MouseScrollDelta::LineDelta { delta_x, delta_y },
                MouseScrollDelta::LineDelta {
                    delta_x: other_x,
                    delta_y: other_y,
                },
            ) => delta_x
                .partial_cmp(other_x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    delta_y
                        .partial_cmp(other_y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                ),
            (
                MouseScrollDelta::PixelDelta { delta_x, delta_y },
                MouseScrollDelta::PixelDelta {
                    delta_x: other_x,
                    delta_y: other_y,
                },
            ) => delta_x
                .partial_cmp(other_x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    delta_y
                        .partial_cmp(other_y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                ),
            _ => std::cmp::Ordering::Equal,
        }
    }
}

impl Eq for MouseScrollDelta {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DragAndDropEvent {
    /// Occured when a drag enter the window.
    Dragleft,
    /// Occured when a drag is moved over the window.
    DragEntered,
    /// Occured when a drag is moved over the window.
    DragMoved,
    /// Occured when a drag dropped on the window.
    DragDropped(Vec<String>), // List of file paths
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    /// Happen when the window is closed, either by user action (such clicking X button on window) or programmatically.
    WindowClosed {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
    },
    /// Happen when a new window is created.
    WindowCreated {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The ID of the parent window, if any. will be [None] if the window is a top-level window.
        ///
        /// This can be achived when creating a new window using the [WindowBuilder::with_parent_window] method.
        parent_ref_id: Option<usize>,
        /// The title of the window.
        title: String,
        /// The size of the window in pixels.
        size: Point2,
        /// The position of the window in pixels, if specified.
        pos: Option<Point2>,
    },
    /// Happen when the window is focused or unfocused.
    WindowFocused {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// Focused state of the window.
        focused: bool,
    },
    /// Happen when the window is resized.
    WindowResized {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The new size of the window in pixels.
        size: Point2,
    },
    /// Happen when the window is moved.
    WindowMoved {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The new position of the window in pixels.
        pos: Point2,
    },
    /// Happen when the cursor enters the window.
    CursorEntered {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
    },
    /// Happen when the cursor leaves the window.
    CursorLeft {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
    },
    /// Happen when the cursor is moved within the window.
    CursorMoved {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The new position of the cursor in pixels.
        pos: Point2, // Position in pixels
    },
    /// Happen when the mouse wheel is scrolled.
    MouseWheel {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The delta of the mouse wheel scroll.
        delta: MouseScrollDelta,
    },
    /// Happen when a mouse button is pressed or released.
    MouseInput {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The button that was pressed or released.
        ///
        /// Either "Left", "Right", "Middle", "Back", or "Forward".
        button: SmolStr, // "Left", "Right", "Middle", "Back", "Forward"
        /// Whether the button was pressed or released.
        pressed: bool, // true if pressed, false if released
    },
    /// Happen when the window requests a redraw.
    ///
    /// Can be manually invoked by calling [Window::request_redraw] method.
    RedrawRequested {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
    },
    /// Happen when a keyboard key is pressed or released.
    KeyboardInput {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The key that was pressed or released.
        ///
        /// The key string can be modifier keys like "Alt", "Control", "Shift", etc.
        /// Which where the cases like `a` can be `A`.
        key: SmolStr,
        /// Whether the key was pressed or released.
        pressed: bool, // true if pressed, false if released
    },
    /// Happen when a drag and drop event occurs in the window.
    DragAndDrop {
        /// The ID of the window that was closed, which can be used to identify the window in the application.
        ///
        /// The window ID can be obtained from the [Window] instance using the [Window::id] method.
        window_id: usize,
        /// The drag and drop event that occurred.
        event: DragAndDropEvent,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum WindowEvent {
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
        size: Point2,
    },
    Position {
        ref_id: usize,
        pos: Point2,
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RunnerError {
    ThreadMissmatch,
    WinitEventLoopPanic,
    WinitEventLoopFailed,
    MaximumWindowReached,
    FailedToCreateWindow(String),
}
