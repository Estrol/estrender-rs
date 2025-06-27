#[derive(Clone, Copy, Debug)]
pub enum PixelWriteMode {
    // Append to the existing pixel value
    Copy,
    // Replace the existing pixel value with the new one
    Clear,
    // Blend the new pixel value with the existing one, such as alpha blending
    Blend,
}

#[derive(Clone, Copy, Debug)]
pub enum PixelBlendMode {
    // Alpha blending
    Alpha,
    // Additive blending
    Add,
    // Subtractive blending
    Subtract,
    // Multiplicative blending
    Multiply,
}

#[derive(Clone, Copy, Debug)]
pub enum PixelBufferError {
    WindowPointerIsNull,
    ContextCreationFailed,
    SurfaceCreationFailed,
    InvalidSize(u32, u32),
    BufferFetchFailed,
    BufferTooSmall,
    PresentFailed,
}

impl std::fmt::Display for PixelBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelBufferError::WindowPointerIsNull => write!(f, "Window pointer is null"),
            PixelBufferError::ContextCreationFailed => write!(f, "Failed to create pixel buffer context"),
            PixelBufferError::SurfaceCreationFailed => write!(f, "Failed to create pixel buffer surface"),
            PixelBufferError::InvalidSize(width, height) => write!(f, "Invalid size: {}x{}", width, height),
            PixelBufferError::BufferFetchFailed => write!(f, "Failed to fetch pixel buffer"),
            PixelBufferError::BufferTooSmall => write!(f, "Pixel buffer is too small"),
            PixelBufferError::PresentFailed => write!(f, "Failed to present pixel buffer"),
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub enum PixelBufferBuilderError {
    WindowIsNull,
    CannotUseWithGPUWindow,
    PixelBufferError(PixelBufferError),
}