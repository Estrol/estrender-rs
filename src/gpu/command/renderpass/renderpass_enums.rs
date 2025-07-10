use std::ops::Range;

use crate::{
    gpu::{
        BindGroupLayout, IndexBufferSize, RenderPipeline, ShaderCullMode, ShaderFrontFace,
        ShaderPollygonMode, ShaderTopology, TextureFormat,
    },
    math::{Point2, RectF},
};

pub enum RenderPassBuildError {
    NoColorOrDepthAttachment,
    ColorAttachmentNotRenderTarget,
    ColorAttachmentMultiSampled,
    MismatchedAttachmentCount(usize, usize),
    MismatchedAttachmentSize(Point2, Point2),
    MismatchedAttachmentSampleCount(u32, u32),
    MismatchedAttachmentFormat(TextureFormat, TextureFormat),
    MsaaTextureNotMultiSampled,
    MsaaTextureNotRenderAttachment,
    MsaaTextureInvalidSize(Point2),
    DepthTextureNotRenderAttachment,
    DepthTextureInvalidSize(Point2),
    DepthTextureFormatNotSupported(TextureFormat),
    SwapchainError(String),
}

impl std::fmt::Display for RenderPassBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderPassBuildError::NoColorOrDepthAttachment => write!(f, "No color attachment provided"),
            RenderPassBuildError::ColorAttachmentNotRenderTarget => {
                write!(f, "Color attachment is not a render target")
            }
            RenderPassBuildError::ColorAttachmentMultiSampled => {
                write!(f, "Color attachment is multi-sampled")
            }
            RenderPassBuildError::MismatchedAttachmentCount(expected, actual) => {
                write!(f, "Expected {} attachments, but got {}", expected, actual)
            }
            RenderPassBuildError::MismatchedAttachmentSize(expected, actual) => write!(
                f,
                "Expected attachment size {:?}, but got {:?}",
                expected, actual
            ),
            RenderPassBuildError::MismatchedAttachmentSampleCount(expected, actual) => {
                write!(f, "Expected sample count {}, but got {}", expected, actual)
            }
            RenderPassBuildError::MismatchedAttachmentFormat(expected, actual) => {
                write!(f, "Expected format {:?}, but got {:?}", expected, actual)
            }
            RenderPassBuildError::MsaaTextureNotMultiSampled => {
                write!(f, "MSAA texture is not multi-sampled")
            }
            RenderPassBuildError::MsaaTextureNotRenderAttachment => {
                write!(f, "MSAA texture is not a render attachment")
            }
            RenderPassBuildError::MsaaTextureInvalidSize(size) => {
                write!(f, "MSAA texture has invalid size {:?}", size)
            }
            RenderPassBuildError::DepthTextureNotRenderAttachment => {
                write!(f, "Depth texture is not a render attachment")
            }
            RenderPassBuildError::DepthTextureInvalidSize(size) => {
                write!(f, "Depth texture has invalid size {:?}", size)
            }
            RenderPassBuildError::DepthTextureFormatNotSupported(format) => {
                write!(f, "Depth texture format {:?} is not supported", format)
            }
            RenderPassBuildError::SwapchainError(err) => write!(f, "Swapchain error: {}", err),
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct IntermediateRenderPipeline {
    pub shader: (wgpu::ShaderModule, wgpu::ShaderModule),
    pub vertex_attribute: (u64, Vec<wgpu::VertexAttribute>),
    pub shader_entry: (String, String),
    pub layout: Vec<BindGroupLayout>,
    pub topology: ShaderTopology,
    pub cull_mode: Option<ShaderCullMode>,
    pub front_face: ShaderFrontFace,
    pub polygon_mode: ShaderPollygonMode,
    pub index_format: Option<IndexBufferSize>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderPassQueue {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: Vec<(u32, wgpu::BindGroup)>,

    pub vbo: Option<wgpu::Buffer>,
    pub ibo: Option<wgpu::Buffer>,
    pub itype: Option<wgpu::IndexFormat>,

    pub viewport: Option<(RectF, f32, f32)>,
    pub scissor: Option<RectF>,

    pub ty: DrawCallType,
    pub push_constant: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub(crate) enum RenderShaderBinding {
    Intermediate(IntermediateRenderPipeline),
    Pipeline(RenderPipeline),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum BindGroupType {
    Uniform(wgpu::Buffer),
    Texture(wgpu::TextureView),
    TextureStorage(wgpu::TextureView),
    Sampler(wgpu::Sampler),
    Storage(wgpu::Buffer),
}

impl std::fmt::Display for BindGroupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindGroupType::Uniform(_) => write!(f, "Uniform"),
            BindGroupType::Texture(_) => write!(f, "Texture"),
            BindGroupType::TextureStorage(_) => write!(f, "TextureStorage"),
            BindGroupType::Sampler(_) => write!(f, "Sampler"),
            BindGroupType::Storage(_) => write!(f, "Storage"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DrawCallType {
    Direct {
        ranges: Range<u32>,
        vertex_offset: i32,
        num_of_instances: u32,
    },

    InDirect {
        buffer: wgpu::Buffer,
        offset: u64,
    },
}
