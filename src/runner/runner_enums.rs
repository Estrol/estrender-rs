use smol_str::SmolStr;
use winit::{keyboard::NamedKey, window::Cursor};

use crate::math::Point2;

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
