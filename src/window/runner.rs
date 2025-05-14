use std::{
    sync::{Arc, Mutex, atomic::AtomicUsize},
    thread::ThreadId,
    time::Duration,
};

use winit::{
    event,
    event_loop::{EventLoop, EventLoopProxy},
    platform::pump_events::{EventLoopExtPumpEvents, PumpStatus},
    window::Window,
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

#[cfg(any(feature = "wayland", target_os = "linux"))]
use winit::platform::wayland::EventLoopBuilderExtWayland;

#[cfg(any(feature = "x11", target_os = "linux"))]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    math::{Point, Timing},
    utils::ArcRef,
};

use super::{
    InnerAttribute,
    inner::{self, WindowEvent},
    input::InputInner,
};

lazy_static::lazy_static! {
    static ref CURRENT_LOOP_THREAD_ID: Mutex<Option<ThreadId>> = Mutex::new(None);
    static ref CURRENT_LOOP: Mutex<Option<EventLoopWrapper>> = Mutex::new(None);
    static ref CURRENT_WINDOW_ID: AtomicUsize = AtomicUsize::new(0);
}

pub struct EventLoopWrapper {
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
    pub(crate) input_events: Vec<ArcRef<InputInner>>,
    pub(crate) rate_timing: Option<Timing>,
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    Close,
    Resize,
    Redraw,
    Move,
    Focus,
    UnFocus,
    KeyPress(&'static str),
    KeyRelease(&'static str),
}

impl Runner {
    pub fn new() -> Result<Self, String> {
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
                EventLoop::<WindowEvent>::with_user_event()
                    .with_any_thread(true)
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
            input_events: Vec::new(),
            rate_timing: Some(Timing::new(60)),
        })
    }

    pub(crate) fn new_window(
        &mut self,
        parent: Option<usize>,
        title: String,
        size: Point,
        pos: Option<Point>,
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

    pub fn pool_events(&mut self, mode: Option<PollMode>) -> bool {
        let mut event_loop = self.event_loop.wait_borrow_mut();

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

        loop {
            match event_loop.pump_app_events(duration, &mut self.app_runner) {
                PumpStatus::Continue => {
                    for window in self.window_events_attributes.iter() {
                        if let Some(mut window) = window.try_borrow_mut() {
                            window.process_event();
                        }
                    }

                    for input in self.input_events.iter() {
                        if let Some(mut input) = input.try_borrow_mut() {
                            input.process_event();
                        }
                    }
                }
                PumpStatus::Exit(code) => {
                    // Exit the event loop
                    println!("Event loop exited with code: {}", code);

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

        if let Some(timing) = self.rate_timing.as_mut() {
            timing.sleep();
        }

        true
    }

    #[allow(unused)]
    pub fn send_event(&self, window: Option<&super::Window>, event: Event) -> Result<(), String> {
        let window_id = if window.is_some() {
            let window = window.unwrap();
            let window_inner = window.inner.borrow();

            Some(window_inner.window_id.clone())
        } else {
            None
        };

        unimplemented!();
    }

    pub fn set_rate(&mut self, rate: Option<Duration>) {
        if let None = rate {
            self.rate_timing = None;
        } else {
            let fps_from_rate = rate.unwrap().as_secs_f64() * 1000.0;
            let fps = 1.0 / fps_from_rate;

            self.rate_timing = Some(Timing::new(fps as u32));
        }
    }

    pub fn set_target_fps(&mut self, fps: u32) {
        if fps <= 0 {
            self.rate_timing = None;
        } else {
            self.rate_timing = Some(Timing::new(fps));
        }
    }

    pub(crate) fn get_events_pointer(
        &self,
        window_id: usize,
    ) -> Option<ArcRef<Vec<event::WindowEvent>>> {
        self.app_runner
            .get_window_by_ref(window_id)
            .map(|handle| handle.events.clone())
    }

    pub(crate) fn get_window_pointer(&self, window_id: usize) -> Option<Arc<Window>> {
        self.app_runner
            .get_window_by_ref(window_id)
            .map(|handle| handle.window.clone())
    }
}
