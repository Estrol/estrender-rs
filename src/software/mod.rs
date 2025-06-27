//! Implementation of the software renderer using softbuffer crate.
//!
//! This module provides a software renderer that can be used for rendering graphics without relying on a GPU.
//! Does not provided any high-level abstractions such drawing quad or image, but rather low-level access to the softbuffer crate. \
//! Provided as it, without any guarantees of performance or correctness.

mod software_enums;
mod software_impls;
pub(crate) mod software_inner;

pub use software_enums::*;
pub use software_impls::*;
