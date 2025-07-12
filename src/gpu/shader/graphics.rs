use core::panic;
use std::{borrow::Cow, collections::HashMap, hash::Hash};

use wgpu::{BindingType, SamplerBindingType, ShaderRuntimeChecks, ShaderStages, naga::front::wgsl};

use crate::{
    utils::ArcRef,
};

use super::{
    types::{
        BindGroupLayout, IndexBufferSize, 
        ShaderBindingType, ShaderCullMode, 
        ShaderFrontFace, ShaderPollygonMode, 
        ShaderReflect, ShaderTopology, 
        StorageAccess, VertexInputType,
        VertexInputReflection,
    },
    super::GPUInner,
};

pub(crate) enum GraphicsShaderSource {
    None,
    Source(String),
    SplitSource(String, String),
    BinarySource(Vec<u8>),
    BinarySplitSource(Vec<u8>, Vec<u8>),
}

/// Builder for creating graphics shaders.
///
/// This builder allows you to set the WGSL vertex and fragment shader source code from files or strings.
/// You can also set the vertex and fragment shader source code separately.
pub struct GraphicsShaderBuilder {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) source: GraphicsShaderSource,
}

impl GraphicsShaderBuilder {
    pub(crate) fn new(graphics: ArcRef<GPUInner>) -> Self {
        Self {
            graphics,
            source: GraphicsShaderSource::None,
        }
    }

    /// Sets the WGSL vertex and fragment shader source code from a file.
    pub fn set_file(mut self, path: &str) -> Self {
        let data = std::fs::read_to_string(path);
        if let Err(err) = data {
            panic!("Failed to read shader file: {:?}", err);
        }

        self.source = GraphicsShaderSource::Source(data.unwrap());

        self
    }

    /// Sets the WGSL vertex and fragment shader source code from a string.
    pub fn set_source(mut self, source: &str) -> Self {
        self.source = GraphicsShaderSource::Source(source.to_string());
        self
    }

    /// Sets the WGSL vertex shader source code from a file.
    ///
    /// You need to also set the fragment shader source code using `set_fragment_file` or `set_fragment_code`.
    pub fn set_vertex_file(mut self, path: &str) -> Self {
        let data = std::fs::read_to_string(path);
        if let Err(err) = data {
            panic!("Failed to read vertex shader file: {:?}", err);
        }

        match self.source {
            GraphicsShaderSource::SplitSource(ref mut vertex_source, _) => {
                self.source =
                    GraphicsShaderSource::SplitSource(data.unwrap(), vertex_source.clone());
            }
            _ => {
                self.source = GraphicsShaderSource::SplitSource(data.unwrap(), "".to_string());
            }
        }

        self
    }

    /// Sets the WGSL fragment shader source code from a file.
    ///
    /// You need to also set the vertex shader source code using `set_vertex_file` or `set_vertex_code`.
    pub fn set_fragment_file(mut self, path: &str) -> Self {
        let data = std::fs::read_to_string(path);
        if let Err(err) = data {
            panic!("Failed to read fragment shader file: {:?}", err);
        }

        match self.source {
            GraphicsShaderSource::SplitSource(ref mut vertex_source, _) => {
                self.source =
                    GraphicsShaderSource::SplitSource(vertex_source.clone(), data.unwrap());
            }
            _ => {
                self.source = GraphicsShaderSource::SplitSource("".to_string(), data.unwrap());
            }
        }

        self
    }

    /// Sets the WGSL vertex shader source code from a string.
    ///
    /// You need to also set the fragment shader source code using `set_fragment_code` or `set_fragment_file`.
    pub fn set_vertex_code(mut self, source: &str) -> Self {
        match self.source {
            GraphicsShaderSource::SplitSource(_, ref mut fragment_source) => {
                self.source =
                    GraphicsShaderSource::SplitSource(source.to_string(), fragment_source.clone());
            }
            _ => {
                self.source = GraphicsShaderSource::SplitSource(source.to_string(), "".to_string());
            }
        }

        self
    }

    /// Sets the WGSL fragment shader source code from a string.
    ///
    /// You need to also set the vertex shader source code using `set_vertex_code` or `set_vertex_file`.
    pub fn set_fragment_code(mut self, source: &str) -> Self {
        match self.source {
            GraphicsShaderSource::SplitSource(ref mut vertex_source, _) => {
                self.source =
                    GraphicsShaderSource::SplitSource(vertex_source.clone(), source.to_string());
            }
            _ => {
                self.source = GraphicsShaderSource::SplitSource("".to_string(), source.to_string());
            }
        }

        self
    }

