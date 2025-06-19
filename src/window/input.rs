use std::collections::HashMap;

use winit::keyboard::{Key, NamedKey, SmolStr};

use crate::{math::Vector2, utils::ArcRef};

use super::{Window, runner::Runner};

pub struct KeyBinding<T: Eq> {
    pub key: T,
}

pub(crate) struct InputInner {
    pub key_bindings: HashMap<Key, bool>,
    pub just_key_bindings: HashMap<Key, bool>,
    pub events: ArcRef<Vec<winit::event::WindowEvent>>,
    pub keyboard_callbacks: Vec<KeyboardInputCallback>,

    pub mouse_position: Vector2,
    pub mouse_wheel: Vector2,
    pub mouse_buttons: HashMap<u8, bool>,
}

pub type KeyboardInputCallback = Box<dyn Fn(Key, bool) + Send + Sync>;

impl InputInner {
    pub fn process_event(&mut self) {
        for event in self.events.wait_borrow().iter() {
            match event {
                winit::event::WindowEvent::KeyboardInput {
                    device_id: _,
                    event,
                    is_synthetic,
                } => {
                    if *is_synthetic {
                        return;
                    }

                    let key = event.logical_key.clone();

                    let previous_state = *self.key_bindings.get(&key).unwrap_or(&false);
                    let current_state = event.state == winit::event::ElementState::Pressed;

                    self.key_bindings
                        .entry(key.clone())
                        .and_modify(|state| *state = current_state)
                        .or_insert(current_state);

                    // Just keybinding set to false when keyup
                    if !current_state {
                        self.just_key_bindings.insert(key.clone(), false);
                    }

                    if previous_state != current_state {
                        for callback in &self.keyboard_callbacks {
                            callback(key.clone(), current_state);
                        }
                    }
                }
                winit::event::WindowEvent::MouseWheel {
                    device_id: _,
                    delta,
                    phase: _,
                } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.mouse_wheel.x += *x;
                        self.mouse_wheel.y += *y;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(physical_position) => {
                        self.mouse_wheel.x += physical_position.x as f32;
                        self.mouse_wheel.y += physical_position.y as f32;
                    }
                },
                winit::event::WindowEvent::MouseInput {
                    device_id: _,
                    state,
                    button,
                } => {
                    let button_code = match button {
                        winit::event::MouseButton::Left => 0,
                        winit::event::MouseButton::Right => 1,
                        winit::event::MouseButton::Middle => 2,
                        winit::event::MouseButton::Back => 3,
                        winit::event::MouseButton::Forward => 4,
                        winit::event::MouseButton::Other(code) => *code as u8,
                    };

                    println!(
                        "Mouse button: {:?}, {}",
                        button,
                        *state == winit::event::ElementState::Pressed
                    );

                    self.mouse_buttons
                        .insert(button_code, *state == winit::event::ElementState::Pressed);
                }
                winit::event::WindowEvent::CursorMoved {
                    device_id: _,
                    position,
                } => {
                    self.mouse_position.x = position.x as f32;
                    self.mouse_position.y = position.y as f32;
                }
                _ => {}
            }
        }
    }
}

pub struct Input {
    pub(crate) inner: ArcRef<InputInner>,
}

impl Input {
    pub fn new(runner: &mut Runner, window: &Window) -> Self {
        let window_inner = window.inner.borrow();
        let window_id = &window_inner
            .window_pointer
            .as_ref()
            .unwrap()
            .lock()
            .get_window_id();

        let inner = InputInner {
            key_bindings: HashMap::new(),
            just_key_bindings: HashMap::new(),
            events: ArcRef::clone(&runner.app_runner.handles.get(window_id).unwrap().events),
            keyboard_callbacks: Vec::new(),

            mouse_position: Vector2::new(0.0, 0.0),
            mouse_wheel: Vector2::new(0.0, 0.0),
            mouse_buttons: HashMap::new(),
        };

        let inner = ArcRef::new(inner);
        runner.input_events.push(ArcRef::clone(&inner));

        Input { inner }
    }

    pub fn is_key_pressed(&self, key: &str) -> bool {
        let inner_ref = self.inner.borrow();
        let key = str_mapping_key(key);

        if let Some(pressed) = inner_ref.key_bindings.get(&key) {
            *pressed
        } else {
            false
        }
    }

    pub fn is_key_released(&self, key: &str) -> bool {
        let inner_ref = self.inner.borrow();
        let key = str_mapping_key(key);

        if let Some(pressed) = inner_ref.key_bindings.get(&key) {
            !pressed
        } else {
            false
        }
    }

