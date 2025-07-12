//! Easy to use winit, softbuffer & wgpu abstractions

/// Font rendering and text layout utilities
pub mod font;
/// GPU graphics rendering abstractions
pub mod gpu;
/// Mathematical utilities and types
pub mod math;
/// Predefined types and traits for easy access
pub mod prelude;
/// Runner for managing the main event loop and window lifecycle
pub mod runner;
/// Software rendering utilities
#[cfg(feature = "software")]
pub mod software;
/// Utility functions and types for common tasks
pub mod utils;
/// Window management
pub mod window;