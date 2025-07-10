#[derive(Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextureUsage(u32);

bitflags::bitflags! {
    impl TextureUsage: u32 {
        const None = 0b00000000;
        const Sampler = 0b00000001;
        const Storage = 0b00000010;
        const RenderAttachment = 0b00000100;
    }
}

impl Into<wgpu::TextureUsages> for TextureUsage {
    fn into(self) -> wgpu::TextureUsages {
        let mut usage = wgpu::TextureUsages::empty();
        if self.contains(TextureUsage::Sampler) {
            usage |= wgpu::TextureUsages::TEXTURE_BINDING;
        }
        if self.contains(TextureUsage::Storage) {
            usage |= wgpu::TextureUsages::STORAGE_BINDING;
        }
        if self.contains(TextureUsage::RenderAttachment) {
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        usage
    }
}

#[derive(Clone, Debug, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SampleCount {
    SampleCount1,
    SampleCount2,
    SampleCount4,
    SampleCount8,
}

impl Into<u32> for SampleCount {
    fn into(self) -> u32 {
        match self {
            SampleCount::SampleCount1 => 1,
            SampleCount::SampleCount2 => 2,
            SampleCount::SampleCount4 => 4,
            SampleCount::SampleCount8 => 8,
        }
    }
}

#[derive(Clone, Debug, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(Clone, Debug, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    SrcAlphaSaturated,
    BlendColor,
    OneMinusBlendColor,
}

#[derive(Clone, Debug, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextureBlend {
    pub color_blend: BlendOperation,
    pub alpha_blend: BlendOperation,
    pub color_src_factor: BlendFactor,
    pub color_dst_factor: BlendFactor,
    pub alpha_src_factor: BlendFactor,
    pub alpha_dst_factor: BlendFactor,
    pub color_blend_constant: [u32; 4],
}

impl TextureBlend {
    pub fn new(
        color_blend: BlendOperation,
        alpha_blend: BlendOperation,
        color_src_factor: BlendFactor,
        color_dst_factor: BlendFactor,
        alpha_src_factor: BlendFactor,
        alpha_dst_factor: BlendFactor,
        color_blend_constant: [u32; 4],
    ) -> Self {
        Self {
            color_blend,
            alpha_blend,
            color_src_factor,
            color_dst_factor,
            alpha_src_factor,
            alpha_dst_factor,
            color_blend_constant,
        }
    }

    pub(crate) fn from_wgpu(
        state: Option<wgpu::BlendState>,
        color_write_mask: Option<wgpu::ColorWrites>,
    ) -> Self {
        let mut write_mask = [0x00, 0x00, 0x00, 0x00];
        if let Some(mask) = color_write_mask {
            if mask.contains(wgpu::ColorWrites::RED) {
                write_mask[0] = 0xFF;
            }
            if mask.contains(wgpu::ColorWrites::GREEN) {
                write_mask[1] = 0xFF;
            }
            if mask.contains(wgpu::ColorWrites::BLUE) {
                write_mask[2] = 0xFF;
            }
            if mask.contains(wgpu::ColorWrites::ALPHA) {
                write_mask[3] = 0xFF;
            }
        }

        let mut blend = Self::NONE;
        if let Some(state) = state {
            blend.color_blend = Self::wgpu_op_to_blend_op(state.color.operation);
            blend.alpha_blend = Self::wgpu_op_to_blend_op(state.alpha.operation);
            blend.color_src_factor = Self::wgpu_factor_to_blend_factor(state.color.src_factor);
            blend.color_dst_factor = Self::wgpu_factor_to_blend_factor(state.color.dst_factor);
            blend.alpha_src_factor = Self::wgpu_factor_to_blend_factor(state.alpha.src_factor);
            blend.alpha_dst_factor = Self::wgpu_factor_to_blend_factor(state.alpha.dst_factor);
        }

        blend.color_blend_constant = write_mask;

        blend
    }

