//! Command

#[cfg(any(debug_assertions, feature = "enable-release-validation"))]
use std::sync::atomic::Ordering;

use std::sync::{atomic::AtomicBool, Arc};

use crate::utils::ArcRef;
use super::{
    GPUInner,
    SwapchainError,
    texture::{Texture, BlendState},
    buffer::Buffer,
};

pub(crate) mod renderpass;
pub(crate) mod computepass;
pub(crate) mod drawing;
pub(crate) mod utils;

use renderpass::{
    RenderPass, RenderPassBuildError, RenderpassBuilder,
};

use utils::BindGroupType;

use computepass::{ComputePass, ComputePassBuildError};
use wgpu::util::TextureBlitter;

pub enum PassAttachment {
    Texture(Texture, BlendState),
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
pub enum CommandBufferBuildError {
    None
}

#[derive(Clone, Debug)]
pub struct CommandBuffer {
    pub(crate) inner: ArcRef<GPUInner>,

    pub(crate) command: Option<ArcRef<wgpu::CommandEncoder>>,
    pub(crate) on_renderpass: Arc<AtomicBool>,
    pub(crate) on_compute: Arc<AtomicBool>,

    pub(crate) swapchain: SurfaceTexture,
}

impl CommandBuffer {
    pub(crate) fn new(inner: ArcRef<GPUInner>) -> Result<Self, CommandBufferBuildError> {
        let inner_ref = inner.borrow();
        let command =
            inner_ref
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        drop(inner_ref);

        Ok(Self {
            inner,
            command: Some(ArcRef::new(command)),
            on_renderpass: Arc::new(AtomicBool::new(false)),
            on_compute: Arc::new(AtomicBool::new(false)),

            swapchain: SurfaceTexture::new(),
        })
    }

    pub(crate) fn new_with_surface(
        inner: ArcRef<GPUInner>,
        surface: SurfaceTexture,
    ) -> Result<Self, CommandBufferBuildError> {
        let inner_ref = inner.borrow();
        let command =
            inner_ref
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        drop(inner_ref);

        Ok(Self {
            inner,
            command: Some(ArcRef::new(command)),
            on_renderpass: Arc::new(AtomicBool::new(false)),
            on_compute: Arc::new(AtomicBool::new(false)),

            swapchain: surface,
        })
    }

    /// Creates a new renderpass builder.
    /// 
    /// This function is used to create a renderpass builder that can be used to
    /// configure and build a render pass.
    /// 
    /// This is useful when you want to create a render pass with specific
    /// configurations, such as adding color attachments, depth attachments, etc.
    pub fn renderpass_builder(&mut self) -> RenderpassBuilder {
        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_arc_ref = ArcRef::clone(self.command.as_ref().unwrap());
        let atomic_pass = Arc::clone(&self.on_renderpass);

        self.on_renderpass.store(true, Ordering::Relaxed);

        RenderpassBuilder::new(gpu_arc_ref, cmd_arc_ref, atomic_pass)
    }

