use std::{
    sync::{Mutex, atomic::AtomicUsize},
    thread::ThreadId,
    time::Duration,
};

use smol_str::SmolStr;
use winit::{
    event,
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::{Key, NativeKey},
    platform::pump_events::{EventLoopExtPumpEvents, PumpStatus},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

#[cfg(all(not(feature = "x11"), target_os = "linux"))]
use winit::platform::wayland::EventLoopBuilderExtWayland;

#[cfg(all(feature = "x11", target_os = "linux"))]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    math::{Point2, Timing},
    runner::{
        named_key_to_str, runner_inner::Handle, Event, MouseScrollDelta, PollMode, WindowEvent
    },
    utils::{ArcMut, ArcRef}, window::{window_inner::WindowInner, WindowBuilder},
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

/// Provide almost cross-platform event loop for the application.
///
/// This wrap winit's [EventLoop] and provides a way to create windows and handle events.
/// But with some limitations:
/// - No support for iOS and WASM platforms.
/// - macOS platform have to use [PollMode::WaitDraw] or drawing at event [Event::RedrawRequested] because
/// how winit setup the window drawing on macOS.
#[allow(dead_code)]
pub struct Runner {
    pub(crate) app_runner: super::runner_inner::RunnerInner,
    pub(crate) event_loop: ArcRef<EventLoop<WindowEvent>>,
    pub(crate) event_loop_proxy: EventLoopProxy<WindowEvent>,
    pub(crate) window_events_attributes: Vec<ArcRef<WindowInner>>,
    pub(crate) rate_timing: Timing,
    pub(crate) pending_events: Vec<Event>,
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
            app_runner: super::runner_inner::RunnerInner::new(),
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
                    }
                }
                PumpStatus::Exit(code) => {
                    // Exit the event loop
                    crate::dbg_log!("Event loop exited with code: {}", code);

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
