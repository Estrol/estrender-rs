use std::sync::Arc;

use wgpu::{PipelineCache, Surface};
use winit::dpi::PhysicalSize;

use crate::{
    runner::Handle, utils::{ArcMut, ArcRef}, window::Window
};

use pipeline::{
    render::RenderPipelineBuilder,
    compute::ComputePipelineBuilder,
    manager::PipelineManager,
};

use shader::{
    bind_group_manager::{BindGroupManager, BindGroupCreateInfo},
    graphics::GraphicsShaderBuilder,
    compute::ComputeShaderBuilder,
};

use command::{
    CommandBuffer, CommandBufferBuildError,
    SurfaceTexture,
    drawing::DrawingGlobalState
};

use texture::{
    TextureBuilder, TextureFormat,
    atlas::TextureAtlasBuilder
};

use pipeline::manager::{ComputePipelineDesc, GraphicsPipelineDesc};

use buffer::BufferBuilder;

pub mod buffer;
pub mod command;
pub mod pipeline;
pub mod shader;
pub mod texture;

/// Creates a new [GPU] instance.
///
/// This is thread-safe and can be called from any thread, except when using
/// the [GPUBuilder::set_window] method, which binds the GPU to the window's thread.
pub fn new<'a>(window: Option<&'a mut crate::window::Window>) -> GPUBuilder<'a> {
    let builder = GPUBuilder::new();

    if let Some(window) = window {
        builder.set_window(window)
    } else {
        builder
    }
}

