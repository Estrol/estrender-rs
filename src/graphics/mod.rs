use std::sync::Arc;

use winit::window::Window;

use crate::utils::ArcRef;

mod bind_group_manager;
pub mod buffer;
mod buffer_manager;
pub mod command;
pub(crate) mod inner;
mod pipeline_manager;
pub mod sampler;
pub mod shader;
pub mod texture;

pub struct GPU {
    pub(crate) inner: ArcRef<inner::GPUInner>,
}

#[derive(Clone)]
pub struct GPUAdapter {
    pub name: String,
    pub vendor: String,
    pub vendor_id: u32,

    pub backend: String,
    pub backend_enum: wgpu::Backend,
    pub is_high_performance: bool,
}

impl GPU {
    pub async fn new(window: Arc<Window>, adapter: Option<&GPUAdapter>) -> Result<GPU, String> {
        let inner = ArcRef::new(inner::GPUInner::new(window, adapter).await?);

        Ok(GPU { inner })
    }

    pub async fn new_headless(adapter: Option<&GPUAdapter>) -> Result<GPU, String> {
        let inner = ArcRef::new(inner::GPUInner::new_headless(adapter).await?);

        Ok(GPU { inner })
    }

    pub fn query_gpu(window: Option<Arc<Window>>) -> Vec<GPUAdapter> {
        let adapter = inner::GPUInner::query_gpu(window);

        adapter
            .into_iter()
            .map(|adapter| {
                let info = adapter.get_info();

                let vendor_name = match info.vendor {
                    0x1002 => "AMD",
                    0x10DE => "NVIDIA",
                    0x8086 => "Intel",
                    0x13B5 => "ARM",
                    _ => "Unknown",
                };

                let backend_string = match info.backend {
                    wgpu::Backend::Vulkan => "Vulkan",
                    wgpu::Backend::Metal => "Metal",
                    wgpu::Backend::Dx12 => "DirectX 12",
                    wgpu::Backend::Gl => "OpenGL",
                    wgpu::Backend::BrowserWebGpu => "WebGPU",
                    _ => "Unknown",
                };

                let is_high_performance = matches!(info.device_type, wgpu::DeviceType::DiscreteGpu);

                GPUAdapter {
                    name: info.name,
                    vendor: vendor_name.to_string(),
                    vendor_id: info.vendor,

                    backend: backend_string.to_string(),
                    backend_enum: info.backend,
                    is_high_performance,
                }
            })
            .collect()
    }

    pub fn run_command(&self, callback: impl FnOnce(&mut command::CommandBuffer)) {
        let mut begin = self.begin_command().unwrap();

        callback(&mut begin);
    }

    pub fn begin_command(&self) -> Option<command::CommandBuffer> {
        Some(command::CommandBuffer::new(self.inner.clone()))
    }

    pub fn texture_builder(&self) -> texture::TextureBuilder {
        texture::TextureBuilder::new(self.inner.clone())
    }

    pub fn graphics_shader_builder(&self) -> shader::GraphicsShaderBuilder {
        shader::GraphicsShaderBuilder::new(self.inner.clone())
    }

    pub fn compute_shader_builder(&self) -> shader::ComputeShaderBuilder {
        shader::ComputeShaderBuilder::new(self.inner.clone())
    }

    pub fn buffer_builder<T: bytemuck::Pod>(&self) -> buffer::BufferBuilder<T> {
        buffer::BufferBuilder::new(self.inner.clone())
    }
}

impl Drop for GPU {
    fn drop(&mut self) {
        let inner = self.inner.borrow();
        inner.device.poll(wgpu::Maintain::Wait);

        dbg!("GPU dropped");
    }
}
