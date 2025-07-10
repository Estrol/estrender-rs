use std::sync::{atomic::AtomicBool, Arc};

use wgpu::CommandEncoder;

#[cfg(any(debug_assertions, feature = "enable-release-validation"))]
use crate::gpu::ShaderReflect;
use crate::{gpu::{BindGroupAttachment, ComputePassQueue, ComputeShaderBinding}, utils::ArcRef};

#[derive(Clone, Debug)]
pub(crate) struct ComputePassInner {
    pub cmd: ArcRef<CommandEncoder>,
    pub shader: Option<ComputeShaderBinding>,
    pub atomic_pass: Arc<AtomicBool>,

    pub queues: Vec<ComputePassQueue>,
    pub attachments: Vec<BindGroupAttachment>,
    pub push_constant: Option<Vec<u8>>,

    #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
    pub reflection: Option<ShaderReflect>,
}