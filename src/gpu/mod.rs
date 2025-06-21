use crate::{
    utils::{ArcMut, ArcRef},
    window::Handle,
};

mod bind_group_manager;
mod buffer;
mod command;
mod inner;
mod pipeline_manager;
mod shader;
mod texture;
mod pipeline;

pub use buffer::*;
pub use command::*;
pub use shader::*;
pub use texture::*;
pub use pipeline::*;

pub(crate) use bind_group_manager::*;
// pub(crate) use buffer_manager::*;
pub(crate) use inner::*;
pub(crate) use pipeline_manager::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwapchainError {
    NotAvailable,
    ConfigNeeded,
    DeviceLost,
    Suboptimal(wgpu::SurfaceTexture),
}

impl std::fmt::Display for SwapchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapchainError::NotAvailable => write!(f, "Swapchain not available"),
            SwapchainError::ConfigNeeded => write!(f, "Swapchain config needed"),
            SwapchainError::DeviceLost => write!(f, "Device lost"),
            SwapchainError::Suboptimal(_) => write!(f, "Swapchain suboptimal"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GPU {
    pub(crate) inner: ArcRef<inner::GPUInner>,
}

#[derive(Debug, Clone)]
pub struct GPUAdapter {
    pub name: String,
    pub vendor: String,
    pub vendor_id: u32,

    pub backend: String,
    pub backend_enum: AdapterBackend,
    pub is_high_performance: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterBackend {
    None,
    Vulkan,
    Metal,
    Dx12,
    Gl,
    BrowserWebGpu,
}

impl GPU {
    pub(crate) async fn new(
        window: ArcMut<Handle>,
        adapter: Option<&GPUAdapter>,
        limits: Option<Limits>,
    ) -> Result<GPU, String> {
        let inner = ArcRef::new(inner::GPUInner::new(window, adapter, limits).await?);

        Ok(GPU { inner })
    }

    pub(crate) async fn new_headless(
        adapter: Option<&GPUAdapter>,
        limits: Option<Limits>,
    ) -> Result<GPU, String> {
        let inner = ArcRef::new(inner::GPUInner::new_headless(adapter, limits).await?);

        Ok(GPU { inner })
    }

    pub(crate) fn query_gpu(window: Option<ArcMut<Handle>>) -> Vec<GPUAdapter> {
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

                let backend = match info.backend {
                    wgpu::Backend::Vulkan => AdapterBackend::Vulkan,
                    wgpu::Backend::Metal => AdapterBackend::Metal,
                    wgpu::Backend::Dx12 => AdapterBackend::Dx12,
                    wgpu::Backend::Gl => AdapterBackend::Gl,
                    wgpu::Backend::BrowserWebGpu => AdapterBackend::BrowserWebGpu,
                    _ => AdapterBackend::None,
                };

                GPUAdapter {
                    name: info.name,
                    vendor: vendor_name.to_string(),
                    vendor_id: info.vendor,

                    backend: backend_string.to_string(),
                    backend_enum: backend,
                    is_high_performance,
                }
            })
            .collect()
    }

    /// Return the swapchain's format.
    pub fn swapchain_format(&self) -> TextureFormat {
        let inner = self.inner.borrow();
        let format = inner.config.as_ref().unwrap().format;

        format.into()
    }

    /// Set the swapchain vsync.
    pub fn set_vsync(&mut self, vsync: bool) {
        let mut inner = self.inner.borrow_mut();
        inner.set_vsync(vsync);
    }

    pub fn is_vsync(&self) -> bool {
        let inner = self.inner.borrow();
        inner.is_vsync()
    }

    pub fn set_panic_callback<F>(&mut self, _callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        // self.inner.borrow().set_panic_callback(callback);
    }

    /// Begins a new command buffer.
    pub fn begin_command(&mut self) -> Option<command::CommandBuffer> {
        Some(command::CommandBuffer::new(self.inner.clone()))
    }

    /// Begins a new command buffer with a surface texture.
    ///
    /// This is useful if you reuse the surface texture from previous command buffer, but
    /// not yet presented to the screen.
    pub fn begin_command_with_surface(
        &mut self,
        surface: SurfaceTexture,
    ) -> Option<command::CommandBuffer> {
        Some(command::CommandBuffer::new_with_surface(
            self.inner.clone(),
            surface,
        ))
    }

    /// Create a new texture.
    pub fn create_texture(&mut self) -> texture::TextureBuilder {
        texture::TextureBuilder::new(self.inner.clone())
    }

    /// Create a new graphics shader.
    pub fn create_graphics_shader(&mut self) -> shader::GraphicsShaderBuilder {
        shader::GraphicsShaderBuilder::new(self.inner.clone())
    }

    /// Create a new compute shader.
    pub fn create_compute_shader(&mut self) -> shader::ComputeShaderBuilder {
        shader::ComputeShaderBuilder::new(self.inner.clone())
    }

    /// Create a new buffer.
    pub fn create_buffer<T: bytemuck::Pod + bytemuck::Zeroable>(&mut self) -> buffer::BufferBuilder<T> {
        buffer::BufferBuilder::new(self.inner.clone())
    }

    /// Create a render pipeline.
    pub fn create_render_pipeline(&mut self) -> pipeline::RenderPipelineBuilder {
        pipeline::RenderPipelineBuilder::new(self.inner.clone())
    }

    /// Create a compute pipeline.
    pub fn create_compute_pipeline(&mut self) -> pipeline::ComputePipelineBuilder {
        pipeline::ComputePipelineBuilder::new(self.inner.clone())
    }

    /// Wait for the GPU to finish processing commands.
    pub fn wait(&mut self, wait_type: GPUWaitType) {
        let inner = self.inner.borrow();
        let poll_type = match wait_type {
            GPUWaitType::Wait => wgpu::PollType::Wait,
            GPUWaitType::Poll => wgpu::PollType::Poll,
        };

        _ = inner.get_device().poll(poll_type);
    }
}

