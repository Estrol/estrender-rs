#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

impl Into<wgpu::PrimitiveTopology> for ShaderTopology {
    fn into(self) -> wgpu::PrimitiveTopology {
        match self {
            ShaderTopology::PointList => wgpu::PrimitiveTopology::PointList,
            ShaderTopology::LineList => wgpu::PrimitiveTopology::LineList,
            ShaderTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            ShaderTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            ShaderTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderCullMode {
    Front,
    Back,
}

impl Into<wgpu::Face> for ShaderCullMode {
    fn into(self) -> wgpu::Face {
        match self {
            ShaderCullMode::Front => wgpu::Face::Front,
            ShaderCullMode::Back => wgpu::Face::Back,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderPollygonMode {
    Fill,
    Line,
    Point,
}

impl Into<wgpu::PolygonMode> for ShaderPollygonMode {
    fn into(self) -> wgpu::PolygonMode {
        match self {
            ShaderPollygonMode::Fill => wgpu::PolygonMode::Fill,
            ShaderPollygonMode::Line => wgpu::PolygonMode::Line,
            ShaderPollygonMode::Point => wgpu::PolygonMode::Point,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderFrontFace {
    Clockwise,
    CounterClockwise,
}

impl Into<wgpu::FrontFace> for ShaderFrontFace {
    fn into(self) -> wgpu::FrontFace {
        match self {
            ShaderFrontFace::Clockwise => wgpu::FrontFace::Cw,
            ShaderFrontFace::CounterClockwise => wgpu::FrontFace::Ccw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StorageAccess(u32);

bitflags::bitflags! {
    impl StorageAccess: u32 {
        const READ = 0b0001;
        const WRITE = 0b0010;
        const ATOMIC = 0b0100;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderBindingType {
    UniformBuffer(u32),
    StorageBuffer(u32, StorageAccess),
    StorageTexture(StorageAccess),
    Sampler(bool),
    Texture(bool),
    PushConstant(u32),
}

impl std::fmt::Display for ShaderBindingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderBindingType::UniformBuffer(size) => write!(f, "UniformBuffer({})", size),
            ShaderBindingType::StorageBuffer(size, access) => {
                write!(f, "StorageBuffer({}, {:?})", size, access)
            }
            ShaderBindingType::StorageTexture(access) => {
                write!(f, "StorageTexture({:?})", access)
            }
            ShaderBindingType::Sampler(is_compare) => {
                write!(f, "Sampler({})", is_compare)
            }
            ShaderBindingType::Texture(is_storage) => {
                write!(f, "Texture({})", is_storage)
            }
            ShaderBindingType::PushConstant(size) => write!(f, "PushConstant({})", size),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexBufferSize {
    U16,
    U32,
}

impl Into<wgpu::IndexFormat> for IndexBufferSize {
    fn into(self) -> wgpu::IndexFormat {
        match self {
            IndexBufferSize::U16 => wgpu::IndexFormat::Uint16,
            IndexBufferSize::U32 => wgpu::IndexFormat::Uint32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShaderBindingInfo {
    pub binding: u32,
    pub group: u32,
    pub name: String,
    pub ty: ShaderBindingType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexInputType {
    Uint8,
    Uint8x2,
    Uint8x4,
    Sint8,
    Sint8x2,
    Sint8x4,
    Unorm8,
    Unorm8x2,
    Unorm8x4,
    Snorm8,
    Snorm8x2,
    Snorm8x4,
    Uint16,
    Uint16x2,
    Uint16x4,
    Sint16,
    Sint16x2,
    Sint16x4,
    Unorm16,
    Unorm16x2,
    Unorm16x4,
    Snorm16,
    Snorm16x2,
    Snorm16x4,
    Float16,
    Float16x2,
    Float16x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
    Sint32,
    Sint32x2,
    Sint32x3,
    Sint32x4,
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
}

impl Into<wgpu::VertexFormat> for VertexInputType {
    fn into(self) -> wgpu::VertexFormat {
        match self {
            VertexInputType::Uint8 => wgpu::VertexFormat::Uint8,
            VertexInputType::Uint8x2 => wgpu::VertexFormat::Uint8x2,
            VertexInputType::Uint8x4 => wgpu::VertexFormat::Uint8x4,
            VertexInputType::Sint8 => wgpu::VertexFormat::Sint8,
            VertexInputType::Sint8x2 => wgpu::VertexFormat::Sint8x2,
            VertexInputType::Sint8x4 => wgpu::VertexFormat::Sint8x4,
            VertexInputType::Unorm8 => wgpu::VertexFormat::Unorm8,
            VertexInputType::Unorm8x2 => wgpu::VertexFormat::Unorm8x2,
            VertexInputType::Unorm8x4 => wgpu::VertexFormat::Unorm8x4,
            VertexInputType::Snorm8 => wgpu::VertexFormat::Snorm8,
            VertexInputType::Snorm8x2 => wgpu::VertexFormat::Snorm8x2,
            VertexInputType::Snorm8x4 => wgpu::VertexFormat::Snorm8x4,
            VertexInputType::Uint16 => wgpu::VertexFormat::Uint16,
            VertexInputType::Uint16x2 => wgpu::VertexFormat::Uint16x2,
            VertexInputType::Uint16x4 => wgpu::VertexFormat::Uint16x4,
            VertexInputType::Sint16 => wgpu::VertexFormat::Sint16,
            VertexInputType::Sint16x2 => wgpu::VertexFormat::Sint16x2,
            VertexInputType::Sint16x4 => wgpu::VertexFormat::Sint16x4,
            VertexInputType::Unorm16 => wgpu::VertexFormat::Unorm16,
            VertexInputType::Unorm16x2 => wgpu::VertexFormat::Unorm16x2,
            VertexInputType::Unorm16x4 => wgpu::VertexFormat::Unorm16x4,
            VertexInputType::Snorm16 => wgpu::VertexFormat::Snorm16,
            VertexInputType::Snorm16x2 => wgpu::VertexFormat::Snorm16x2,
            VertexInputType::Snorm16x4 => wgpu::VertexFormat::Snorm16x4,
            VertexInputType::Float16 => wgpu::VertexFormat::Float16,
            VertexInputType::Float16x2 => wgpu::VertexFormat::Float16x2,
            VertexInputType::Float16x4 => wgpu::VertexFormat::Float16x4,
            VertexInputType::Uint32 => wgpu::VertexFormat::Uint32,
            VertexInputType::Uint32x2 => wgpu::VertexFormat::Uint32x2,
            VertexInputType::Uint32x3 => wgpu::VertexFormat::Uint32x3,
            VertexInputType::Uint32x4 => wgpu::VertexFormat::Uint32x4,
            VertexInputType::Sint32 => wgpu::VertexFormat::Sint32,
            VertexInputType::Sint32x2 => wgpu::VertexFormat::Sint32x2,
            VertexInputType::Sint32x3 => wgpu::VertexFormat::Sint32x3,
            VertexInputType::Sint32x4 => wgpu::VertexFormat::Sint32x4,
            VertexInputType::Float32 => wgpu::VertexFormat::Float32,
            VertexInputType::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexInputType::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexInputType::Float32x4 => wgpu::VertexFormat::Float32x4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexInputAttribute {
    pub shader_location: u32,
    pub offset: u64,
    pub format: VertexInputType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexInputDesc {
    pub stride: u64,
    pub attributes: Vec<VertexInputAttribute>,
}

#[derive(Debug, Clone, Eq, Hash)]
pub enum ShaderReflect {
    Vertex {
        entry_point: String,
        input: Option<VertexInputReflection>,
        bindings: Vec<ShaderBindingInfo>,
    },
    Fragment {
        entry_point: String,
        bindings: Vec<ShaderBindingInfo>,
    },
    VertexFragment {
        vertex_entry_point: String,
        vertex_input: Option<VertexInputReflection>,
        fragment_entry_point: String,
        bindings: Vec<ShaderBindingInfo>,
    },
    Compute {
        entry_point: String,
        bindings: Vec<ShaderBindingInfo>,
    },
}

impl PartialEq for ShaderReflect {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                ShaderReflect::Vertex {
                    entry_point,
                    input,
                    bindings,
                },
                ShaderReflect::Vertex {
                    entry_point: other_entry_point,
                    input: other_input,
                    bindings: other_bindings,
                },
            ) => {
                entry_point == other_entry_point
                    && input == other_input
                    && bindings == other_bindings
            }
            (
                ShaderReflect::Fragment {
                    entry_point,
                    bindings,
                },
                ShaderReflect::Fragment {
                    entry_point: other_entry_point,
                    bindings: other_bindings,
                },
            ) => entry_point == other_entry_point && bindings == other_bindings,
            (
                ShaderReflect::VertexFragment {
                    vertex_entry_point,
                    vertex_input,
                    fragment_entry_point,
                    bindings,
                },
                ShaderReflect::VertexFragment {
                    vertex_entry_point: other_vertex_entry_point,
                    vertex_input: other_vertex_input,
                    fragment_entry_point: other_fragment_entry_point,
                    bindings: other_bindings,
                },
            ) => {
                vertex_entry_point == other_vertex_entry_point
                    && vertex_input == other_vertex_input
                    && fragment_entry_point == other_fragment_entry_point
                    && bindings == other_bindings
            }
            (
                ShaderReflect::Compute {
                    entry_point,
                    bindings,
                },
                ShaderReflect::Compute {
                    entry_point: other_entry_point,
                    bindings: other_bindings,
                },
            ) => entry_point == other_entry_point && bindings == other_bindings,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindGroupLayout {
    pub group: u32,
    pub bindings: Vec<u32>,
    pub layout: wgpu::BindGroupLayout,
}

#[derive(Debug, Clone, Eq, Hash)]
pub struct VertexInputReflection {
    pub name: String,
    pub stride: u64,
    pub attributes: Vec<(u32, u64, VertexInputType)>,
}

impl PartialEq for VertexInputReflection {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.stride == other.stride
            && self.attributes == other.attributes
    }
}
