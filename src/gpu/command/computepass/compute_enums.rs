use crate::gpu::{BindGroupLayout, ComputePipeline};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum DispatchType {
    Dispatch { x: u32, y: u32, z: u32 },
    DispatchIndirect { buffer: wgpu::Buffer, offset: u64 },
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct IntermediateComputeBinding {
    pub shader: wgpu::ShaderModule,
    pub layout: Vec<BindGroupLayout>,
    pub entry_point: String,
}

#[derive(Clone, Debug)]
pub(crate) enum ComputeShaderBinding {
    Intermediate(IntermediateComputeBinding),
    Pipeline(ComputePipeline),
}

#[derive(Clone, Debug)]
pub enum ComputePassBuildError {
    None
}