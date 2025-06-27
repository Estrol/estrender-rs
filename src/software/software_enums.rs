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
