use std::collections::HashMap;

use smol_str::SmolStr;

use crate::{math::Point2, runner::{Event, Runner}, utils::ArcRef};

pub type KeyboardEvent = Box<dyn Fn(&str, bool) + Send + Sync>;
pub type MouseEvent = Box<dyn Fn(&str, Point2, bool) + Send + Sync>;
pub type MouseMoveEvent = Box<dyn Fn(Point2) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct Input {
    pub(crate) inner: ArcRef<InputInner>,
}

impl Input {
    pub(crate) fn new(runner: &mut Runner, window_id: Option<usize>) -> Self {
        let mut inner = InputInner::default();
        inner.window_id = window_id;

        let inner = ArcRef::new(inner);
        runner.input_events_attributes.push(ArcRef::clone(&inner));

        Self {
            inner
        }
    }

    /// Returns the current mouse position in pixels.
    pub fn mouse_position(&self) -> Point2 {
        self.inner.borrow().mouse_position
    }

    /// Returns true if the mouse button is currently pressed down.
    /// 
    /// Expected button names are:
    /// - "Left"
    /// - "Right"
    /// - "Middle"
    /// - "Back"
    /// - "Forward"
    pub fn mouse_pressed(&self, button: &str) -> bool {
        self.inner.borrow()
            .mouse_buttons
            .get(button)
            .copied()
            .unwrap_or(false)
    }

    /// Returns true if the mouse button was pressed once since the last call to this method.
    /// 
    /// See [`Input::mouse_pressed`] for expected button names.
    pub fn mouse_pressed_once(&self, button: &str) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some(pressed) = inner.mouse_buttons_once.get(button) {
            if *pressed {
                inner.mouse_buttons_once.insert(SmolStr::from(button), false);
                return true;
            }
        }

        false
    }

    /// Returns true if the key is currently pressed down.
    /// 
    /// The key should be a string representation of the key, such as "a", "Enter", "Space", etc.
    /// 
    /// The normal key names are used, such as:
    /// - "a"
    /// - "b"
    /// - etc.
    /// 
    /// The modifier keys are also supported such as:
    /// - "Shift"
    /// - etc.
    /// 
    /// Which also can be combined with other keys, such as:
    /// - "A" (Shift + "a")
    /// - "B" (Shift + "b")
    /// - etc.
    /// 
    /// This also supports unknown scancodes!
    pub fn key_pressed(&self, key: &str) -> bool {
        self.inner.borrow()
            .keyboard_keys
            .get(key)
            .copied()
            .unwrap_or(false)
    }

    /// Returns true if the key was pressed once since the last call to this method.
    /// 
    /// See [`Input::key_pressed`] for expected key names.
    pub fn key_pressed_once(&self, key: &str) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some(pressed) = inner.keyboard_keys_once.get(key) {
            if *pressed {
                inner.keyboard_keys_once.insert(SmolStr::from(key), false);
                return true;
            }
        }

        false
    }

    /// Listens for mouse events.
    pub fn listen_mouse_event<F>(&mut self, event: F)
    where
        F: Fn(&str, Point2, bool) + Send + Sync + 'static,
    {
        self.inner.borrow_mut().mouse_events.push(Box::new(event));
    }

    /// Listens for mouse move events.
    pub fn listen_mouse_move_event<F>(&mut self, event: F)
    where
        F: Fn(Point2) + Send + Sync + 'static,
    {
        self.inner.borrow_mut().mouse_move_events.push(Box::new(event));
    }

    /// Listens for keyboard events.
    pub fn listen_keyboard_event<F>(&mut self, event: F)
    where
        F: Fn(&str, bool) + Send + Sync + 'static,
    {
        self.inner.borrow_mut().keyboard_events.push(Box::new(event));
    }
}

#[derive(Default)]
pub(crate) struct InputInner {
    window_id: Option<usize>,

    mouse_position: Point2,

    mouse_buttons: HashMap<SmolStr, bool>,
    mouse_buttons_once: HashMap<SmolStr, bool>,

    keyboard_keys: HashMap<SmolStr, bool>,
    keyboard_keys_once: HashMap<SmolStr, bool>,

    mouse_events: Vec<MouseEvent>,
    keyboard_events: Vec<KeyboardEvent>,
    mouse_move_events: Vec<MouseMoveEvent>,
}

impl InputInner {
    pub fn process_event(&mut self, event: &Event) {
        match event {
            Event::CursorMoved { pos, window_id } => {
                if self.window_id.is_some() && self.window_id != Some(*window_id) {
                    return;
                }

                self.mouse_position = Point2::new(pos.x as f32, pos.y as f32);

                for mouse_move_event in &self.mouse_move_events {
                    mouse_move_event(self.mouse_position);
                }
            }
            Event::MouseInput { button, pressed, window_id } => {
                if self.window_id.is_some() && self.window_id != Some(*window_id) {
                    return;
                }
                let last_state = self.mouse_buttons.get(button).copied().unwrap_or(false);
                self.mouse_buttons.insert(button.clone(), *pressed);
                self.mouse_buttons_once.insert(button.clone(), *pressed);

                if last_state != *pressed {
                    for mouse_event in &self.mouse_events {
                        mouse_event(button, self.mouse_position, *pressed);
                    }
                }
            }
            Event::KeyboardInput { key, pressed, window_id } => {
                if self.window_id.is_some() && self.window_id != Some(*window_id) {
                    return;
                }

                let last_state = self.keyboard_keys.get(key).copied().unwrap_or(false);
                self.keyboard_keys.insert(key.clone(), *pressed);
                self.keyboard_keys_once.insert(key.clone(), *pressed);

                if last_state != *pressed {
                    for keyboard_event in &self.keyboard_events {
                        keyboard_event(key, *pressed);
                    }
                }
            }
            _ => {}
        }
    }
}