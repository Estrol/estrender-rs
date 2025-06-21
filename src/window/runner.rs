use std::{
    sync::{Mutex, atomic::AtomicUsize},
    thread::ThreadId,
    time::Duration,
};

use smol_str::SmolStr;
use winit::{
    event,
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::{Key, NamedKey, NativeKey},
    platform::pump_events::{EventLoopExtPumpEvents, PumpStatus},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

#[cfg(all(not(feature = "x11"), target_os = "linux"))]
use winit::platform::wayland::EventLoopBuilderExtWayland;

#[cfg(all(feature = "x11", target_os = "linux"))]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    dbg_log,
    math::{Point2, Timing},
    utils::{ArcMut, ArcRef},
};

use super::{
    Handle, InnerAttribute, Window,
    inner::{self, WindowEvent},
};

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

#[allow(dead_code)]
pub struct Runner {
    pub(crate) app_runner: inner::WindowInner,
    pub(crate) event_loop: ArcRef<EventLoop<WindowEvent>>,
    pub(crate) event_loop_proxy: EventLoopProxy<WindowEvent>,
    pub(crate) window_events_attributes: Vec<ArcRef<InnerAttribute>>,
    // pub(crate) input_events: Vec<ArcRef<InputInner>>,
    pub(crate) rate_timing: Timing,
    pub(crate) pending_events: Vec<Event>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PollMode {
    /// The event loop will poll for events and return immediately.
    Poll,
    /// The event loop will wait for events and return when an event is available.
    Wait,
    /// The event loop will wait for events and return when the window needs to be redrawn.
    /// Unless calling the `request_redraw` method.
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
    Dragleft,
    DragEntered,
    DragMoved,
    DragDropped(Vec<String>), // List of file paths
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    Closed {
        window_id: usize,
    },
    Created {
        ref_id: usize,
        parent_ref_id: Option<usize>,
        title: String,
        size: Point2,
        pos: Option<Point2>,
    },
    Focused {
        window_id: usize,
        focused: bool,
    },
    Resized {
        window_id: usize,
        size: Point2,
    },
    Moved {
        window_id: usize,
        pos: Point2,
    },
    CursorEntered {
        window_id: usize,
    },
    CursorLeft {
        window_id: usize,
    },
    CursorMoved {
        window_id: usize,
        pos: Point2, // Position in pixels
    },
    MouseWheel {
        window_id: usize,
        delta: MouseScrollDelta,
    },
    MouseInput {
        window_id: usize,
        button: SmolStr, // "Left", "Right", "Middle", "Back", "Forward"
        pressed: bool,   // true if pressed, false if released
    },
    RedrawRequested {
        window_id: usize,
    },
    KeyboardInput {
        window_id: usize,
        key: SmolStr,
        pressed: bool, // true if pressed, false if released
    },
    DragAndDrop {
        window_id: usize,
        event: DragAndDropEvent,
    },
}

impl Runner {
    pub(crate) fn new() -> Result<Self, String> {
        let thread_id = std::thread::current().id();

        if CURRENT_LOOP_THREAD_ID.lock().unwrap().is_none() {
            *CURRENT_LOOP_THREAD_ID.lock().unwrap() = Some(thread_id);
        } else if CURRENT_LOOP_THREAD_ID.lock().unwrap().as_ref() != Some(&thread_id) {
            return Err("Event loop can only be created in the last caller thread.".to_string());
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

                event_loop_builder
                    .build()
                    .expect("Failed to create EventLoop")
            });

            if event_loop_result.is_err() {
                *CURRENT_LOOP_THREAD_ID.lock().unwrap() = None;

                return Err(format!(
                    "Failed to create EventLoop: {:?}",
                    event_loop_result.err()
                ));
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
            app_runner: inner::WindowInner::new(),
            event_loop,
            event_loop_proxy,
            window_events_attributes: Vec::new(),
            // input_events: Vec::new(),
            rate_timing: Timing::new(60),
            pending_events: Vec::new(),
        })
    }

    pub fn get_events(&self) -> &Vec<Event> {
        &self.pending_events
    }

    /// Creates a new window with the given title, size, and position.
    ///
    /// **NOTE:** This function will make the thread caller the main thread,
    /// it will panic if called from a different thread after the first call.
    ///
    /// # Example
    /// ```rs
    /// use engine::prelude::*;
    ///
    /// let mut runner = make_runner().unwrap();
    /// let window = runner.make_window("My Window", Point::new(800, 600))
    ///    .build()
    ///    .unwrap();
    /// ```
    pub fn create_window(&mut self, title: &str, size: Point2) -> WindowBuilder {
        WindowBuilder::new(self, title, size)
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
    // pub fn make_input(&mut self, window: &Window) -> Input {
    //     Input::new(self, window)
    // }

    pub(crate) fn internal_new_window(
        &mut self,
        parent: Option<usize>,
        title: String,
        size: Point2,
        pos: Option<Point2>,
    ) -> Result<(usize, EventLoopProxy<WindowEvent>), String> {
        let mut event_loop = self.event_loop.wait_borrow_mut();
        let event_loop_proxy = event_loop.create_proxy();

        let window_id = CURRENT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if window_id >= 1000 {
            return Err("Maximum window reached!".to_string());
        }

        let res = event_loop_proxy.send_event(WindowEvent::Create {
            ref_id: window_id,
            parent_ref_id: parent,
            title,
            size,
            pos,
        });

        if res.is_err() {
            return Err(self
                .app_runner
                .last_error
                .clone()
                .unwrap_or_else(|| "Failed to create window!".to_string()));
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
            return Err(self
                .app_runner
                .last_error
                .clone()
                .unwrap_or_else(|| "Failed to create window!".to_string()));
        }

        Ok((window_id, event_loop_proxy))
    }

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

                            let window_events = window.window_events.wait_borrow_mut();
                            for event in window_events.iter() {
                                match event {
                                    event::WindowEvent::CloseRequested => {
                                        self.pending_events.push(Event::Closed {
                                            window_id: window.window_id,
                                        });
                                    }
                                    event::WindowEvent::Resized(size) => {
                                        self.pending_events.push(Event::Resized {
                                            window_id: window.window_id,
                                            size: Point2::new(size.width, size.height),
                                        });
                                    }
                                    event::WindowEvent::Moved(pos) => {
                                        self.pending_events.push(Event::Moved {
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
                                        self.pending_events.push(Event::Focused {
                                            window_id: window.window_id,
                                            focused: *focused,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    // for input in self.input_events.iter() {
                    //     if let Some(mut input) = input.try_borrow_mut() {
                    //         input.process_event();
                    //     }
                    // }
                }
                PumpStatus::Exit(code) => {
                    // Exit the event loop
                    dbg_log!("Event loop exited with code: {}", code);

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

    // #[allow(unused)]
    // pub fn send_event(&self, window: Option<&super::Window>, event: Event) -> Result<(), String> {
    //     let window_id = if window.is_some() {
    //         let window = window.unwrap();
    //         let window_inner = window.inner.borrow();

    //         Some(window_inner.window_id.clone())
    //     } else {
    //         None
    //     };

    //     unimplemented!();
    // }

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

    pub fn set_target_fps(&mut self, fps: u32) {
        self.rate_timing.set_fps(fps);
    }

    pub fn get_target_fps(&self) -> u32 {
        self.rate_timing.get_fps()
    }

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

    pub fn build(self) -> Result<Window, String> {
        Window::new(
            self.runner,
            self.parent_window,
            self.title,
            self.size,
            self.pos,
        )
    }
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