    pub(crate) fn wgpu_op_to_blend_op(op: wgpu::BlendOperation) -> BlendOperation {
        match op {
            wgpu::BlendOperation::Add => BlendOperation::Add,
            wgpu::BlendOperation::Subtract => BlendOperation::Subtract,
            wgpu::BlendOperation::ReverseSubtract => BlendOperation::ReverseSubtract,
            wgpu::BlendOperation::Min => BlendOperation::Min,
            wgpu::BlendOperation::Max => BlendOperation::Max,
        }
    }

    pub(crate) fn wgpu_factor_to_blend_factor(factor: wgpu::BlendFactor) -> BlendFactor {
        match factor {
            wgpu::BlendFactor::Zero => BlendFactor::Zero,
            wgpu::BlendFactor::One => BlendFactor::One,
            wgpu::BlendFactor::Src => BlendFactor::SrcColor,
            wgpu::BlendFactor::OneMinusSrc => BlendFactor::OneMinusSrcColor,
            wgpu::BlendFactor::Dst => BlendFactor::DstColor,
            wgpu::BlendFactor::OneMinusDst => BlendFactor::OneMinusDstColor,
            wgpu::BlendFactor::SrcAlpha => BlendFactor::SrcAlpha,
            wgpu::BlendFactor::OneMinusSrcAlpha => BlendFactor::OneMinusSrcAlpha,
            wgpu::BlendFactor::DstAlpha => BlendFactor::DstAlpha,
            wgpu::BlendFactor::OneMinusDstAlpha => BlendFactor::OneMinusDstAlpha,
            wgpu::BlendFactor::SrcAlphaSaturated => BlendFactor::SrcAlphaSaturated,
            wgpu::BlendFactor::Constant => BlendFactor::ConstantColor,
            wgpu::BlendFactor::OneMinusConstant => BlendFactor::OneMinusConstantColor,
            _ => {
                panic!("Unsupported blend factor: {:?}", factor);
            }
        }
    }

    pub(crate) fn blend_op_convert(op: BlendOperation) -> wgpu::BlendOperation {
        match op {
            BlendOperation::Add => wgpu::BlendOperation::Add,
            BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
            BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
            BlendOperation::Min => wgpu::BlendOperation::Min,
            BlendOperation::Max => wgpu::BlendOperation::Max,
        }
    }

    pub(crate) fn blend_factor_convert(op: BlendFactor) -> wgpu::BlendFactor {
        match op {
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::SrcColor => wgpu::BlendFactor::Src,
            BlendFactor::OneMinusSrcColor => wgpu::BlendFactor::OneMinusSrc,
            BlendFactor::DstColor => wgpu::BlendFactor::Dst,
            BlendFactor::OneMinusDstColor => wgpu::BlendFactor::OneMinusDst,
            BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
            BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
            BlendFactor::SrcAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
            BlendFactor::BlendColor => wgpu::BlendFactor::Constant,
            BlendFactor::OneMinusBlendColor => wgpu::BlendFactor::OneMinusConstant,
            BlendFactor::ConstantColor => wgpu::BlendFactor::Constant,
            BlendFactor::OneMinusConstantColor => wgpu::BlendFactor::OneMinusConstant,
        }
    }

