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