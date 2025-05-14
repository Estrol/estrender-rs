use compute::ComputePass;
use graphics::GraphicsPass;
use wgpu::CommandEncoder;

use crate::utils::ArcRef;

use super::{
    buffer::Buffer,
    inner::GPUInner,
    texture::{Texture, TextureBlend},
};

pub mod compute;
pub mod graphics;

pub enum PassAttachment {
    Texture(Texture, TextureBlend),
}

pub struct CommandBuffer {
    pub(crate) inner: ArcRef<GPUInner>,
    pub(crate) command: Option<CommandEncoder>,

    pub(crate) swapchain: Option<wgpu::SurfaceTexture>,
    pub(crate) swapchain_view: Option<wgpu::TextureView>,
}

impl CommandBuffer {
    pub(crate) fn new(inner: ArcRef<GPUInner>) -> CommandBuffer {
        let inner_ref = inner.borrow();
        let command = inner_ref
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        drop(inner_ref);

        CommandBuffer {
            inner,
            command: Some(command),

            swapchain: None,
            swapchain_view: None,
        }
    }

    /// Begins a new graphics pass.
    pub fn begin_graphics(&mut self) -> Option<GraphicsPass<'_>> {
        if self.inner.borrow().is_invalid {
            panic!("Graphics context is invalid");
        }

        if self.swapchain.is_none() {
            let inner_ref = self.inner.borrow();

            if inner_ref.surface.is_none() {
                panic!(
                    "GPU not configured for window surface, please create the instance with `with_window` in GPU Builder."
                );
            }

            let config = inner_ref.config.as_ref().unwrap();

            if config.width == 0 || config.height == 0 {
                return None;
            }

            let surface_texture = inner_ref.surface.as_ref().unwrap().get_current_texture();

            if surface_texture.is_err() {
                return None;
            }

            self.swapchain = Some(surface_texture.unwrap());
        }

        let surface_view =
            self.swapchain
                .as_ref()
                .unwrap()
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Swapchain Texture View"),
                    ..Default::default()
                });

        self.swapchain_view = Some(surface_view);

        let cmd_ref = self.command.as_mut().unwrap();

        let swapchain_view = self.swapchain_view.as_ref().unwrap().clone();
        let format = self.swapchain.as_ref().unwrap().texture.format();

        Some(GraphicsPass::new(
            &self.inner,
            cmd_ref,
            swapchain_view,
            format,
        ))
    }

    /// Begins a new graphics pass to a render target texture.
    pub fn begin_texture<'a>(&'a mut self, _texture: &'a Texture) -> Option<GraphicsPass<'a>> {
        if self.inner.borrow().is_invalid {
            panic!("Graphics context is invalid");
        }

        let texture = _texture.inner.borrow();
        let texture_view = texture.wgpu_view.clone();
        let format = texture.format;

        drop(texture);

        let cmd_ref = self.command.as_mut().unwrap();

        Some(GraphicsPass::new(
            &self.inner,
            cmd_ref,
            texture_view,
            format.into(),
        ))
    }

    /// Begins a new compute pass.
    pub fn begin_compute(&mut self) -> Option<ComputePass<'_>> {
        if self.inner.borrow().is_invalid {
            panic!("Graphics context is invalid");
        }

        Some(ComputePass::new(&self.inner, self.command.as_mut().unwrap()))
    }

    /// Writes a buffer to a destination buffer.
    pub fn write_buffer(&mut self, src: &Buffer, dst: &Buffer) {
        if dst.size < src.size {
            panic!("Destination buffer is too small");
        }

        dst.write_cmd(src, self.command.as_mut().unwrap());
    }

    /// Writes a buffer to a destination buffer with raw data.
    pub fn write_buffer_raw<T: bytemuck::Pod>(&mut self, data: &[T], dst: &Buffer) {
        if dst.size < data.len() as u64 {
            panic!("Destination buffer is too small");
        }

        dst.write_raw_cmd(data, self.command.as_mut().unwrap());
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        let inner_ref = self.inner.borrow();

        let cmd_buffer = self.command.take().unwrap();
        inner_ref.queue.submit(std::iter::once(cmd_buffer.finish()));

        if let Some(swapchain) = self.swapchain.take() {
            swapchain.present();
        }
    }
}