    pub const NONE: Self = Self {
        color_blend: BlendOperation::Add,
        alpha_blend: BlendOperation::Add,
        color_src_factor: BlendFactor::One,
        color_dst_factor: BlendFactor::Zero,
        alpha_src_factor: BlendFactor::One,
        alpha_dst_factor: BlendFactor::Zero,
        color_blend_constant: [0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub const ALPHA_BLEND: Self = Self {
        color_blend: BlendOperation::Add,
        alpha_blend: BlendOperation::Add,
        color_src_factor: BlendFactor::SrcAlpha,
        color_dst_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_src_factor: BlendFactor::One,
        alpha_dst_factor: BlendFactor::Zero,
        color_blend_constant: [0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub const ADDITIVE_BLEND: Self = Self {
        color_blend: BlendOperation::Add,
        alpha_blend: BlendOperation::Add,
        color_src_factor: BlendFactor::One,
        color_dst_factor: BlendFactor::One,
        alpha_src_factor: BlendFactor::One,
        alpha_dst_factor: BlendFactor::Zero,
        color_blend_constant: [0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub const MULTIPLY_BLEND: Self = Self {
        color_blend: BlendOperation::Add,
        alpha_blend: BlendOperation::Add,
        color_src_factor: BlendFactor::DstColor,
        color_dst_factor: BlendFactor::Zero,
        alpha_src_factor: BlendFactor::DstAlpha,
        alpha_dst_factor: BlendFactor::Zero,
        color_blend_constant: [0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub const MODULATE_BLEND: Self = Self {
        color_blend: BlendOperation::Add,
        alpha_blend: BlendOperation::Add,
        color_src_factor: BlendFactor::SrcColor,
        color_dst_factor: BlendFactor::DstColor,
        alpha_src_factor: BlendFactor::SrcAlpha,
        alpha_dst_factor: BlendFactor::DstAlpha,
        color_blend_constant: [0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub(crate) fn create_wgpu_blend_state(&self) -> wgpu::BlendState {
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: Self::blend_factor_convert(self.color_src_factor),
                dst_factor: Self::blend_factor_convert(self.color_dst_factor),
                operation: Self::blend_op_convert(self.color_blend),
            },
            alpha: wgpu::BlendComponent {
                src_factor: Self::blend_factor_convert(self.alpha_src_factor),
                dst_factor: Self::blend_factor_convert(self.alpha_dst_factor),
                operation: Self::blend_op_convert(self.alpha_blend),
            },
        }
    }

    pub(crate) fn create_wgpu_color_write_mask(&self) -> wgpu::ColorWrites {
        let write_mask = self.color_blend_constant[0]
            | self.color_blend_constant[1] << 8
            | self.color_blend_constant[2] << 16
            | self.color_blend_constant[3] << 24;

        let mut mask = wgpu::ColorWrites::empty();
        if write_mask & 0x00000001 != 0 {
            mask |= wgpu::ColorWrites::RED;
        }
        if write_mask & 0x00000100 != 0 {
            mask |= wgpu::ColorWrites::GREEN;
        }
        if write_mask & 0x00010000 != 0 {
            mask |= wgpu::ColorWrites::BLUE;
        }
        if write_mask & 0x01000000 != 0 {
            mask |= wgpu::ColorWrites::ALPHA;
        }
        if mask.is_empty() {
            wgpu::ColorWrites::ALL
        } else {
            mask
        }
    }
}

impl Into<wgpu::BlendState> for TextureBlend {
    fn into(self) -> wgpu::BlendState {
        self.create_wgpu_blend_state()
    }
}

impl Into<wgpu::ColorWrites> for TextureBlend {
    fn into(self) -> wgpu::ColorWrites {
        self.create_wgpu_color_write_mask()
    }
}

#[derive(Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddressMode {
    ClampToEdge = 0,
    Repeat = 1,
    MirrorRepeat = 2,
    ClampToBorder = 3,
}

impl Into<wgpu::AddressMode> for AddressMode {
    fn into(self) -> wgpu::AddressMode {
        match self {
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            AddressMode::Repeat => wgpu::AddressMode::Repeat,
            AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
            AddressMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
        }
    }
}

#[derive(Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl Into<wgpu::FilterMode> for FilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

#[derive(Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

impl Into<wgpu::CompareFunction> for CompareFunction {
    fn into(self) -> wgpu::CompareFunction {
        match self {
            CompareFunction::Never => wgpu::CompareFunction::Never,
            CompareFunction::Less => wgpu::CompareFunction::Less,
            CompareFunction::Equal => wgpu::CompareFunction::Equal,
            CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
            CompareFunction::Greater => wgpu::CompareFunction::Greater,
            CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
            CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
            CompareFunction::Always => wgpu::CompareFunction::Always,
        }
    }
}

#[derive(Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SamplerBorderColor {
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

impl Into<wgpu::SamplerBorderColor> for SamplerBorderColor {
    fn into(self) -> wgpu::SamplerBorderColor {
        match self {
            SamplerBorderColor::TransparentBlack => wgpu::SamplerBorderColor::TransparentBlack,
            SamplerBorderColor::OpaqueBlack => wgpu::SamplerBorderColor::OpaqueBlack,
            SamplerBorderColor::OpaqueWhite => wgpu::SamplerBorderColor::OpaqueWhite,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TextureSampler {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<CompareFunction>,
    pub anisotropy_clamp: Option<u16>,
    pub border_color: Option<SamplerBorderColor>,
}

impl TextureSampler {
    pub fn new(
        address_mode_u: AddressMode,
        address_mode_v: AddressMode,
        address_mode_w: AddressMode,
        mag_filter: FilterMode,
        min_filter: FilterMode,
        mipmap_filter: FilterMode,
        lod_min_clamp: f32,
        lod_max_clamp: f32,
        compare: Option<CompareFunction>,
        anisotropy_clamp: Option<u16>,
        border_color: Option<SamplerBorderColor>,
    ) -> Self {
        Self {
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            compare,
            anisotropy_clamp,
            border_color,
        }
    }

    pub fn make_wgpu(&self, device: &wgpu::Device) -> wgpu::Sampler {
        let desc = wgpu::SamplerDescriptor {
            label: Some("texture sampler"),
            address_mode_u: self.address_mode_u.into(),
            address_mode_v: self.address_mode_v.into(),
            address_mode_w: self.address_mode_w.into(),
            mag_filter: self.mag_filter.into(),
            min_filter: self.min_filter.into(),
            mipmap_filter: self.mipmap_filter.into(),
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare.map(|x| x.into()),
            anisotropy_clamp: self.anisotropy_clamp.unwrap_or(1u16),
            border_color: self.border_color.map(|x| x.into()),
        };

        device.create_sampler(&desc)
    }

    pub const DEFAULT: Self = Self {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 1000.0,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
    };
}

impl Eq for TextureSampler {}

impl PartialEq for TextureSampler {
    fn eq(&self, other: &Self) -> bool {
        self.address_mode_u == other.address_mode_u
            && self.address_mode_v == other.address_mode_v
            && self.address_mode_w == other.address_mode_w
            && self.mag_filter == other.mag_filter
            && self.min_filter == other.min_filter
            && self.mipmap_filter == other.mipmap_filter
            && self.lod_min_clamp == other.lod_min_clamp
            && self.lod_max_clamp == other.lod_max_clamp
            && self.compare == other.compare
            && self.anisotropy_clamp == other.anisotropy_clamp
            && self.border_color == other.border_color
    }
}

#[derive(Debug, Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureFormat {
    // Normal 8 bit formats
    /// Red channel only. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    R8Unorm,
    /// Red channel only. 8 bit integer per channel. [-127, 127] converted to/from float [-1, 1] in shader.
    R8Snorm,
    /// Red channel only. 8 bit integer per channel. Unsigned in shader.
    R8Uint,
    /// Red channel only. 8 bit integer per channel. Signed in shader.
    R8Sint,

    // Normal 16 bit formats
    /// Red channel only. 16 bit integer per channel. Unsigned in shader.
    R16Uint,
    /// Red channel only. 16 bit integer per channel. Signed in shader.
    R16Sint,
    /// Red channel only. 16 bit float per channel. Float in shader.
    R16Float,
    /// Red and green channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Rg8Unorm,
    /// Red and green channels. 8 bit integer per channel. [-127, 127] converted to/from float [-1, 1] in shader.
    Rg8Snorm,
    /// Red and green channels. 8 bit integer per channel. Unsigned in shader.
    Rg8Uint,
    /// Red and green channels. 8 bit integer per channel. Signed in shader.
    Rg8Sint,

    // Normal 32 bit formats
    /// Red channel only. 32 bit integer per channel. Unsigned in shader.
    R32Uint,
    /// Red channel only. 32 bit integer per channel. Signed in shader.
    R32Sint,
    /// Red channel only. 32 bit float per channel. Float in shader.
    R32Float,
    /// Red and green channels. 16 bit integer per channel. Unsigned in shader.
    Rg16Uint,
    /// Red and green channels. 16 bit integer per channel. Signed in shader.
    Rg16Sint,
    /// Red and green channels. 16 bit float per channel. Float in shader.
    Rg16Float,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Rgba8Unorm,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    Rgba8UnormSrgb,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. [-127, 127] converted to/from float [-1, 1] in shader.
    Rgba8Snorm,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Unsigned in shader.
    Rgba8Uint,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Signed in shader.
    Rgba8Sint,
    /// Blue, green, red, and alpha channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Bgra8Unorm,
    /// Blue, green, red, and alpha channels. 8 bit integer per channel. Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    Bgra8UnormSrgb,

    // Packed 32 bit formats
    /// Packed unsigned float with 9 bits mantisa for each RGB component, then a common 5 bits exponent
    Rgb9e5Ufloat,
    /// Red, green, blue, and alpha channels. 10 bit integer for RGB channels, 2 bit integer for alpha channel. Unsigned in shader.
    Rgb10a2Uint,
    /// Red, green, blue, and alpha channels. 10 bit integer for RGB channels, 2 bit integer for alpha channel. [0, 1023] ([0, 3] for alpha) converted to/from float [0, 1] in shader.
    Rgb10a2Unorm,
    /// Red, green, and blue channels. 11 bit float with no sign bit for RG channels. 10 bit float with no sign bit for blue channel. Float in shader.
    Rg11b10Ufloat,

    // Normal 64 bit formats
    /// Red and green channels. 32 bit integer per channel. Unsigned in shader.
    Rg32Uint,
    /// Red and green channels. 32 bit integer per channel. Signed in shader.
    Rg32Sint,
    /// Red and green channels. 32 bit float per channel. Float in shader.
    Rg32Float,
    /// Red, green, blue, and alpha channels. 16 bit integer per channel. Unsigned in shader.
    Rgba16Uint,
    /// Red, green, blue, and alpha channels. 16 bit integer per channel. Signed in shader.
    Rgba16Sint,
    /// Red, green, blue, and alpha channels. 16 bit float per channel. Float in shader.
    Rgba16Float,

    // Normal 128 bit formats
    /// Red, green, blue, and alpha channels. 32 bit integer per channel. Unsigned in shader.
    Rgba32Uint,
    /// Red, green, blue, and alpha channels. 32 bit integer per channel. Signed in shader.
    Rgba32Sint,
    /// Red, green, blue, and alpha channels. 32 bit float per channel. Float in shader.
    Rgba32Float,

    // Depth and stencil formats
    /// Stencil format with 8 bit integer stencil.
    Stencil8,
    /// Special depth format with 16 bit integer depth.
    Depth16Unorm,
    /// Special depth format with at least 24 bit integer depth.
    Depth24Plus,
    /// Special depth/stencil format with at least 24 bit integer depth and 8 bits integer stencil.
    Depth24PlusStencil8,
    /// Special depth format with 32 bit floating point depth.
    Depth32Float,
    /// Special depth/stencil format with 32 bit floating point depth and 8 bits integer stencil.
    Depth32FloatStencil8,
}

impl TextureFormat {
    pub fn get_size(&self) -> u32 {
        match self {
            TextureFormat::R8Unorm => 1,
            TextureFormat::R8Snorm => 1,
            TextureFormat::R8Uint => 1,
            TextureFormat::R8Sint => 1,
            TextureFormat::R16Uint => 2,
            TextureFormat::R16Sint => 2,
            TextureFormat::R16Float => 2,
            TextureFormat::Rg8Unorm => 2,
            TextureFormat::Rg8Snorm => 2,
            TextureFormat::Rg8Uint => 2,
            TextureFormat::Rg8Sint => 2,
            TextureFormat::R32Uint => 4,
            TextureFormat::R32Sint => 4,
            TextureFormat::R32Float => 4,
            TextureFormat::Rg16Uint => 4,
            TextureFormat::Rg16Sint => 4,
            TextureFormat::Rg16Float => 4,
            TextureFormat::Rgba8Unorm => 4,
            TextureFormat::Rgba8UnormSrgb => 4,
            TextureFormat::Rgba8Snorm => 4,
            TextureFormat::Rgba8Uint => 4,
            TextureFormat::Rgba8Sint => 4,
            TextureFormat::Bgra8Unorm => 4,
            TextureFormat::Bgra8UnormSrgb => 4,
            TextureFormat::Rgb9e5Ufloat => 4,
            TextureFormat::Rgb10a2Uint => 4,
            TextureFormat::Rgb10a2Unorm => 4,
            TextureFormat::Rg11b10Ufloat => 4,
            TextureFormat::Rg32Uint => 8,
            TextureFormat::Rg32Sint => 8,
            TextureFormat::Rg32Float => 8,
            TextureFormat::Rgba16Uint => 8,
            TextureFormat::Rgba16Sint => 8,
            TextureFormat::Rgba16Float => 8,
            TextureFormat::Rgba32Uint => 16,
            TextureFormat::Rgba32Sint => 16,
            TextureFormat::Rgba32Float => 16,
            TextureFormat::Stencil8 => 1,
            TextureFormat::Depth16Unorm => 2,
            TextureFormat::Depth24Plus => 3,
            TextureFormat::Depth24PlusStencil8 => 4,
            TextureFormat::Depth32Float => 4,
            TextureFormat::Depth32FloatStencil8 => 5,
        }
    }
}

impl Into<wgpu::TextureFormat> for TextureFormat {
    fn into(self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
            TextureFormat::R8Snorm => wgpu::TextureFormat::R8Snorm,
            TextureFormat::R8Uint => wgpu::TextureFormat::R8Uint,
            TextureFormat::R8Sint => wgpu::TextureFormat::R8Sint,
            TextureFormat::R16Uint => wgpu::TextureFormat::R16Uint,
            TextureFormat::R16Sint => wgpu::TextureFormat::R16Sint,
            TextureFormat::R16Float => wgpu::TextureFormat::R16Float,
            TextureFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
            TextureFormat::Rg8Snorm => wgpu::TextureFormat::Rg8Snorm,
            TextureFormat::Rg8Uint => wgpu::TextureFormat::Rg8Uint,
            TextureFormat::Rg8Sint => wgpu::TextureFormat::Rg8Sint,
            TextureFormat::R32Uint => wgpu::TextureFormat::R32Uint,
            TextureFormat::R32Sint => wgpu::TextureFormat::R32Sint,
            TextureFormat::R32Float => wgpu::TextureFormat::R32Float,
            TextureFormat::Rg16Uint => wgpu::TextureFormat::Rg16Uint,
            TextureFormat::Rg16Sint => wgpu::TextureFormat::Rg16Sint,
            TextureFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Rgba8Snorm => wgpu::TextureFormat::Rgba8Snorm,
            TextureFormat::Rgba8Uint => wgpu::TextureFormat::Rgba8Uint,
            TextureFormat::Rgba8Sint => wgpu::TextureFormat::Rgba8Sint,
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            TextureFormat::Rgb9e5Ufloat => wgpu::TextureFormat::Rgb9e5Ufloat,
            TextureFormat::Rgb10a2Uint => wgpu::TextureFormat::Rgb10a2Uint,
            TextureFormat::Rgb10a2Unorm => wgpu::TextureFormat::Rgb10a2Unorm,
            TextureFormat::Rg11b10Ufloat => wgpu::TextureFormat::Rg11b10Ufloat,
            TextureFormat::Rg32Uint => wgpu::TextureFormat::Rg32Uint,
            TextureFormat::Rg32Sint => wgpu::TextureFormat::Rg32Sint,
            TextureFormat::Rg32Float => wgpu::TextureFormat::Rg32Float,
            TextureFormat::Rgba16Uint => wgpu::TextureFormat::Rgba16Uint,
            TextureFormat::Rgba16Sint => wgpu::TextureFormat::Rgba16Sint,
            TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
            TextureFormat::Rgba32Uint => wgpu::TextureFormat::Rgba32Uint,
            TextureFormat::Rgba32Sint => wgpu::TextureFormat::Rgba32Sint,
            TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
            TextureFormat::Stencil8 => wgpu::TextureFormat::Stencil8,
            TextureFormat::Depth16Unorm => wgpu::TextureFormat::Depth16Unorm,
            TextureFormat::Depth24Plus => wgpu::TextureFormat::Depth24Plus,
            TextureFormat::Depth24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
            TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
            TextureFormat::Depth32FloatStencil8 => wgpu::TextureFormat::Depth32FloatStencil8,
        }
    }
}

impl From<wgpu::TextureFormat> for TextureFormat {
    fn from(format: wgpu::TextureFormat) -> Self {
        match format {
            wgpu::TextureFormat::R8Unorm => TextureFormat::R8Unorm,
            wgpu::TextureFormat::R8Snorm => TextureFormat::R8Snorm,
            wgpu::TextureFormat::R8Uint => TextureFormat::R8Uint,
            wgpu::TextureFormat::R8Sint => TextureFormat::R8Sint,
            wgpu::TextureFormat::R16Uint => TextureFormat::R16Uint,
            wgpu::TextureFormat::R16Sint => TextureFormat::R16Sint,
            wgpu::TextureFormat::R16Float => TextureFormat::R16Float,
            wgpu::TextureFormat::Rg8Unorm => TextureFormat::Rg8Unorm,
            wgpu::TextureFormat::Rg8Snorm => TextureFormat::Rg8Snorm,
            wgpu::TextureFormat::Rg8Uint => TextureFormat::Rg8Uint,
            wgpu::TextureFormat::Rg8Sint => TextureFormat::Rg8Sint,
            wgpu::TextureFormat::R32Uint => TextureFormat::R32Uint,
            wgpu::TextureFormat::R32Sint => TextureFormat::R32Sint,
            wgpu::TextureFormat::R32Float => TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Uint => TextureFormat::Rg16Uint,
            wgpu::TextureFormat::Rg16Sint => TextureFormat::Rg16Sint,
            wgpu::TextureFormat::Rg16Float => TextureFormat::Rg16Float,
            wgpu::TextureFormat::Rgba8Unorm => TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb => TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Rgba8Snorm => TextureFormat::Rgba8Snorm,
            wgpu::TextureFormat::Rgba8Uint => TextureFormat::Rgba8Uint,
            wgpu::TextureFormat::Rgba8Sint => TextureFormat::Rgba8Sint,
            wgpu::TextureFormat::Bgra8Unorm => TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb => TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgb9e5Ufloat => TextureFormat::Rgb9e5Ufloat,
            wgpu::TextureFormat::Rgb10a2Uint => TextureFormat::Rgb10a2Uint,
            wgpu::TextureFormat::Rgb10a2Unorm => TextureFormat::Rgb10a2Unorm,
            wgpu::TextureFormat::Rg11b10Ufloat => TextureFormat::Rg11b10Ufloat,
            wgpu::TextureFormat::Rg32Uint => TextureFormat::Rg32Uint,
            wgpu::TextureFormat::Rg32Sint => TextureFormat::Rg32Sint,
            wgpu::TextureFormat::Rg32Float => TextureFormat::Rg32Float,
            wgpu::TextureFormat::Rgba16Uint => TextureFormat::Rgba16Uint,
            wgpu::TextureFormat::Rgba16Sint => TextureFormat::Rgba16Sint,
            wgpu::TextureFormat::Rgba16Float => TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba32Uint => TextureFormat::Rgba32Uint,
            wgpu::TextureFormat::Rgba32Sint => TextureFormat::Rgba32Sint,
            wgpu::TextureFormat::Rgba32Float => TextureFormat::Rgba32Float,
            wgpu::TextureFormat::Stencil8 => TextureFormat::Stencil8,
            wgpu::TextureFormat::Depth16Unorm => TextureFormat::Depth16Unorm,
            wgpu::TextureFormat::Depth24Plus => TextureFormat::Depth24Plus,
            wgpu::TextureFormat::Depth24PlusStencil8 => TextureFormat::Depth24PlusStencil8,
            wgpu::TextureFormat::Depth32Float => TextureFormat::Depth32Float,
            wgpu::TextureFormat::Depth32FloatStencil8 => TextureFormat::Depth32FloatStencil8,
            _ => panic!("Unsupported texture format"),
        }
    }
}
