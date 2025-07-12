pub(crate) mod bind_group_manager;
pub(crate) mod compute;
pub(crate) mod graphics;
pub(crate) mod reflection;
pub(crate) mod types;

pub use compute::{
    ComputeShader,
    ComputeShaderBuilder,
};

pub use graphics::{
    GraphicsShader,
    GraphicsShaderBuilder,
};

pub use types::{
    ShaderTopology,
    ShaderCullMode,
    ShaderPollygonMode,
    ShaderFrontFace,
    StorageAccess,
    ShaderBindingType,
    IndexBufferSize,
    ShaderBindingInfo,
    VertexInputType,
    VertexInputAttribute,
    VertexInputDesc,
    BindGroupLayout,
};

pub use reflection::is_shader_valid;