    /// Begins a new graphics pass.
    /// 
    /// This function will initiate a renderpass with the swapchain's surface texture.
    /// If the swapchain is not valid, it will attempt to recreate it.
    /// 
    /// The swapchain will be used as the color attachment at index 0 with a default blend mode (NONE).
    pub fn begin_renderpass(&mut self) -> Result<RenderPass, RenderPassBuildError> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass.load(Ordering::Relaxed) || self.on_compute.load(Ordering::Relaxed) {
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
                    crate::log!("Swapchain error: {}", err);
                    return Err(RenderPassBuildError::SwapchainError(format!(
                        "Failed to create swapchain: {}",
                        err
                    )));
                }
            }
        }

        self.on_renderpass.store(true, Ordering::Relaxed);

        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_arc_ref = ArcRef::clone(self.command.as_ref().unwrap());
        let atomic_pass = Arc::clone(&self.on_renderpass);

        RenderpassBuilder::new(gpu_arc_ref, cmd_arc_ref, atomic_pass)
            .add_surface_color_attachment(&self.swapchain, None)
            .build()
    }

    /// Begins a new graphics pass with a depth texture.
    ///
    /// This function is used to create a render pass with a depth texture for depth-only rendering.
    /// Which can be useful for shadow mapping or other depth-based effects.
    pub fn begin_depth_texture(
        &mut self,
        texture: &Texture,
    ) -> Result<RenderPass, RenderPassBuildError> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass.load(Ordering::Relaxed) || self.on_compute.load(Ordering::Relaxed) {
            panic!("CMD already in a render pass or compute pass");
        }

        self.on_renderpass.store(true, Ordering::Relaxed);

        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_arc_ref = ArcRef::clone(self.command.as_ref().unwrap());
        let atomic_pass = Arc::clone(&self.on_renderpass);

        RenderpassBuilder::new(gpu_arc_ref, cmd_arc_ref, atomic_pass)
            .set_depth_attachment(texture)
            .build()
    }

    /// Begins a new graphics pass to a render target texture.
    ///
    /// Can be used to render to a specific texture instead of the default swapchain.
    /// This is useful for offscreen rendering or rendering to a texture for later use.
    /// 
    /// The render texture will be placed at index 0 with a default blend mode (NONE).
    pub fn begin_texture<'a>(
        &'a mut self,
        texture: &'a Texture,
    ) -> Result<RenderPass, RenderPassBuildError> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass.load(Ordering::Relaxed) || self.on_compute.load(Ordering::Relaxed) {
            panic!("CMD already in a render pass or compute pass");
        }

        self.on_renderpass.store(false, Ordering::Relaxed);

        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_arc_ref = ArcRef::clone(self.command.as_ref().unwrap());
        let atomic_pass = Arc::clone(&self.on_renderpass);

        RenderpassBuilder::new(gpu_arc_ref, cmd_arc_ref, atomic_pass)
            .add_color_attachment(texture, None)
            .build()
    }

    /// Begins a new compute pass.
    pub fn begin_computepass(&mut self) -> Result<ComputePass, ComputePassBuildError> {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        if self.on_renderpass.load(Ordering::Relaxed) || self.on_compute.load(Ordering::Relaxed) {
            panic!("CMD already in a render pass or compute pass");
        }

        self.on_renderpass.store(false, Ordering::Relaxed);

        let gpu_arc_ref = ArcRef::clone(&self.inner);
        let cmd_ref = ArcRef::clone(self.command.as_ref().unwrap());
        let atomic_pass = Arc::clone(&self.on_compute);

        ComputePass::new(gpu_arc_ref, cmd_ref, atomic_pass)
    }

    /// Writes a buffer to a destination buffer.
    ///
    /// This is useful to copy from compute buffers or other buffers
    /// to a destination buffer, such as when you want to update a buffer with new data
    pub fn write_buffer(&mut self, src: &Buffer, dst: &Buffer) {
        dst.write_cmd(src, self);
    }

    /// Writes a buffer to a destination buffer with raw data.
    ///
    /// Will panic if the data is not compatible with the destination buffer, such
    /// in the case of mismatched sizes or types.
    pub fn write_buffer_raw<T: bytemuck::Pod>(&mut self, data: &[T], dst: &Buffer) {
        dst.write_raw_cmd(data, self);
    }

    /// Copies a source texture to a destination texture.
    ///
    /// This function uses a texture blitter to perform the copy operation, such copying
    /// between different texture formats or sizes (ex from a render target to a texture).
    pub fn blit_texture(&mut self, src: &Texture, dst: &Texture) {
        let gpu_inner = self.inner.borrow();
        let mut cmd = self.command.as_ref().unwrap().borrow_mut();

        let blitter = {
            let dst_format = dst.inner.borrow().format;

            TextureBlitter::new(gpu_inner.device(), dst_format.into())
        };

        let src_view = &src.inner.borrow().wgpu_view;

        let dst_view = &dst.inner.borrow().wgpu_view;

        blitter.copy(&gpu_inner.device(), &mut cmd, src_view, dst_view);
    }

    /// Copies a source texture to a destination texture.
    ///
    /// The 'src' texture must be compatible with the 'dst' texture in format and size.
    ///
    /// This is useful for copying textures that are already in the GPU memory,
    /// such as when you want to copy a texture from one render target to another.
    pub fn copy_texture(&mut self, src: &Texture, dst: &Texture) {
        let mut cmd = self.command.as_ref().unwrap().borrow_mut();

        // Make sure src and dst texture format and size are compatible
        let src_inner = src.inner.borrow();
        let dst_inner = dst.inner.borrow();

        if src_inner.format != dst_inner.format {
            panic!("Source and destination textures must have the same format");
        }

        if src_inner.size != dst_inner.size {
            panic!("Source and destination textures must have the same size");
        }

        if src_inner.wgpu_texture.mip_level_count() != 1 {
            panic!("Source texture must have only one mip level");
        }

        if dst_inner.wgpu_texture.mip_level_count() != 1 {
            panic!("Destination texture must have only one mip level");
        }

        let src_tex = &src_inner.wgpu_texture;
        let dst_tex = &dst_inner.wgpu_texture;

        cmd.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: src_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfoBase {
                texture: dst_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: src_inner.size.x as u32,
                height: src_inner.size.y as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn end(&mut self, present: bool) {
        let inner_ref = self.inner.borrow();

        if self.command.is_none() {
            return;
        }

        let cmd = ArcRef::try_unwrap(self.command.take().unwrap()).unwrap_or_else(|_| {
            panic!("Command buffer dropped while still in use");
        });

        inner_ref.queue().submit(std::iter::once(cmd.finish()));

        if present {
            self.swapchain.present();
        }
    }

    /// Returns the current surface texture handle.
    /// 
    /// This will attach current swapchain texture to the command buffer if it is not already set.
    pub fn get_surface_texture(&mut self) -> Result<SurfaceTexture, SurfaceTextureError> {
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
                    match err {
                        SwapchainError::NotAvailable => {
                            return Err(SurfaceTextureError::NotAvailable);
                        }
                        SwapchainError::ConfigNeeded => {
                            return Err(SurfaceTextureError::ConfigNeeded);
                        }
                        SwapchainError::DeviceLost => {
                            return Err(SurfaceTextureError::DeviceLost);
                        }
                        _ => {
                            crate::log!("Swapchain error: {}", err);
                            return Err(SurfaceTextureError::NotAvailable);
                        }
                    }
                }
            }
        }

        Ok(self.swapchain.clone())
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        if std::thread::panicking() {
            crate::dbg_log!("Dropping command buffer while panicking");
            return;
        }

        if self.on_renderpass.load(Ordering::Relaxed) || self.on_compute.load(Ordering::Relaxed) {
            panic!("Command buffer dropped while still in a render pass or compute pass");
        }

        self.end(true);
    }
}

#[derive(Clone, Debug)]
pub enum SurfaceTextureError {
    NotAvailable,
    ConfigNeeded,
    DeviceLost,
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceTextureInner {
    pub texture: Option<wgpu::SurfaceTexture>,
    pub suboptimal: bool,
    pub presented: bool,
}

/// Represents a texture handle that is used for rendering to the surface (swapchain).
/// 
/// This texture is created by the GPU and is used to present the rendered content to the screen.
/// Can be used with the [CommandBuffer] to render to the surface.
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