    /// Sets the precompiled binary shader source code.
    ///
    /// This is useful for using shaders compiled with tools like `glslangValidator` or `shaderc`.
    pub fn set_binary_source(mut self, binary: &[u8]) -> Self {
        self.source = GraphicsShaderSource::BinarySource(binary.to_vec());
        self
    }

    /// Sets the precompiled binary vertex and fragment shader source code.
    ///
    /// This is useful for using shaders compiled with tools like `glslangValidator` or `shaderc`.
    pub fn set_binary_file(mut self, path: &str) -> Self {
        let data = std::fs::read(path);
        if let Err(err) = data {
            panic!("Failed to read binary shader file: {:?}", err);
        }

        self.source = GraphicsShaderSource::BinarySource(data.unwrap());
        self
    }

    /// Sets the precompiled binary vertex shader source code.
    ///
    /// You need to also set the fragment shader source code using `set_binary_fragment`.
    pub fn set_binary_vertex(mut self, binary: &[u8]) -> Self {
        match self.source {
            GraphicsShaderSource::BinarySplitSource(ref mut vertex_bin, _) => {
                self.source =
                    GraphicsShaderSource::BinarySplitSource(binary.to_vec(), vertex_bin.clone());
            }
            _ => {
                self.source = GraphicsShaderSource::BinarySplitSource(binary.to_vec(), vec![]);
            }
        }

        self
    }

    /// Sets the precompiled binary fragment shader source code.
    ///
    /// You need to also set the vertex shader source code using `set_binary_vertex`.
    pub fn set_binary_fragment(mut self, binary: &[u8]) -> Self {
        match self.source {
            GraphicsShaderSource::BinarySplitSource(_, ref mut fragment_bin) => {
                self.source =
                    GraphicsShaderSource::BinarySplitSource(fragment_bin.clone(), binary.to_vec());
            }
            _ => {
                self.source = GraphicsShaderSource::BinarySplitSource(vec![], binary.to_vec());
            }
        }

        self
    }

