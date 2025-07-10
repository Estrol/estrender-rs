use std::sync::Arc;

use wgpu::{PipelineCache, Surface, SurfaceTexture};
use winit::dpi::PhysicalSize;

use crate::{
    gpu::{
        AdapterBackend, BindGroupCreateInfo, BindGroupManager, ComputePipelineDesc, DrawingGlobalState, GPUAdapter, GraphicsPipelineDesc, Limits, PipelineManager, SwapchainError
    }, runner::runner_inner::Handle, utils::{ArcMut, ArcRef}
};

lazy_static::lazy_static! {
    pub static ref INSTANCE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
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

    pub fn get_swapchain(&self) -> Result<SurfaceTexture, SwapchainError> {
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

    pub fn get_device(&self) -> &wgpu::Device {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.device.as_ref().unwrap()
    }

    pub fn get_queue(&self) -> &wgpu::Queue {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.queue.as_ref().unwrap()
    }

    pub fn get_surface(&self) -> &Surface<'static> {
        if self.is_invalid {
            panic!("Invalid GPU context");
        }

        self.surface.as_ref().unwrap()
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
