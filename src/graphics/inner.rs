use std::sync::Arc;

use wgpu::{PipelineCache, Surface};
use winit::{dpi::PhysicalSize, window::Window};

use super::{
    GPUAdapter,
    bind_group_manager::BindGroupManager,
    buffer_manager::BufferManager,
    pipeline_manager::{ComputePipelineDesc, GraphicsPipelineDesc, PipelineManager},
    shader::GraphicsShader,
    texture::Texture,
};

#[allow(unused)]
pub struct GPUInner {
    pub is_invalid: bool,

    pub window: Option<Arc<Window>>,
    pub surface: Option<Surface<'static>>,

    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub pipeline_cache: Option<PipelineCache>,

    pub buffer_manager: BufferManager,
    pub pipeline_manager: PipelineManager,
    pub bind_group_manager: BindGroupManager,

    pub drawing_default_texture: Option<Texture>,
    pub drawing_default_shader: Option<GraphicsShader>,
}

impl GPUInner {
    pub fn query_gpu(window: Option<Arc<Window>>) -> Vec<wgpu::Adapter> {
        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&instance_descriptor);

        if let Some(window) = window {
            let surface = instance.create_surface(window);
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

    pub async fn new(window: Arc<Window>, adapter: Option<&GPUAdapter>) -> Result<Self, String> {
        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&instance_descriptor);

        let surface = instance.create_surface(Arc::clone(&window));

        if let Err(e) = surface {
            return Err(format!("Failed to create surface: {:?}", e));
        }

        let surface = surface.unwrap();

        let adapter = {
            if adapter.is_none() {
                let adapter_descriptor = wgpu::RequestAdapterOptionsBase {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                };

                let adapter = instance.request_adapter(&adapter_descriptor).await;

                if adapter.is_none() {
                    return Err("Failed to request adapter".to_string());
                }

                adapter.unwrap()
            } else {
                let gpu_adapter = adapter.unwrap();

                // query again
                let adapters = instance.enumerate_adapters(wgpu::Backends::PRIMARY);
                let mut found = false;

                let mut adapter = None;
                for a in adapters {
                    if a.get_info().name == gpu_adapter.name
                        && a.get_info().vendor == gpu_adapter.vendor_id
                        && a.get_info().backend == gpu_adapter.backend_enum
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
        };

        let optional_features = [wgpu::Features::DEPTH32FLOAT_STENCIL8];

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

        let req_dev = adapter.request_device(&device_descriptor, None).await;

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

        let surface_capabilities = surface.get_capabilities(&adapter);
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

        let buffer_manager = BufferManager::new();
        let pipeline_manager = PipelineManager::new();
        let bind_group_manager = BindGroupManager::new();

        Ok(Self {
            is_invalid: false,

            window: Some(window),
            surface: Some(surface),
            config: Some(config),

            device,
            queue,
            pipeline_cache,
            buffer_manager,
            pipeline_manager,
            bind_group_manager,
            drawing_default_shader: None,
            drawing_default_texture: None,
        })
    }

    pub async fn new_headless(adapter: Option<&GPUAdapter>) -> Result<Self, String> {
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

                if adapter.is_none() {
                    return Err("Failed to request adapter".to_string());
                }

                adapter.unwrap()
            } else {
                let gpu_adapter = adapter.unwrap();

                // query again
                let adapters = instance.enumerate_adapters(wgpu::Backends::PRIMARY);
                let mut found = false;

                let mut adapter = None;
                for a in adapters {
                    if a.get_info().name == gpu_adapter.name
                        && a.get_info().vendor == gpu_adapter.vendor_id
                        && a.get_info().backend == gpu_adapter.backend_enum
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
        };

        let optional_features = [wgpu::Features::DEPTH32FLOAT_STENCIL8];

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

        let req_dev = adapter.request_device(&device_descriptor, None).await;

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

        let buffer_manager = BufferManager::new();
        let pipeline_manager = PipelineManager::new();
        let bind_group_manager = BindGroupManager::new();

        Ok(Self {
            is_invalid: false,

            window: None,
            surface: None,
            config: None,

            device,
            queue,
            pipeline_cache,
            buffer_manager,
            pipeline_manager,
            bind_group_manager,
            drawing_default_shader: None,
            drawing_default_texture: None,
        })
    }

    pub fn cycle_manager(&mut self) {
        self.buffer_manager.cycle();
        self.pipeline_manager.cycle();
        self.bind_group_manager.cycle();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
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
            .configure(&self.device, config);
    }

    pub fn insert_buffer(&mut self, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.buffer_manager.insert(&self.device, data, usage, true)
    }

    pub fn make_buffer(
        &mut self,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        self.buffer_manager.make(&self.device, size, usage, true)
    }

    pub fn insert_graphics_pipeline<'a>(
        &'a mut self,
        desc: GraphicsPipelineDesc<'a>,
    ) -> wgpu::RenderPipeline {
        self.pipeline_manager.insert_graphics_pipeline(
            &self.device,
            self.pipeline_cache.as_ref(),
            desc,
        )
    }

    pub fn insert_compute_pipeline<'a>(
        &'a mut self,
        desc: ComputePipelineDesc<'a>,
    ) -> wgpu::ComputePipeline {
        self.pipeline_manager.insert_compute_pipeline(
            &self.device,
            self.pipeline_cache.as_ref(),
            desc,
        )
    }

    pub fn insert_bind_group(
        &mut self,
        layout: &wgpu::BindGroupLayout,
        attachment: &[wgpu::BindGroupEntry],
    ) -> wgpu::BindGroup {
        self.bind_group_manager
            .insert(&self.device, layout, attachment)
    }

    pub fn destroy(&mut self) {
        if self.is_invalid {
            return;
        }

        self.is_invalid = true;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(pipeline_cache) = &self.pipeline_cache {
            let data = pipeline_cache.get_data();
            if let Some(data) = data {
                let path = std::env::current_exe().unwrap();
                let path = path.parent().unwrap();

                std::fs::create_dir_all(path.join("cache")).unwrap();
                std::fs::write(path.join("cache/pipeline_cache.wgpu"), data).unwrap();
            }
        }

        self.device.poll(wgpu::MaintainBase::Wait);

        self.surface = None;
        self.window = None;
    }
}

impl Drop for GPUInner {
    fn drop(&mut self) {
        self.destroy();
    }
}
