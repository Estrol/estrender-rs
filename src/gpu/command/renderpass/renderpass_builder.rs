use std::sync::{Arc, atomic::AtomicBool};

use wgpu::CommandEncoder;

use crate::{
    gpu::{
        RenderPass, RenderPassBuildError, SampleCount, SurfaceTexture, Texture, TextureBlend,
        TextureFormat, TextureUsage, gpu_inner::GPUInner, renderpass_inner::RenderpassRenderTarget,
    },
    math::Point2,
    utils::ArcRef,
};

#[derive(Clone, Debug)]
pub(crate) enum RenderpassAttachment<'a> {
    SurfaceTexture(&'a SurfaceTexture),
    Texture(&'a Texture),
}

#[derive(Clone, Debug)]
pub struct RenderpassBuilder<'a> {
    gpu: ArcRef<GPUInner>,
    cmd: ArcRef<CommandEncoder>,
    atomic_pass: Arc<AtomicBool>,

    color_attachments: Vec<(RenderpassAttachment<'a>, Option<TextureBlend>)>,
    msaa_attachments: Vec<&'a Texture>,
    depth_attachment: Option<&'a Texture>,
}

impl<'a> RenderpassBuilder<'a> {
    pub(crate) fn new(
        gpu: ArcRef<GPUInner>,
        cmd: ArcRef<CommandEncoder>,
        atomic_pass: Arc<AtomicBool>,
    ) -> Self {
        Self {
            gpu,
            cmd,
            atomic_pass,

            color_attachments: Vec::new(),
            msaa_attachments: Vec::new(),
            depth_attachment: None,
        }
    }

    /// Add swapchain's SurfaceTexture color attachment.
    pub fn add_surface_color_attachment(
        mut self,
        surface: &'a SurfaceTexture,
        blend: Option<&TextureBlend>,
    ) -> Self {
        self.color_attachments.push((
            RenderpassAttachment::SurfaceTexture(surface),
            blend.cloned(),
        ));

        self
    }

    pub fn add_color_attachment(
        mut self,
        texture: &'a Texture,
        blend: Option<&TextureBlend>,
    ) -> Self {
        self.color_attachments
            .push((RenderpassAttachment::Texture(texture), blend.cloned()));

        self
    }

    pub fn add_msaa_attachment(mut self, texture: &'a Texture) -> Self {
        self.msaa_attachments.push(texture);

        self
    }

    pub fn set_depth_attachment(mut self, texture: &'a Texture) -> Self {
        self.depth_attachment = Some(texture);

        self
    }

    pub fn build(self) -> Result<RenderPass, RenderPassBuildError> {
        let mut surface_size = None;

        let mut color_attachments = Vec::with_capacity(self.color_attachments.len());
        for (attachment, blend) in self.color_attachments {
            let (view, format, size) = match attachment {
                RenderpassAttachment::SurfaceTexture(surface_texture) => {
                    let view = surface_texture.get_view();
                    let format = surface_texture.get_format();
                    let size = surface_texture.get_size();

                    (view, format, Point2::new(size.width, size.height))
                }
                RenderpassAttachment::Texture(texture) => {
                    let texture_inner = texture.inner.borrow();

                    if !texture_inner
                        .usages
                        .contains(TextureUsage::RenderAttachment)
                    {
                        return Err(RenderPassBuildError::ColorAttachmentNotRenderTarget);
                    }

                    if texture_inner.size.x == 0 || texture_inner.size.y == 0 {
                        return Err(RenderPassBuildError::MismatchedAttachmentSize(
                            Point2::new(0.0, 0.0),
                            texture_inner.size,
                        ));
                    }

                    if texture_inner.sample_count != SampleCount::SampleCount1 {
                        return Err(RenderPassBuildError::ColorAttachmentMultiSampled);
                    }

                    (
                        texture_inner.wgpu_view.clone(),
                        texture_inner.format.into(),
                        texture_inner.size,
                    )
                }
            };

            if surface_size.is_some() {
                let surface_size = surface_size.unwrap();
                if surface_size != size {
                    return Err(RenderPassBuildError::MismatchedAttachmentSize(
                        surface_size,
                        size,
                    ));
                }
            }

            if surface_size.is_none() {
                surface_size = Some(size);
            }

            color_attachments.push(RenderpassRenderTarget {
                view,
                format,
                blend: blend.map(|b| b.create_wgpu_blend_state()),
                write_mask: blend.map(|b| b.create_wgpu_color_write_mask()),
            });
        }

        let mut multi_sample_target = Vec::with_capacity(self.msaa_attachments.len());
        let mut multi_sample_count = None;

        for msaa_texture in self.msaa_attachments {
            let texture_inner = msaa_texture.inner.borrow();

            if !texture_inner
                .usages
                .contains(TextureUsage::RenderAttachment)
            {
                return Err(RenderPassBuildError::MsaaTextureNotRenderAttachment);
            }

            if texture_inner.sample_count == SampleCount::SampleCount1 {
                return Err(RenderPassBuildError::MsaaTextureNotMultiSampled);
            }

            if texture_inner.size.x == 0 || texture_inner.size.y == 0 {
                return Err(RenderPassBuildError::MsaaTextureInvalidSize(Point2::new(
                    0.0, 0.0,
                )));
            }

            if surface_size.is_some() {
                let surface_size = surface_size.unwrap();
                if surface_size != texture_inner.size {
                    return Err(RenderPassBuildError::MismatchedAttachmentSize(
                        surface_size,
                        texture_inner.size,
                    ));
                }
            }

            let sample_count: u32 = texture_inner.sample_count.into();

            if multi_sample_count.is_some() && multi_sample_count.unwrap() != sample_count {
                return Err(RenderPassBuildError::MismatchedAttachmentSampleCount(
                    multi_sample_count.unwrap(),
                    sample_count,
                ));
            }

            if multi_sample_count.is_none() {
                multi_sample_count = Some(sample_count);
            }

            multi_sample_target.push(texture_inner.wgpu_view.clone());
        }

        let mut depth_view = None;
        let mut depth_format = None;

        if let Some(depth_texture) = self.depth_attachment {
            let texture_inner = depth_texture.inner.borrow();

            if !texture_inner
                .usages
                .contains(TextureUsage::RenderAttachment)
            {
                return Err(RenderPassBuildError::DepthTextureNotRenderAttachment);
            }

            if texture_inner.size.x == 0 || texture_inner.size.y == 0 {
                return Err(RenderPassBuildError::DepthTextureInvalidSize(Point2::new(
                    0.0, 0.0,
                )));
            }

            if texture_inner.format != TextureFormat::Depth32Float
                && texture_inner.format != TextureFormat::Depth24PlusStencil8
            {
                return Err(RenderPassBuildError::DepthTextureFormatNotSupported(
                    texture_inner.format,
                ));
            }

            if surface_size.is_some() {
                let surface_size = surface_size.unwrap();
                if surface_size != texture_inner.size {
                    return Err(RenderPassBuildError::MismatchedAttachmentSize(
                        surface_size,
                        texture_inner.size,
                    ));
                }
            }

            if surface_size.is_none() {
                surface_size = Some(texture_inner.size);
            }

            depth_view = Some(texture_inner.wgpu_view.clone());
            depth_format = Some(texture_inner.format.into());
        }

        if surface_size.is_none() {
            return Err(RenderPassBuildError::NoColorOrDepthAttachment);
        }

        let renderpass = RenderPass::new(self.gpu, self.cmd, self.atomic_pass);
        {
            let mut inner = renderpass.inner.borrow_mut();

            inner.render_targets = color_attachments;
            inner.multi_sample_target = multi_sample_target;
            inner.multi_sample_count = multi_sample_count;
            inner.depth_target = depth_view;
            inner.depth_target_format = depth_format;
            inner.surface_size = surface_size.unwrap();
        }

        Ok(renderpass)
    }
}