    pub fn build(self) -> Result<GraphicsShader, String> {
        GraphicsShader::new(self.graphics, self.source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsShaderType {
    GraphicsSingle {
        module: wgpu::ShaderModule,
    },
    GraphicsSplit {
        vertex_module: wgpu::ShaderModule,
        fragment_module: wgpu::ShaderModule,
    },
}

impl Hash for GraphicsShaderType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            GraphicsShaderType::GraphicsSingle { module } => {
                module.hash(state);
            }
            GraphicsShaderType::GraphicsSplit {
                vertex_module,
                fragment_module,
            } => {
                vertex_module.hash(state);
                fragment_module.hash(state);
            }
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct GraphicsShaderInner {
    pub ty: GraphicsShaderType,
    pub reflection: Vec<ShaderReflect>,

    pub bind_group_layouts: Vec<BindGroupLayout>,
}

impl PartialEq for GraphicsShaderInner {
    fn eq(&self, other: &Self) -> bool {
        let ty_equal = self.ty == other.ty;

        let reflection_equal = self.reflection.len() == other.reflection.len()
            && self
                .reflection
                .iter()
                .zip(&other.reflection)
                .all(|(a, b)| a == b);

        let layouts_equal = self.bind_group_layouts.len() == other.bind_group_layouts.len()
            && self
                .bind_group_layouts
                .iter()
                .zip(&other.bind_group_layouts)
                .all(|(a, b)| {
                    a.group == b.group && a.bindings == b.bindings && a.layout == b.layout
                });

        ty_equal && reflection_equal && layouts_equal
    }
}

#[derive(Clone, Debug, Eq, Hash)]
pub(crate) struct VertexInputDescription {
    pub index: Option<IndexBufferSize>,
    pub topology: ShaderTopology,
    pub cull_mode: Option<ShaderCullMode>,
    pub polygon_mode: ShaderPollygonMode,
    pub front_face: ShaderFrontFace,
    pub stride: wgpu::BufferAddress,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl PartialEq for VertexInputDescription {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
            && self.topology == other.topology
            && self.cull_mode == other.cull_mode
            && self.polygon_mode == other.polygon_mode
            && self.front_face == other.front_face
            && self.stride == other.stride
            && self.attributes == other.attributes
    }
}

#[derive(Clone, Debug, Eq)]
#[allow(unused)]
pub struct GraphicsShader {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<GraphicsShaderInner>,

    pub(crate) attrib: ArcRef<VertexInputDescription>,
}

impl GraphicsShader {
    pub(crate) fn new(
        graphics: ArcRef<GPUInner>,
        wgls_data: GraphicsShaderSource,
    ) -> Result<Self, String> {
        let graphics_ref = graphics.borrow();
        let device_ref = graphics_ref.device.as_ref().ok_or("Missing device")?;

        fn create_vertex_input_attrib(input: &VertexInputReflection) -> Vec<wgpu::VertexAttribute> {
            input
                .attributes
                .iter()
                .map(|(location, offset, vtype)| wgpu::VertexAttribute {
                    format: vtype.clone().into(),
                    offset: *offset as wgpu::BufferAddress,
                    shader_location: *location,
                })
                .collect()
        }

        fn create_input_desc(reflection: &ShaderReflect) -> Result<VertexInputDescription, String> {
            let (vertex_input, stride) = match reflection {
                ShaderReflect::Vertex { input, .. }
                | ShaderReflect::VertexFragment {
                    vertex_input: input,
                    ..
                } => {
                    let input = input.as_ref().ok_or("Missing vertex input")?;
                    (input, input.stride as wgpu::BufferAddress)
                }
                _ => return Err("Invalid shader type for vertex input".to_string()),
            };

            let attributes = create_vertex_input_attrib(vertex_input);
            Ok(VertexInputDescription {
                index: Some(IndexBufferSize::U16),
                stride,
                attributes,
                topology: ShaderTopology::TriangleList,
                cull_mode: None,
                polygon_mode: ShaderPollygonMode::Fill,
                front_face: ShaderFrontFace::Clockwise,
            })
        }

        fn build_single_shader(
            device: &wgpu::Device,
            source: &str,
        ) -> Result<(wgpu::ShaderModule, ShaderReflect), String> {
            let module = wgsl::parse_str(source).map_err(|e| format!("Parse error: {e:?}"))?;
            let reflection = super::reflection::parse(module).map_err(|e| format!("Reflect error: {e:?}"))?;
            Ok((
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                }),
                reflection,
            ))
        }

        fn build_binary_shader(
            device: &wgpu::Device,
            binary: &[u8],
        ) -> Result<(wgpu::ShaderModule, ShaderReflect), String> {
            let binary_shader = super::reflection::load_binary_shader(binary)
                .map_err(|e| format!("Binary load error: {e:?}"))?;
            let spirv_u32 = Cow::Borrowed(bytemuck::cast_slice(&binary_shader.spirv));
            Ok((
                // SAFETY: All binary shaders are validated and built with our shader compiler (est-shader-compiler).
                // This used for fast shader loading, so we assume that the binary shader is valid.
                unsafe {
                    let desc = wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::SpirV(spirv_u32),
                    };

                    let runtime_checks = ShaderRuntimeChecks {
                        bounds_checks: true,
                        force_loop_bounding: false,
                    };

                    device.create_shader_module_trusted(desc, runtime_checks)
                },
                binary_shader.reflect,
            ))
        }

        match wgls_data {
            GraphicsShaderSource::None => Err("No shader source provided".to_string()),

            GraphicsShaderSource::Source(source) => {
                let (module, reflection) = build_single_shader(device_ref, &source)?;
                match reflection {
                    ShaderReflect::VertexFragment { .. } => {
                        let layout = Self::make_group_layout(device_ref, &[reflection.clone()]);
                        let input_desc = create_input_desc(&reflection)?;
                        Ok(Self {
                            graphics: ArcRef::clone(&graphics),
                            inner: ArcRef::new(GraphicsShaderInner {
                                ty: GraphicsShaderType::GraphicsSingle { module },
                                reflection: vec![reflection],
                                bind_group_layouts: layout,
                            }),
                            attrib: ArcRef::new(input_desc),
                        })
                    }
                    _ => Err("Shader source is not VertexFragment shader!".to_string()),
                }
            }

            GraphicsShaderSource::SplitSource(vertex_src, fragment_src) => {
                let (vertex_module, vertex_reflect) = build_single_shader(device_ref, &vertex_src)?;
                let (fragment_module, fragment_reflect) =
                    build_single_shader(device_ref, &fragment_src)?;

                match (&vertex_reflect, &fragment_reflect) {
                    (ShaderReflect::Vertex { .. }, ShaderReflect::Fragment { .. }) => {
                        let layout = Self::make_group_layout(
                            device_ref,
                            &[vertex_reflect.clone(), fragment_reflect.clone()],
                        );
                        let input_desc = create_input_desc(&vertex_reflect)?;
                        Ok(Self {
                            graphics: ArcRef::clone(&graphics),
                            inner: ArcRef::new(GraphicsShaderInner {
                                ty: GraphicsShaderType::GraphicsSplit {
                                    vertex_module,
                                    fragment_module,
                                },
                                reflection: vec![vertex_reflect, fragment_reflect],
                                bind_group_layouts: layout,
                            }),
                            attrib: ArcRef::new(input_desc),
                        })
                    }
                    _ => Err("Invalid shader pair for SplitSource".to_string()),
                }
            }

            GraphicsShaderSource::BinarySource(binary) => {
                let (module, reflection) = build_binary_shader(device_ref, &binary)?;
                match reflection {
                    ShaderReflect::VertexFragment { .. } => {
                        let layout = Self::make_group_layout(device_ref, &[reflection.clone()]);
                        let input_desc = create_input_desc(&reflection)?;
                        Ok(Self {
                            graphics: ArcRef::clone(&graphics),
                            inner: ArcRef::new(GraphicsShaderInner {
                                ty: GraphicsShaderType::GraphicsSingle { module },
                                reflection: vec![reflection],
                                bind_group_layouts: layout,
                            }),
                            attrib: ArcRef::new(input_desc),
                        })
                    }
                    _ => Err("Binary shader is not VertexFragment shader!".to_string()),
                }
            }

            GraphicsShaderSource::BinarySplitSource(vertex_bin, fragment_bin) => {
                let (vertex_module, vertex_reflect) = build_binary_shader(device_ref, &vertex_bin)?;
                let (fragment_module, fragment_reflect) =
                    build_binary_shader(device_ref, &fragment_bin)?;

                match (&vertex_reflect, &fragment_reflect) {
                    (ShaderReflect::Vertex { .. }, ShaderReflect::Fragment { .. }) => {
                        let layout = Self::make_group_layout(
                            device_ref,
                            &[vertex_reflect.clone(), fragment_reflect.clone()],
                        );
                        let input_desc = create_input_desc(&vertex_reflect)?;
                        Ok(Self {
                            graphics: ArcRef::clone(&graphics),
                            inner: ArcRef::new(GraphicsShaderInner {
                                ty: GraphicsShaderType::GraphicsSplit {
                                    vertex_module,
                                    fragment_module,
                                },
                                reflection: vec![vertex_reflect, fragment_reflect],
                                bind_group_layouts: layout,
                            }),
                            attrib: ArcRef::new(input_desc),
                        })
                    }
                    _ => Err("Invalid binary shader pair for BinarySplitSource".to_string()),
                }
            }
        }
    }

