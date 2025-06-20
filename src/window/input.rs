use std::collections::HashMap;

use winit::{keyboard::{Key, NamedKey, SmolStr}};

use crate::{math::Vector2, utils::ArcRef, window::{Event, MouseScrollDelta}};

use super::{Window, runner::Runner};

pub struct KeyBinding<T: Eq> {
    pub key: T,
}

pub(crate) struct InputInner {
    pub window_id: usize,
    pub key_bindings: HashMap<SmolStr, bool>,
    pub just_key_bindings: HashMap<SmolStr, bool>,
    
    pub mouse_buttons: HashMap<SmolStr, bool>,
    pub mouse_position: Vector2,
    pub mouse_wheel: Vector2,
}

pub type KeyboardInputCallback = Box<dyn Fn(Key, bool) + Send + Sync>;

impl InputInner {
    pub fn process_event(&mut self, events: &[Event]) {
        for event in events {
            match event {
                Event::KeyboardInput { key, pressed, window_id } => {
                    if self.window_id != *window_id {
                        continue;
                    }

                    self.key_bindings
                        .insert(key.clone(), *pressed);
                    self.just_key_bindings
                        .insert(key.clone(), *pressed);
                },
                Event::MouseInput { button, pressed, window_id } => {
                    if self.window_id != *window_id {
                        continue;
                    }

                    self.mouse_buttons
                        .insert(button.clone(), *pressed);
                }
                Event::CursorMoved { pos, window_id } => {
                    if self.window_id != *window_id {
                        continue;
                    }

                    self.mouse_position = Vector2::new(pos.x as f32, pos.y as f32);
                }
                Event::MouseWheel { delta, window_id } => {
                    if self.window_id != *window_id {
                        continue;
                    }

                    match delta {
                        MouseScrollDelta::LineDelta { delta_x, delta_y} => {
                            self.mouse_wheel.x += *delta_x as f32;
                            self.mouse_wheel.y += *delta_y as f32;
                        }
                        MouseScrollDelta::PixelDelta { delta_x, delta_y} => {
                            self.mouse_wheel.x += *delta_x as f32;
                            self.mouse_wheel.y += *delta_y as f32;
                        }
                    }
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
        let inner = InputInner {
            window_id: window.inner.wait_borrow().window_id,
            key_bindings: HashMap::new(),
            just_key_bindings: HashMap::new(),
            mouse_buttons: HashMap::new(),
            mouse_position: Vector2::default(),
            mouse_wheel: Vector2::default(),
        };

        let inner = ArcRef::new(inner);

        runner.input_events.push(inner.clone());
        
        Self {
            inner,
        }
    }

    // pub fn is_key_pressed(&self, key: &str) -> bool {
    //     let inner_ref = self.inner.borrow();
    //     let key = str_mapping_key(key);

    //     if let Some(pressed) = inner_ref.key_bindings.get(&key) {
    //         *pressed
    //     } else {
    //         false
    //     }
    // }

    // pub fn is_key_released(&self, key: &str) -> bool {
    //     let inner_ref = self.inner.borrow();
    //     let smol_str = SmolStr::from
    // }

    // pub fn is_key_just_pressed(&self, key: &str) -> bool {
    //     let mut inner_ref = self.inner.borrow_mut();
    //     let key = str_mapping_key(key);

    //     if let Some(pressed) = inner_ref.just_key_bindings.get_mut(&key) {
    //         *pressed = true;
    //         *pressed
    //     } else {
    //         false
    //     }
    // }

    // pub(crate) fn get_mouse_code(&self, button: &str) -> Option<u8> {
    //     match button {
    //         "Left" => Some(0),
    //         "Right" => Some(1),
    //         "Middle" => Some(2),
    //         "Back" => Some(3),
    //         "Forward" => Some(4),
    //         _ => None,
    //     }
    // }

    // pub fn is_mouse_button_pressed(&self, button: &str) -> bool {
    //     let button_code = self.get_mouse_code(button);
    //     if let None = button_code {
    //         return false;
    //     }

    //     let button_code = button_code.unwrap();
    //     let inner_ref = self.inner.borrow();

    //     if let Some(pressed) = inner_ref.mouse_buttons.get(&button_code) {
    //         *pressed
    //     } else {
    //         false
    //     }
    // }

    // pub fn is_mouse_button_released(&self, button: &str) -> bool {
    //     let button_code = self.get_mouse_code(button);
    //     if let None = button_code {
    //         return false;
    //     }

    //     let button_code = button_code.unwrap();
    //     let inner_ref = self.inner.borrow();

    //     if let Some(pressed) = inner_ref.mouse_buttons.get(&button_code) {
    //         !pressed
    //     } else {
    //         false
    //     }
    // }

    // pub fn get_mouse_position(&self) -> Vector2 {
    //     let inner_ref = self.inner.borrow();
    //     inner_ref.mouse_position
    // }

    // pub fn get_mouse_wheel(&self) -> Vector2 {
    //     let inner_ref = self.inner.borrow();
    //     inner_ref.mouse_wheel
    // }

    // pub fn connect_keyboard_callback<F>(&mut self, callback: F)
    // where
    //     F: Fn(Key, bool) + Send + Sync + 'static,
    // {
    //     self.inner
    //         .borrow_mut()
    //         .keyboard_callbacks
    //         .push(Box::new(callback));
    // }
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