/// Queries the available GPU's [GPUAdapter].
///
/// This is useful for checking the available GPU adapters on the system and the supported \
/// graphics APIs, allowing you to choose the best GPU and graphics API for your application.
///
/// This function can be called from any thread.
pub fn query_gpu_adapter(window: Option<&crate::window::Window>) -> Vec<GPUAdapter> {
    let mut window_arc = None;
    if let Some(window) = window {
        window_arc = Some(
            window
                .inner
                .borrow()
                .window_pointer
                .as_ref()
                .unwrap()
                .clone(),
        );
    }

    GPU::query_gpu(window_arc)
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

#[derive(Clone, Debug)]
pub enum GPUWaitType {
    Wait,
    Poll,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SwapchainError {
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
    pub fn begin_command(&mut self) -> Result<CommandBuffer, CommandBufferBuildError> {
        CommandBuffer::new(self.inner.clone())
    }

    /// Begins a new command buffer with a surface texture.
    ///
    /// This is useful if you reuse the surface texture from previous command buffer, but
    /// not yet presented to the screen.
    pub fn begin_command_with_surface(
        &mut self,
        surface: SurfaceTexture,
    ) -> Result<CommandBuffer, CommandBufferBuildError> {
        CommandBuffer::new_with_surface(
            self.inner.clone(),
            surface,
        )
    }

    /// Create a new texture.
    pub fn create_texture(&mut self) -> TextureBuilder {
        TextureBuilder::new(self.inner.clone())
    }

    /// Create a new texture atlas.
    pub fn create_texture_atlas(&mut self) -> TextureAtlasBuilder {
        TextureAtlasBuilder::new(self.inner.clone())
    }

    /// Create a new graphics shader.
    pub fn create_graphics_shader(&mut self) -> GraphicsShaderBuilder {
        GraphicsShaderBuilder::new(self.inner.clone())
    }

    /// Create a new compute shader.
    pub fn create_compute_shader(&mut self) -> ComputeShaderBuilder {
        ComputeShaderBuilder::new(self.inner.clone())
    }

    /// Create a new buffer.
    pub fn create_buffer<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
    ) -> BufferBuilder<T> {
        BufferBuilder::new(self.inner.clone())
    }

    /// Create a render pipeline.
    pub fn create_render_pipeline(&mut self) -> RenderPipelineBuilder {
        RenderPipelineBuilder::new(self.inner.clone())
    }

    /// Create a compute pipeline.
    pub fn create_compute_pipeline(&mut self) -> ComputePipelineBuilder {
        ComputePipelineBuilder::new(self.inner.clone())
    }

    /// Wait for the GPU to finish processing commands.
    pub fn wait(&mut self, wait_type: GPUWaitType) {
        let inner = self.inner.borrow();
        let poll_type = match wait_type {
            GPUWaitType::Wait => wgpu::PollType::Wait,
            GPUWaitType::Poll => wgpu::PollType::Poll,
        };

        _ = inner.device().poll(poll_type);
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

lazy_static::lazy_static! {
    pub(crate) static ref INSTANCE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) struct GPUInner {
    pub is_invalid: bool,
    pub instance_id: usize,

    pub instance: Option<wgpu::Instance>,
    pub window: Option<ArcMut<Handle>>,
    pub surface: Option<Arc<Surface<'static>>>,

    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,
    pub adapter: Option<wgpu::Adapter>,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub pipeline_cache: Option<PipelineCache>,

    pub pipeline_manager: Option<PipelineManager>,
    pub bind_group_manager: Option<BindGroupManager>,

    pub drawing_state: Option<ArcRef<DrawingGlobalState>>,
}

#[allow(unused)]
impl GPUInner {
    pub fn query_gpu(window: Option<ArcMut<Handle>>) -> Vec<wgpu::Adapter> {
        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&instance_descriptor);

        if let Some(window) = window {
            let window = window.lock();

            if window.is_closed() {
                panic!("Window is closed");
            }

            let surface = instance.create_surface(window.get_window());
            let surface = surface.unwrap();

            let adapter = instance.enumerate_adapters(wgpu::Backends::PRIMARY);
            let mut result = Vec::new();

            for adapter in adapter {
                if adapter.is_surface_supported(&surface) {
                    result.push(adapter);
                }
            }

            result
        } else {
            instance.enumerate_adapters(wgpu::Backends::PRIMARY)
        }
    }

    pub async fn new(
        window: ArcMut<Handle>,
        adapter: Option<&GPUAdapter>,
        limits: Option<Limits>,
    ) -> Result<Self, String> {
        let mut window_lock = window.lock();

        if window_lock.is_closed() {
            return Err("Window is closed".to_string());
        }

        if window_lock.is_pinned() {
            return Err("Window is already pinned to existing softbuffer/gpu".to_string());
        }

        let mut instance = Self::new_headless(adapter.clone(), limits).await?;

        let surface = instance
            .instance
            .as_ref()
            .unwrap()
            .create_surface(Arc::clone(window_lock.get_window()));

        if let Err(e) = surface {
            return Err(format!("Failed to create surface: {:?}", e));
        }

        let surface = surface.unwrap();
        let surface_capabilities = surface.get_capabilities(instance.adapter.as_ref().unwrap());
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 0,
            height: 0,
            present_mode: surface_capabilities.present_modes[0],
            view_formats: vec![],
            alpha_mode: surface_capabilities.alpha_modes[0],
            desired_maximum_frame_latency: 2,
        };

        window_lock.set_pinned(true);

        drop(window_lock);

        instance.surface = Some(Arc::new(surface));
        instance.window = Some(window);
        instance.config = Some(config);

        Ok(instance)
    }

    pub async fn new_headless(
        adapter: Option<&GPUAdapter>,
        limits: Option<Limits>,
    ) -> Result<Self, String> {
        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&instance_descriptor);

        let adapter = {
            if adapter.is_none() {
                let adapter_descriptor = wgpu::RequestAdapterOptionsBase {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                };

                let adapter = instance.request_adapter(&adapter_descriptor).await;

                if adapter.is_err() {
                    return Err(format!("Failed to request adapter: {:?}", adapter.err()));
                }

                adapter.unwrap()
            } else {
                let gpu_adapter = adapter.unwrap();

                // query again
                let adapters = instance.enumerate_adapters(wgpu::Backends::PRIMARY);
                let mut found = false;

                let desired_backend = match gpu_adapter.backend_enum {
                    AdapterBackend::Vulkan => wgpu::Backend::Vulkan,
                    AdapterBackend::Metal => wgpu::Backend::Metal,
                    AdapterBackend::Dx12 => wgpu::Backend::Dx12,
                    AdapterBackend::Gl => wgpu::Backend::Gl,
                    AdapterBackend::BrowserWebGpu => wgpu::Backend::BrowserWebGpu,
                    AdapterBackend::None => wgpu::Backend::Noop,
                };

                let mut adapter = None;
                for a in adapters {
                    let backend = a.get_info().backend;
                    if backend == desired_backend
                        && a.get_info().name == gpu_adapter.name
                        && a.get_info().vendor == gpu_adapter.vendor_id
                    {
                        adapter = Some(a);
                        found = true;
                        break;
                    }
                }

                if !found {
                    return Err("Adapter not found".to_string());
                }

                adapter.unwrap()
            }
        };

        let mut device_descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            },
            label: Some("Device"),
            memory_hints: Default::default(),
            ..Default::default()
        };

        if limits.is_some() {
            let limits = limits.unwrap();
            let wgpu_limits = wgpu::Limits {
                max_texture_dimension_1d: limits.max_texture_dimension_1d,
                max_texture_dimension_2d: limits.max_texture_dimension_2d,
                max_texture_dimension_3d: limits.max_texture_dimension_3d,
                max_texture_array_layers: limits.max_texture_array_layers,
                max_bind_groups: limits.max_bind_groups,
                max_bindings_per_bind_group: limits.max_bindings_per_bind_group,
                max_dynamic_uniform_buffers_per_pipeline_layout: limits
                    .max_dynamic_uniform_buffers_per_pipeline_layout,
                max_dynamic_storage_buffers_per_pipeline_layout: limits
                    .max_dynamic_storage_buffers_per_pipeline_layout,
                max_sampled_textures_per_shader_stage: limits.max_sampled_textures_per_shader_stage,
                max_samplers_per_shader_stage: limits.max_samplers_per_shader_stage,
                max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
                max_storage_textures_per_shader_stage: limits.max_storage_textures_per_shader_stage,
                max_uniform_buffers_per_shader_stage: limits.max_uniform_buffers_per_shader_stage,
                max_binding_array_elements_per_shader_stage: limits
                    .max_binding_array_elements_per_shader_stage,
                max_binding_array_sampler_elements_per_shader_stage: limits
                    .max_binding_array_sampler_elements_per_shader_stage,
                max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
                max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
                max_vertex_buffers: limits.max_vertex_buffers,
                max_buffer_size: limits.max_buffer_size,
                max_vertex_attributes: limits.max_vertex_attributes,
                max_vertex_buffer_array_stride: limits.max_vertex_buffer_array_stride,
                min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
                min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment,
                max_inter_stage_shader_components: limits.max_inter_stage_shader_components,
                max_color_attachments: limits.max_color_attachments,
                max_color_attachment_bytes_per_sample: limits.max_color_attachment_bytes_per_sample,
                max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
                max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
                max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
                max_compute_workgroup_size_y: limits.max_compute_workgroup_size_y,
                max_compute_workgroup_size_z: limits.max_compute_workgroup_size_z,
                max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
                min_subgroup_size: limits.min_subgroup_size,
                max_subgroup_size: limits.max_subgroup_size,
                max_push_constant_size: limits.max_push_constant_size,
                max_non_sampler_bindings: limits.max_non_sampler_bindings,
            };

            device_descriptor.required_limits = wgpu_limits;
        }

        let mut optional_features = vec![
            wgpu::Features::DEPTH32FLOAT_STENCIL8,
            wgpu::Features::VERTEX_WRITABLE_STORAGE,
        ];

        #[cfg(not(target_arch = "wasm32"))]
        {
            optional_features.push(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        }

        for feature in optional_features.iter() {
            if adapter.features().contains(*feature) {
                device_descriptor.required_features |= *feature;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if adapter.get_info().backend == wgpu::Backend::Vulkan {
            device_descriptor.required_features |=
                wgpu::Features::PIPELINE_CACHE | wgpu::Features::PUSH_CONSTANTS;
        }

        let req_dev = adapter.request_device(&device_descriptor).await;

        if req_dev.is_err() {
            return Err(format!("Failed to request device: {:?}", req_dev.err()));
        }

        let (device, queue) = req_dev.unwrap();

        let mut pipeline_cache: Option<PipelineCache> = None;

        #[cfg(not(target_arch = "wasm32"))]
        if adapter.get_info().backend == wgpu::Backend::Vulkan {
            let path = std::env::current_exe().unwrap();
            let path = path.parent().unwrap();

            let data = std::fs::read(path.join("cache/pipeline_cache.wgpu")).unwrap_or_default();

            let pipeline_cache_desc = wgpu::PipelineCacheDescriptor {
                label: Some("Pipeline_cache"),
                data: if data.len() > 0 {
                    Some(&data[..])
                } else {
                    None
                },
                fallback: true,
            };

            pipeline_cache = Some(unsafe { device.create_pipeline_cache(&pipeline_cache_desc) });
        }

        let pipeline_manager = PipelineManager::new();
        let bind_group_manager = BindGroupManager::new();

        let id = INSTANCE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(Self {
            is_invalid: false,
            instance_id: id,

            instance: Some(instance),
            window: None,
            surface: None,
            config: None,

            device: Some(device),
            queue: Some(queue),
            adapter: Some(adapter),
            pipeline_cache,
            pipeline_manager: Some(pipeline_manager),
            bind_group_manager: Some(bind_group_manager),
            
            drawing_state: None,
        })
    }

    pub fn is_srgb(&self) -> bool {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        if self.config.is_none() {
            panic!("GPU config not initialized");
        }

        self.config.as_ref().unwrap().format.is_srgb()
    }

    pub fn is_vsync(&self) -> bool {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        if self.config.is_none() {
            panic!("GPU config not initialized");
        }

        self.config.as_ref().unwrap().present_mode == wgpu::PresentMode::Fifo
    }

    pub fn get_swapchain(&self) -> Result<wgpu::SurfaceTexture, SwapchainError> {
        if self.surface.is_none() {
            return Err(SwapchainError::NotAvailable);
        }

        let config = self.config.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();

        if config.width == 0 || config.height == 0 {
            return Err(SwapchainError::ConfigNeeded);
        }

        let swapchain = surface.get_current_texture();
        if swapchain.is_err() {
            return Err(SwapchainError::DeviceLost);
        }

        let swapchain = swapchain.unwrap();

        if swapchain.suboptimal {
            return Err(SwapchainError::Suboptimal(swapchain));
        } else {
            return Ok(swapchain);
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.device.as_ref().unwrap()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.queue.as_ref().unwrap()
    }

    pub fn surface(&self) -> &Surface<'static> {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.surface.as_ref().unwrap()
    }

    pub fn limits(&self) -> wgpu::Limits {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.device.as_ref().unwrap().limits()
    }

    pub fn cycle_manager(&mut self) {
        if self.is_invalid {
            return;
        }

        if let Some(ref mut pipeline_manager) = self.pipeline_manager {
            pipeline_manager.cycle();
        }

        if let Some(ref mut bind_group_manager) = self.bind_group_manager {
            bind_group_manager.cycle();
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if self.is_invalid {
            return;
        }

        if self.window.is_none() || self.surface.is_none() {
            panic!("Graphics not initialized with window");
        }

        if size.width == 0 || size.height == 0 {
            let config = self.config.as_mut().unwrap();
            config.width = 0;
            config.height = 0;
            return;
        }

        let config = self.config.as_mut().unwrap();
        if config.width == size.width && config.height == size.height {
            return;
        }

        config.width = size.width;
        config.height = size.height;

        self.surface
            .as_mut()
            .unwrap()
            .configure(self.device.as_ref().unwrap(), config);
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        if self.is_invalid {
            return;
        }

        if self.window.is_none() || self.surface.is_none() {
            panic!("Graphics not initialized with window");
        }

        let config = self.config.as_mut().unwrap();
        config.present_mode = if vsync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        };

        if config.width == 0 || config.height == 0 {
            return;
        }

        self.surface
            .as_mut()
            .unwrap()
            .configure(self.device.as_ref().unwrap(), config);
    }

    pub fn create_buffer(
        &mut self,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages,
        mapped_at_creation: bool,
    ) -> wgpu::Buffer {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        if size == 0 {
            panic!("Buffer size must be greater than 0");
        }

        let buffer = self.internal_make_buffer(size, usage, mapped_at_creation);

        buffer
    }

    pub fn create_buffer_with<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
        data: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        if data.is_empty() {
            panic!("Data slice cannot be empty");
        }

        let buffer = self.internal_make_buffer(
            (data.len() * std::mem::size_of::<T>()) as wgpu::BufferAddress,
            usage,
            true,
        );

        let mut mapped_range = buffer.slice(..).get_mapped_range_mut();
        let dst = &mut mapped_range[..data.len() * std::mem::size_of::<T>()];
        dst.copy_from_slice(bytemuck::cast_slice(data));

        drop(mapped_range);

        buffer.unmap();

        buffer
    }

    fn internal_make_buffer(
        &mut self,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages,
        mapped_at_creation: bool,
    ) -> wgpu::Buffer {
        if size == 0 {
            panic!("Buffer size must be greater than 0");
        }

        let device = self.device.as_ref().unwrap();

        // This is to honor vulkan's requirement that buffer sizes must be a multiple of COPY_BUFFER_ALIGNMENT.
        let unaligned_size = wgpu::COPY_BUFFER_ALIGNMENT - 1;
        let size = ((size + unaligned_size) & !unaligned_size).max(wgpu::COPY_BUFFER_ALIGNMENT);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(
                format!("Internal Buffer, usage: {}, size: {}", usage.bits(), size).as_str(),
            ),
            size,
            usage,
            mapped_at_creation,
        });

        buffer
    }

    pub fn get_graphics_pipeline(&mut self, key: u64) -> Option<wgpu::RenderPipeline> {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        let pipeline_manager_ref = self.pipeline_manager.as_mut().unwrap();

        pipeline_manager_ref.get_graphics_pipeline(key as usize)
    }

    pub fn create_graphics_pipeline(
        &mut self,
        key: u64,
        desc: GraphicsPipelineDesc,
    ) -> wgpu::RenderPipeline {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        let device_ref = self.device.as_ref().unwrap();
        let pipeline_manager_ref = self.pipeline_manager.as_mut().unwrap();

        pipeline_manager_ref.create_graphics_pipeline(
            key as usize,
            device_ref,
            self.pipeline_cache.as_ref(),
            desc,
        )
    }

    pub fn get_compute_pipeline(&mut self, key: u64) -> Option<wgpu::ComputePipeline> {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        let pipeline_manager_ref = self.pipeline_manager.as_mut().unwrap();

        pipeline_manager_ref.get_compute_pipeline(key as usize)
    }

    pub fn create_compute_pipeline(
        &mut self,
        key: u64,
        desc: ComputePipelineDesc,
    ) -> wgpu::ComputePipeline {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        let device_ref = self.device.as_ref().unwrap();
        let pipeline_manager_ref = self.pipeline_manager.as_mut().unwrap();

        pipeline_manager_ref.create_compute_pipeline(
            key as usize,
            device_ref,
            self.pipeline_cache.as_ref(),
            desc,
        )
    }

    pub fn create_bind_group(
        &mut self,
        key: u64,
        attachment: BindGroupCreateInfo,
    ) -> Vec<(u32, wgpu::BindGroup)> {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        let device_ref = self.device.as_ref().unwrap();
        let bind_group_manager_ref = self.bind_group_manager.as_mut().unwrap();

        bind_group_manager_ref.create(key as usize, device_ref, attachment)
    }

    pub fn get_bind_group(&mut self, key: u64) -> Option<Vec<(u32, wgpu::BindGroup)>> {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        let bind_group_manager_ref = self.bind_group_manager.as_mut().unwrap();

        bind_group_manager_ref.get(key as usize)
    }
}

impl Drop for GPUInner {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(pipeline_cache) = &self.pipeline_cache {
            let data = pipeline_cache.get_data();
            if let Some(data) = data {
                let path = std::env::current_exe().unwrap();
                let path = path.parent().unwrap();

                std::fs::create_dir_all(path.join("cache")).unwrap();
                let pipeline_cache_path = path.join("cache/pipeline_cache.wgpu");

                std::fs::write(&pipeline_cache_path, data).unwrap();

                crate::dbg_log!("Saving pipeline cache to {:?}", pipeline_cache_path);
            }
        }

        crate::dbg_log!("GPU destroyed");
    }
}

impl PartialEq for GPUInner {
    fn eq(&self, other: &Self) -> bool {
        self.device == other.device
            && self.queue == other.queue
            && self.adapter == other.adapter
            && self.config == other.config
            && self.pipeline_cache == other.pipeline_cache
            && self.pipeline_manager == other.pipeline_manager
            && self.bind_group_manager == other.bind_group_manager
    }
}