    fn make_group_layout(
        device: &wgpu::Device,
        reflects: &[ShaderReflect],
    ) -> Vec<BindGroupLayout> {
        let mut layouts: HashMap<u32, Vec<wgpu::BindGroupLayoutEntry>> = HashMap::new();

        fn find_existing(
            layouts: &mut HashMap<u32, Vec<wgpu::BindGroupLayoutEntry>>,
            group: u32,
            binding: u32,
            _ty: wgpu::BindingType,
        ) -> Option<&mut wgpu::BindGroupLayoutEntry> {
            layouts.get_mut(&group).and_then(|entries| {
                entries
                    .iter_mut()
                    .find(|entry| entry.binding == binding && matches!(entry.ty, _ty))
            })
        }

        fn create_layout_ty(ty: ShaderBindingType) -> wgpu::BindingType {
            match ty {
                ShaderBindingType::UniformBuffer(size) => BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: if size == u32::MAX {
                        None
                    } else {
                        wgpu::BufferSize::new(size as u64)
                    },
                },
                ShaderBindingType::Texture(multisampled) => BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled,
                },
                ShaderBindingType::Sampler(comparison) => BindingType::Sampler(if comparison {
                    SamplerBindingType::Comparison
                } else {
                    SamplerBindingType::Filtering
                }),
                ShaderBindingType::StorageBuffer(size, access) => BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: access.contains(StorageAccess::READ)
                            && !access.contains(StorageAccess::WRITE),
                    },
                    has_dynamic_offset: false,
                    min_binding_size: if size == u32::MAX {
                        None
                    } else {
                        wgpu::BufferSize::new(size as u64)
                    },
                },
                ShaderBindingType::StorageTexture(access) => BindingType::StorageTexture {
                    access: if access.contains(StorageAccess::READ)
                        && access.contains(StorageAccess::WRITE)
                    {
                        wgpu::StorageTextureAccess::ReadWrite
                    } else if access.contains(StorageAccess::READ) {
                        wgpu::StorageTextureAccess::ReadOnly
                    } else if access.contains(StorageAccess::WRITE) {
                        wgpu::StorageTextureAccess::WriteOnly
                    } else if access.contains(StorageAccess::ATOMIC) {
                        wgpu::StorageTextureAccess::Atomic
                    } else {
                        panic!("Invalid storage texture access")
                    },
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                _ => unreachable!(),
            }
        }

        for reflect in reflects {
            match reflect {
                ShaderReflect::Vertex { bindings, .. } => {
                    for binding in bindings.iter() {
                        let ty = create_layout_ty(binding.ty.clone());
                        let existing =
                            find_existing(&mut layouts, binding.group, binding.binding, ty);
                        if let Some(existing) = existing {
                            existing.visibility |= ShaderStages::VERTEX;
                            crate::dbg_log!(
                                "BindGroupLayout: group {}, binding: {}, ty: {:?} (existing)",
                                binding.group,
                                binding.binding,
                                binding.ty
                            );
                        } else {
                            // Push new layout entry
                            let layout_desc = wgpu::BindGroupLayoutEntry {
                                ty,
                                binding: binding.binding,
                                visibility: ShaderStages::VERTEX,
                                count: None,
                            };

                            let group = layouts.entry(binding.group).or_insert_with(Vec::new);

                            crate::dbg_log!(
                                "BindGroupLayout: group {}, binding: {}, ty: {:?}",
                                binding.group,
                                binding.binding,
                                binding.ty
                            );
                            group.push(layout_desc);
                        }
                    }
                }
                ShaderReflect::Fragment { bindings, .. } => {
                    for binding in bindings.iter() {
                        let ty = create_layout_ty(binding.ty.clone());
                        let existing =
                            find_existing(&mut layouts, binding.group, binding.binding, ty);
                        if let Some(existing) = existing {
                            existing.visibility |= ShaderStages::FRAGMENT;
                            crate::dbg_log!(
                                "BindGroupLayout: group {}, binding: {}, ty: {:?} (existing)",
                                binding.group,
                                binding.binding,
                                binding.ty
                            );
                        } else {
                            // Push new layout entry
                            let layout_desc = wgpu::BindGroupLayoutEntry {
                                ty,
                                binding: binding.binding,
                                visibility: ShaderStages::FRAGMENT,
                                count: None,
                            };

                            let group = layouts.entry(binding.group).or_insert_with(Vec::new);

                            crate::dbg_log!(
                                "BindGroupLayout: group {}, binding: {}, ty: {:?}",
                                binding.group,
                                binding.binding,
                                binding.ty
                            );
                            group.push(layout_desc);
                        }
                    }
                }
                ShaderReflect::VertexFragment { bindings, .. } => {
                    for binding in bindings.iter() {
                        let ty = create_layout_ty(binding.ty.clone());

                        // Push new layout entry
                        let layout_desc = wgpu::BindGroupLayoutEntry {
                            ty,
                            binding: binding.binding,
                            visibility: ShaderStages::VERTEX_FRAGMENT,
                            count: None,
                        };

                        let group = layouts.entry(binding.group).or_insert_with(Vec::new);

                        crate::dbg_log!(
                            "BindGroupLayout: group {}, binding: {}, ty: {:?}",
                            binding.group,
                            binding.binding,
                            binding.ty
                        );
                        group.push(layout_desc);
                    }
                }
                _ => continue,
            }
        }

        let mut layout_vec = layouts.into_iter().collect::<Vec<_>>();
        layout_vec.sort_by_key(|(group, _)| *group);
        layout_vec
            .into_iter()
            .map(|(group, layout)| {
                // Label: "BindGroupLayout for group {group}, binding: {binding} (ex: 0, 1, 2)"
                let label = if !layout.is_empty() {
                    let mut s = format!("BindGroupLayout for group {}, binding: ", group);
                    for (i, entry) in layout.iter().enumerate() {
                        s.push_str(&entry.binding.to_string());
                        if i != layout.len() - 1 {
                            s.push_str(", ");
                        }
                    }
                    Some(s)
                } else {
                    None
                };

                let bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: label.as_deref(),
                        entries: &layout,
                    });

                crate::dbg_log!(
                    "Created BindGroupLayout for group {} with {} entries",
                    group,
                    layout.len()
                );

                BindGroupLayout {
                    group,
                    bindings: layout.iter().map(|entry| entry.binding).collect(),
                    layout: bind_group_layout,
                }
            })
            .collect()
    }

    pub fn get_uniform_location(&self, name: &str) -> Option<(u32, u32)> {
        let inner = self.inner.borrow();

        let reflection = &inner.reflection;
        for reflect in reflection.iter() {
            match reflect {
                ShaderReflect::Vertex { bindings, .. } => {
                    if let Some(binding) = bindings.iter().find(|b| {
                        b.name == name && matches!(b.ty, ShaderBindingType::UniformBuffer(_))
                    }) {
                        return Some((binding.group, binding.binding));
                    }
                }
                ShaderReflect::Fragment { bindings, .. } => {
                    if let Some(binding) = bindings.iter().find(|b| {
                        b.name == name && matches!(b.ty, ShaderBindingType::UniformBuffer(_))
                    }) {
                        return Some((binding.group, binding.binding));
                    }
                }
                ShaderReflect::VertexFragment { bindings, .. } => {
                    if let Some(binding) = bindings.iter().find(|b| {
                        b.name == name && matches!(b.ty, ShaderBindingType::UniformBuffer(_))
                    }) {
                        return Some((binding.group, binding.binding));
                    }
                }
                _ => continue,
            }
        }

        None
    }

    pub fn get_uniform_size(&self, group: u32, binding: u32) -> Option<u32> {
        let inner = self.inner.borrow();

        let reflection = &inner.reflection;
        for reflect in reflection.iter() {
            match reflect {
                ShaderReflect::Vertex { bindings, .. } => {
                    if let Some(binding) = bindings
                        .iter()
                        .find(|b| b.group == group && b.binding == binding)
                    {
                        if let ShaderBindingType::UniformBuffer(size) = binding.ty {
                            return Some(size);
                        }
                    }
                }
                ShaderReflect::Fragment { bindings, .. } => {
                    if let Some(binding) = bindings
                        .iter()
                        .find(|b| b.group == group && b.binding == binding)
                    {
                        if let ShaderBindingType::UniformBuffer(size) = binding.ty {
                            return Some(size);
                        }
                    }
                }
                ShaderReflect::VertexFragment { bindings, .. } => {
                    if let Some(binding) = bindings
                        .iter()
                        .find(|b| b.group == group && b.binding == binding)
                    {
                        if let ShaderBindingType::UniformBuffer(size) = binding.ty {
                            return Some(size);
                        }
                    }
                }
                _ => continue,
            }
        }

        None
    }

    pub fn set_topology(&mut self, topology: ShaderTopology) -> Result<(), String> {
        self.attrib.borrow_mut().topology = topology;
        Ok(())
    }

    pub fn set_cull_mode(&mut self, cull_mode: Option<ShaderCullMode>) -> Result<(), String> {
        self.attrib.borrow_mut().cull_mode = cull_mode;
        Ok(())
    }

    pub fn set_polygon_mode(&mut self, polygon_mode: ShaderPollygonMode) -> Result<(), String> {
        self.attrib.borrow_mut().polygon_mode = polygon_mode;
        Ok(())
    }

    pub fn set_front_face(&mut self, front_face: ShaderFrontFace) -> Result<(), String> {
        self.attrib.borrow_mut().front_face = front_face;
        Ok(())
    }

    pub fn set_vertex_index_ty(&mut self, index_ty: Option<IndexBufferSize>) -> Result<(), String> {
        self.attrib.borrow_mut().index = index_ty;
        Ok(())
    }

    pub fn set_vertex_input(
        &mut self,
        location: u32,
        vtype: VertexInputType,
    ) -> Result<(), String> {
        let inner = self.inner.borrow_mut();

        let vertex_input = match inner.reflection.first() {
            Some(ShaderReflect::Vertex { input, .. }) => input.as_ref(),
            Some(ShaderReflect::VertexFragment { vertex_input, .. }) => vertex_input.as_ref(),
            _ => None,
        };

        if vertex_input.is_none() {
            return Err("Shader does not have vertex input".to_string());
        }

        let vertex_input = vertex_input.unwrap();

        let input = vertex_input
            .attributes
            .iter()
            .find(|attr| attr.0 == location);
        if input.is_none() {
            return Err(format!("Vertex input location {} not found", location));
        }

        let (location, _offset, og_vtype) = input.unwrap();
        if !is_format_conversion_supported(*og_vtype, vtype) {
            return Err(format!(
                "Vertex input type {:?} is not supported for location {}",
                vtype, location
            ));
        }

        let mut attrib = self.attrib.borrow_mut();
        let vertex_input_attrib = attrib
            .attributes
            .iter_mut()
            .find(|attr| attr.shader_location == *location);

        if vertex_input_attrib.is_none() {
            return Err(format!(
                "Vertex input location {} not found in shader attributes",
                location
            ));
        }

        let vertex_input_attrib = vertex_input_attrib.unwrap();
        vertex_input_attrib.format = vtype.into();

        Ok(())
    }
}