    pub fn is_key_just_pressed(&self, key: &str) -> bool {
        let mut inner_ref = self.inner.borrow_mut();
        let key = str_mapping_key(key);

        if let Some(pressed) = inner_ref.just_key_bindings.get_mut(&key) {
            *pressed = true;
            *pressed
        } else {
            false
        }
    }

    pub(crate) fn get_mouse_code(&self, button: &str) -> Option<u8> {
        match button {
            "Left" => Some(0),
            "Right" => Some(1),
            "Middle" => Some(2),
            "Back" => Some(3),
            "Forward" => Some(4),
            _ => None,
        }
    }

    pub fn is_mouse_button_pressed(&self, button: &str) -> bool {
        let button_code = self.get_mouse_code(button);
        if let None = button_code {
            return false;
        }

        let button_code = button_code.unwrap();
        let inner_ref = self.inner.borrow();

        if let Some(pressed) = inner_ref.mouse_buttons.get(&button_code) {
            *pressed
        } else {
            false
        }
    }

    pub fn is_mouse_button_released(&self, button: &str) -> bool {
        let button_code = self.get_mouse_code(button);
        if let None = button_code {
            return false;
        }

        let button_code = button_code.unwrap();
        let inner_ref = self.inner.borrow();

        if let Some(pressed) = inner_ref.mouse_buttons.get(&button_code) {
            !pressed
        } else {
            false
        }
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        let inner_ref = self.inner.borrow();
        inner_ref.mouse_position
    }

    pub fn get_mouse_wheel(&self) -> Vector2 {
        let inner_ref = self.inner.borrow();
        inner_ref.mouse_wheel
    }

    pub fn connect_keyboard_callback<F>(&mut self, callback: F)
    where
        F: Fn(Key, bool) + Send + Sync + 'static,
    {
        self.inner
            .borrow_mut()
            .keyboard_callbacks
            .push(Box::new(callback));
    }
}

pub(crate) fn str_mapping_key(key: &str) -> Key {
    match key.to_lowercase().as_str() {
        "alt" => Key::Named(NamedKey::Alt),
        "altgraph" => Key::Named(NamedKey::AltGraph),
        "capslock" => Key::Named(NamedKey::CapsLock),
        "control" => Key::Named(NamedKey::Control),
        "fn" => Key::Named(NamedKey::Fn),
        "fnlock" => Key::Named(NamedKey::FnLock),
        "numlock" => Key::Named(NamedKey::NumLock),
        "scrolllock" => Key::Named(NamedKey::ScrollLock),
        "shift" => Key::Named(NamedKey::Shift),
        "symbol" => Key::Named(NamedKey::Symbol),
        "symbollock" => Key::Named(NamedKey::SymbolLock),
        "meta" => Key::Named(NamedKey::Meta),
        "hyper" => Key::Named(NamedKey::Hyper),
        "super" => Key::Named(NamedKey::Super),
        "enter" => Key::Named(NamedKey::Enter),
        "tab" => Key::Named(NamedKey::Tab),
        "space" => Key::Named(NamedKey::Space),
        "arrowdown" => Key::Named(NamedKey::ArrowDown),
        "arrowleft" => Key::Named(NamedKey::ArrowLeft),
        "arrowright" => Key::Named(NamedKey::ArrowRight),
        "arrowup" => Key::Named(NamedKey::ArrowUp),
        "end" => Key::Named(NamedKey::End),
        "home" => Key::Named(NamedKey::Home),
        "pagedown" => Key::Named(NamedKey::PageDown),
        "pageup" => Key::Named(NamedKey::PageUp),
        "backspace" => Key::Named(NamedKey::Backspace),
        "clear" => Key::Named(NamedKey::Clear),
        "delete" => Key::Named(NamedKey::Delete),
        "insert" => Key::Named(NamedKey::Insert),
        "escape" => Key::Named(NamedKey::Escape),
        "pause" => Key::Named(NamedKey::Pause),
        "f1" => Key::Named(NamedKey::F1),
        "f2" => Key::Named(NamedKey::F2),
        "f3" => Key::Named(NamedKey::F3),
        "f4" => Key::Named(NamedKey::F4),
        "f5" => Key::Named(NamedKey::F5),
        "f6" => Key::Named(NamedKey::F6),
        "f7" => Key::Named(NamedKey::F7),
        "f8" => Key::Named(NamedKey::F8),
        "f9" => Key::Named(NamedKey::F9),
        "f10" => Key::Named(NamedKey::F10),
        "f11" => Key::Named(NamedKey::F11),
        "f12" => Key::Named(NamedKey::F12),

        _ => Key::Character(SmolStr::new(key.to_lowercase())),
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
