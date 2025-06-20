use wgpu::CommandEncoder;

use crate::{dbg_log, log, math::Point2, utils::ArcRef};

use super::{
    SwapchainError, TextureUsage,
    buffer::Buffer,
    inner::GPUInner,
    texture::{Texture, TextureBlend},
};

pub(crate) mod compute;
pub(crate) mod drawing;
pub(crate) mod graphics;

pub use compute::*;
pub use graphics::*;

pub enum PassAttachment {
    Texture(Texture, TextureBlend),
}

#[derive(Clone, Debug)]
pub(crate) struct BindGroupAttachment {
    pub group: u32,
    pub binding: u32,
    pub attachment: BindGroupType,
}

pub struct TextureInput<'a> {
    pub texture: Option<&'a Texture>,
    pub binding_texture: usize,
    pub binding_sampler: usize,
}

#[derive(Clone, Debug)]
pub struct CommandBuffer {
    pub(crate) inner: ArcRef<GPUInner>,

    pub(crate) command: Option<ArcRef<CommandEncoder>>,
    pub(crate) on_renderpass: bool,
    pub(crate) on_compute: bool,

    pub(crate) swapchain: SurfaceTexture,
}

impl CommandBuffer {
    pub(crate) fn new(inner: ArcRef<GPUInner>) -> CommandBuffer {
        let inner_ref = inner.borrow();
        let command =
            inner_ref
                .get_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        drop(inner_ref);

        CommandBuffer {
            inner,
            command: Some(ArcRef::new(command)),
            on_renderpass: false,
            on_compute: false,

            swapchain: SurfaceTexture::new(),
        }
    }

    pub(crate) fn new_with_surface(
        inner: ArcRef<GPUInner>,
        surface: SurfaceTexture,
    ) -> CommandBuffer {
        let inner_ref = inner.borrow();
        let command =
            inner_ref
                .get_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        drop(inner_ref);

        CommandBuffer {
            inner,
            command: Some(ArcRef::new(command)),
            on_renderpass: false,
            on_compute: false,

            swapchain: surface,
        }
    }

    /// Begins a new graphics pass.
    pub fn begin_renderpass(&mut self) -> Option<RenderPass> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass || self.on_compute {
            panic!("CMD already in a render pass or compute pass");
        }

        if !self.swapchain.is_valid() {
            let inner_ref = self.inner.borrow();

            let swapchain = inner_ref.get_swapchain();

            match swapchain {
                Ok(swapchain) => {
                    self.swapchain.set_texture(swapchain);
                }
                Err(SwapchainError::Suboptimal(swapchain)) => {
                    self.swapchain.set_texture(swapchain);
                }
                Err(err) => {
                    log!("Swapchain error: {}", err);
                    return None;
                }
            }
        }

        // let format = self.swapchain.as_ref().unwrap().texture.format();
        // let size = self.swapchain.as_ref().unwrap().texture.size();

        let view = self.swapchain.get_view();
        let format = self.swapchain.get_format();
        let size = self.swapchain.get_size();