impl std::hash::Hash for GraphicsShader {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ArcRef::as_ptr(&self.graphics).hash(state);
        self.inner.hash(state);
        self.attrib.hash(state);
    }
}

// origin is O.G value based on reflection, and target is user-input type
// example: if origin is Float32 and target is Unorm32, then it is supported
#[inline]
fn is_format_conversion_supported(origin: VertexInputType, target: VertexInputType) -> bool {
    match origin {
        VertexInputType::Float32 => match target {
            VertexInputType::Float32 => true,
            VertexInputType::Snorm8 => true,
            VertexInputType::Unorm8 => true,
            VertexInputType::Snorm16 => true,
            _ => false,
        },
        VertexInputType::Float32x2 => match target {
            VertexInputType::Float32x2 => true,
            VertexInputType::Snorm8x2 => true,
            VertexInputType::Unorm8x2 => true,
            VertexInputType::Snorm16x2 => true,
            _ => false,
        },
        VertexInputType::Float32x3 => {
            match target {
                VertexInputType::Float32x3 => true,
                // normalized types are not supported for 3-component vectors
                _ => false,
            }
        }
        VertexInputType::Float32x4 => match target {
            VertexInputType::Float32x4 => true,
            VertexInputType::Snorm8x4 => true,
            VertexInputType::Unorm8x4 => true,
            VertexInputType::Snorm16x4 => true,
            _ => false,
        },
        VertexInputType::Uint32 => match target {
            VertexInputType::Uint32 => true,
            VertexInputType::Uint16 => true,
            VertexInputType::Uint8 => true,
            _ => false,
        },
        VertexInputType::Uint32x2 => match target {
            VertexInputType::Uint32x2 => true,
            VertexInputType::Uint16x2 => true,
            VertexInputType::Uint8x2 => true,
            _ => false,
        },
        VertexInputType::Uint32x3 => match target {
            VertexInputType::Uint32x3 => true,
            VertexInputType::Uint16x4 => true,
            VertexInputType::Uint8x4 => true,
            _ => false,
        },
        VertexInputType::Uint32x4 => match target {
            VertexInputType::Uint32x4 => true,
            VertexInputType::Uint16x4 => true,
            VertexInputType::Uint8x4 => true,
            _ => false,
        },
        _ => origin == target,
    }
}

// impl PartialEq for VertexInputDescription {
//     fn eq(&self, other: &Self) -> bool {
//         self.index == other.index
//             && self.topology == other.topology
//             && self.cull_mode == other.cull_mode
//             && self.polygon_mode == other.polygon_mode
//             && self.front_face == other.front_face
//             && self.stride == other.stride
//             && self.attributes == other.attributes
//     }
// }

// impl PartialEq for GraphicsShaderInner {
//     fn eq(&self, other: &Self) -> bool {
//         // self.ty == other.ty && self.reflection == other.reflection
//         self.ty == other.ty
//     }
// }

impl PartialEq for GraphicsShader {
    fn eq(&self, other: &Self) -> bool {
        ArcRef::ptr_eq(&self.graphics, &other.graphics)
            && ArcRef::ptr_eq(&self.inner, &other.inner)
            && ArcRef::ptr_eq(&self.attrib, &other.attrib)
    }
}
