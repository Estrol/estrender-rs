use std::{num::NonZero, sync::Arc};

use softbuffer::{Context, Surface};
use winit::dpi::PhysicalSize;

use crate::math::Vector2;

pub type SoftbufferSurface = Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>;
pub type SoftbufferContext = Context<Arc<winit::window::Window>>;

pub(crate) struct PixelBufferInner {
    pub _context: SoftbufferContext,
    pub surface: SoftbufferSurface,
    pub surface_size: Vector2,
}

impl PixelBufferInner {
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), String> {
        if size.width == 0 || size.height == 0 {
            return Err("Invalid size".to_string());
        }

        self.surface_size = Vector2::new(size.width as f32, size.height as f32);

        let width: NonZero<u32> = NonZero::new(size.width).ok_or("Width cannot be zero")?;
        let height: NonZero<u32> = NonZero::new(size.height).ok_or("Height cannot be zero")?;

        let result = self.surface.resize(width, height);
        if result.is_err() {
            return Err(format!(
                "Failed to resize softbuffer surface: {:?}",
                result.err()
            ));
        }

        Ok(())
    }
}