        Some(RenderPass::new(
            ArcRef::clone(&self.inner),
            ArcRef::clone(self.command.as_ref().unwrap()),
            view,
            format.into(),
            Point2::new(size.width as i32, size.height as i32),
        ))
    }

    /// Begins a new graphics pass to a render target texture.
    pub fn begin_texture<'a>(&'a mut self, _texture: &'a Texture) -> Option<RenderPass> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass || self.on_compute {
            panic!("CMD already in a render pass or compute pass");
        }

        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_arc_ref = ArcRef::clone(self.command.as_ref().unwrap());

        self.on_renderpass = true;
        scopeguard::defer! {
            self.on_renderpass = false;
        }

        let texture = _texture.inner.borrow();
        if !texture.usages.contains(TextureUsage::RenderAttachment) {
            return None;
        }

        let texture_view = texture.wgpu_view.clone();
        let format = texture.format;
        let size = texture.size;

        drop(texture);

        Some(RenderPass::new(
            gpu_arc_ref,
            cmd_arc_ref,
            texture_view,
            format.into(),
            Point2::new(size.w as i32, size.h as i32),
        ))
    }

    /// Begins a new compute pass.
    pub fn begin_computepass(&mut self) -> Option<ComputePass> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass || self.on_compute {
            panic!("CMD already in a render pass or compute pass");
        }

        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_ref = ArcRef::clone(self.command.as_ref().unwrap());

        self.on_compute = true;
        scopeguard::defer! {
            self.on_compute = false;
        }

        Some(ComputePass::new(gpu_arc_ref, cmd_ref))
    }

    /// Writes a buffer to a destination buffer.
    pub fn write_buffer(&mut self, src: &Buffer, dst: &Buffer) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if dst.size < src.size {
            panic!("Destination buffer is too small");
        }

        dst.write_cmd(src, self);
    }

    /// Writes a buffer to a destination buffer with raw data.
    pub fn write_buffer_raw<T: bytemuck::Pod>(&mut self, data: &[T], dst: &Buffer) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if dst.size < data.len() as u64 {
            panic!("Destination buffer is too small");
        }

        dst.write_raw_cmd(data, self);
    }

    pub fn end(&mut self, present: bool) {
        let inner_ref = self.inner.borrow();

        if self.command.is_none() {
            return;
        }

        let cmd = ArcRef::try_unwrap(self.command.take().unwrap()).unwrap_or_else(|_| {
            panic!("Command buffer dropped while still in use");
        });

        inner_ref.get_queue().submit(std::iter::once(cmd.finish()));

        if present {
            self.swapchain.present();
        }
    }

    pub fn get_surface_texture(&self) -> Option<SurfaceTexture> {
        if self.swapchain.is_valid() {
            Some(self.swapchain.clone())
        } else {
            None
        }
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        if std::thread::panicking() {
            dbg_log!("Dropping command buffer while panicking");
            return;
        }

        self.end(true);
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceTextureInner {
    pub(crate) texture: Option<wgpu::SurfaceTexture>,
    pub(crate) suboptimal: bool,
    pub(crate) presented: bool,
}

#[derive(Clone, Debug)]
pub struct SurfaceTexture {
    pub(crate) inner: ArcRef<SurfaceTextureInner>,
}

impl SurfaceTexture {
    pub(crate) fn new() -> SurfaceTexture {
        SurfaceTexture {
            inner: ArcRef::new(SurfaceTextureInner {
                texture: None,
                suboptimal: false,
                presented: false,
            }),
        }
    }

    pub(crate) fn set_texture(&mut self, texture: wgpu::SurfaceTexture) {
        let mut inner = self.inner.borrow_mut();
        inner.suboptimal = texture.suboptimal;
        inner.texture = Some(texture);
        inner.presented = false;
    }

    pub fn get_view(&self) -> wgpu::TextureView {
        let inner = self.inner.borrow();
        inner.texture.as_ref().map_or_else(
            || {
                panic!("SurfaceTexture has no texture");
            },
            |texture| {
                texture.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Surface Texture View"),
                    ..Default::default()
                })
            },
        )
    }

    pub fn get_size(&self) -> wgpu::Extent3d {
        let inner = self.inner.borrow();
        inner.texture.as_ref().map_or_else(
            || {
                panic!("SurfaceTexture has no texture");
            },
            |texture| texture.texture.size(),
        )
    }

    pub fn get_format(&self) -> wgpu::TextureFormat {
        let inner = self.inner.borrow();
        inner.texture.as_ref().map_or_else(
            || {
                panic!("SurfaceTexture has no texture");
            },
            |texture| texture.texture.format(),
        )
    }

    pub fn is_valid(&self) -> bool {
        let inner = self.inner.borrow();
        inner.texture.is_some()
    }

    pub fn is_suboptimal(&self) -> bool {
        let inner = self.inner.borrow();
        inner.suboptimal
    }

    pub fn present(&mut self) {
        let mut inner = self.inner.borrow_mut();
        if let Some(texture) = inner.texture.take() {
            texture.present();
            inner.presented = true;
        }
    }
}
