use crate::{
    gpu::{
        AdapterBackend, GPUWaitType, SurfaceTexture, TextureFormat, buffer, command,
        gpu_inner::GPUInner, pipeline, shader, texture,
    },
    runner::runner_inner::Handle,
    utils::{ArcMut, ArcRef},
    window::Window,
};

#[derive(Debug, Clone)]
pub struct GPUAdapter {
    pub name: String,
    pub vendor: String,
    pub vendor_id: u32,

    pub backend: String,
    pub backend_enum: AdapterBackend,
    pub is_high_performance: bool,
}

#[derive(Debug, Clone)]
pub struct GPU {
    pub(crate) inner: ArcRef<GPUInner>,
}

impl GPU {
    pub(crate) async fn new(
        window: ArcMut<Handle>,
        adapter: Option<&GPUAdapter>,
        limits: Option<Limits>,
    ) -> Result<GPU, String> {
        let inner = ArcRef::new(GPUInner::new(window, adapter, limits).await?);

        Ok(GPU { inner })
    }

    pub(crate) async fn new_headless(
        adapter: Option<&GPUAdapter>,
        limits: Option<Limits>,
    ) -> Result<GPU, String> {
        let inner = ArcRef::new(GPUInner::new_headless(adapter, limits).await?);

        Ok(GPU { inner })
    }

    pub(crate) fn query_gpu(window: Option<ArcMut<Handle>>) -> Vec<GPUAdapter> {
        let adapter = GPUInner::query_gpu(window);

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

    /// Returns the vsync setting of the swapchain.
    pub fn is_vsync(&self) -> bool {
        let inner = self.inner.borrow();
        inner.is_vsync()
    }

    /// Check if the swapchain is using sRGB format.
    ///
    /// This is useful for determining if you want to use sRGB textures or not.
    pub fn is_surface_srgb(&self) -> bool {
        let inner = self.inner.borrow();
        inner.is_srgb()
    }

    pub fn set_panic_callback<F>(&mut self, _callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        // self.inner.borrow().set_panic_callback(callback);
    }

    /// Begins a new command buffer.
    pub fn begin_command(&mut self) -> Result<command::CommandBuffer, command::CommandBufferBuildError> {
        command::CommandBuffer::new(self.inner.clone())
    }

    /// Begins a new command buffer with a surface texture.
    ///
    /// This is useful if you reuse the surface texture from previous command buffer, but
    /// not yet presented to the screen.
    pub fn begin_command_with_surface(
        &mut self,
        surface: SurfaceTexture,
    ) -> Result<command::CommandBuffer, command::CommandBufferBuildError> {
        command::CommandBuffer::new_with_surface(
            self.inner.clone(),
            surface,
        )
    }

    /// Create a new texture.
    pub fn create_texture(&mut self) -> texture::TextureBuilder {
        texture::TextureBuilder::new(self.inner.clone())
    }

    /// Create a new texture atlas.
    pub fn create_texture_atlas(&mut self) -> texture::TextureAtlasBuilder {
        texture::TextureAtlasBuilder::new(self.inner.clone())
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
    pub fn create_buffer<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
    ) -> buffer::BufferBuilder<T> {
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

pub struct GPUBuilder<'a> {
    window: Option<&'a mut Window>,
    adapter: Option<&'a GPUAdapter>,
    limits: Option<Limits>,
}

impl<'a> GPUBuilder<'a> {
    pub(crate) fn new() -> Self {
        GPUBuilder {
            window: None,
            adapter: None,
            limits: None,
        }
    }

    /// Sets the window for this GPU instance.
    ///
    /// This is useful for creating a GPU instance that is bound to a specific window.
    /// The window must be created before this GPU instance.
    pub fn set_window(mut self, window: &'a mut Window) -> Self {
        self.window = Some(window);
        self
    }

    /// Sets the GPU adapter for this GPU instance.
    ///
    /// This is useful for creating a GPU instance that uses a specific GPU adapter.
    /// The adapter can be queried using the `Engine::query_gpu_adapter` function.
    pub fn set_adapter(mut self, adapter: &'a GPUAdapter) -> Self {
        self.adapter = Some(adapter);
        self
    }

    pub fn set_limits(mut self, limits: Limits) -> Self {
        self.limits = Some(limits);
        self
    }

    pub fn build(self) -> Result<GPU, String> {
        let gpu;

        if self.window.is_some() {
            let window_ref = self.window.unwrap();
            let mut window_inner = window_ref.inner.borrow_mut();

            #[cfg(feature = "software")]
            if window_inner.pixelbuffer.is_some() {
                return Err(
                    "GPU cannot be created along side PixelBuffer (software rendering)".to_string(),
                );
            }

            let window_cloned = window_inner.window_pointer.as_ref().unwrap().clone();

            gpu = futures::executor::block_on(GPU::new(window_cloned, self.adapter, self.limits))?;

            window_inner.graphics = Some(gpu.inner.clone());
        } else {
            gpu = futures::executor::block_on(GPU::new_headless(self.adapter, self.limits))?;
        }

        Ok(gpu)
    }
}
