use std::sync::{Arc, atomic::AtomicBool};

use wgpu::CommandEncoder;

#[cfg(any(debug_assertions, feature = "enable-release-validation"))]
use crate::gpu::ShaderReflect;
use crate::{
    gpu::{BindGroupAttachment, RenderPassQueue, RenderShaderBinding},
    math::{Color, Point2, RectF},
    utils::ArcRef,
};

#[derive(Debug, Clone)]
pub(crate) struct RenderpassRenderTarget {
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub blend: Option<wgpu::BlendState>,
    pub write_mask: Option<wgpu::ColorWrites>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderPassInner {
    pub cmd: ArcRef<CommandEncoder>,
    pub atomic_pass: Arc<AtomicBool>,

    pub render_targets: Vec<RenderpassRenderTarget>,
    pub depth_target: Option<wgpu::TextureView>,
    pub depth_target_format: Option<wgpu::TextureFormat>,

    pub surface_size: Point2,

    pub multi_sample_target: Vec<wgpu::TextureView>,
    pub multi_sample_count: Option<u32>,

    pub clear_color: Option<Color>,
    pub viewport: Option<(RectF, f32, f32)>,
    pub scissor: Option<RectF>,

    pub vertex: Option<wgpu::Buffer>,
    pub index: Option<wgpu::Buffer>,

    pub shader: Option<RenderShaderBinding>,
    #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
    pub shader_reflection: Option<Vec<ShaderReflect>>,

    pub attachments: Vec<BindGroupAttachment>,
    pub push_constant: Option<Vec<u8>>,

    pub queues: Vec<RenderPassQueue>,
}