pub enum GPUWaitType {
    Wait,
    Poll,
}

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_texture_dimension_1d: u32,
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_texture_array_layers: u32,
    pub max_bind_groups: u32,
    pub max_bindings_per_bind_group: u32,
    pub max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    pub max_dynamic_storage_buffers_per_pipeline_layout: u32,
    pub max_sampled_textures_per_shader_stage: u32,
    pub max_samplers_per_shader_stage: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_storage_textures_per_shader_stage: u32,
    pub max_uniform_buffers_per_shader_stage: u32,
    pub max_binding_array_elements_per_shader_stage: u32,
    pub max_binding_array_sampler_elements_per_shader_stage: u32,
    pub max_uniform_buffer_binding_size: u32,
    pub max_storage_buffer_binding_size: u32,
    pub max_vertex_buffers: u32,
    pub max_buffer_size: u64,
    pub max_vertex_attributes: u32,
    pub max_vertex_buffer_array_stride: u32,
    pub min_uniform_buffer_offset_alignment: u32,
    pub min_storage_buffer_offset_alignment: u32,
    pub max_inter_stage_shader_components: u32,
    pub max_color_attachments: u32,
    pub max_color_attachment_bytes_per_sample: u32,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub min_subgroup_size: u32,
    pub max_subgroup_size: u32,
    pub max_push_constant_size: u32,
    pub max_non_sampler_bindings: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_texture_dimension_1d: 8192,
            max_texture_dimension_2d: 8192,
            max_texture_dimension_3d: 2048,
            max_texture_array_layers: 256,
            max_bind_groups: 4,
            max_bindings_per_bind_group: 1000,
            max_dynamic_uniform_buffers_per_pipeline_layout: 8,
            max_dynamic_storage_buffers_per_pipeline_layout: 4,
            max_sampled_textures_per_shader_stage: 16,
            max_samplers_per_shader_stage: 16,
            max_storage_buffers_per_shader_stage: 8,
            max_storage_textures_per_shader_stage: 4,
            max_uniform_buffers_per_shader_stage: 12,
            max_binding_array_elements_per_shader_stage: 0,
            max_binding_array_sampler_elements_per_shader_stage: 0,
            max_uniform_buffer_binding_size: 64 << 10, // (64 KiB)
            max_storage_buffer_binding_size: 128 << 20, // (128 MiB)
            max_vertex_buffers: 8,
            max_buffer_size: 256 << 20, // (256 MiB)
            max_vertex_attributes: 16,
            max_vertex_buffer_array_stride: 2048,
            min_uniform_buffer_offset_alignment: 256,
            min_storage_buffer_offset_alignment: 256,
            max_inter_stage_shader_components: 60,
            max_color_attachments: 8,
            max_color_attachment_bytes_per_sample: 32,
            max_compute_workgroup_storage_size: 16384,
            max_compute_invocations_per_workgroup: 256,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_compute_workgroups_per_dimension: 65535,
            min_subgroup_size: 0,
            max_subgroup_size: 0,
            max_push_constant_size: 0,
            max_non_sampler_bindings: 1_000_000,
        }
    }
}
