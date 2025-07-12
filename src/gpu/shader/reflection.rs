use core::str;
use std::io::{Cursor, Read};

use byteorder_lite::{LittleEndian, ReadBytesExt};
use wgpu::naga::{
    AddressSpace, ArraySize, Binding, Module, Scalar, ScalarKind, ShaderStage, TypeInner,
    VectorSize,
};

use super::types::{
    ShaderBindingInfo, ShaderBindingType, ShaderReflect, StorageAccess, VertexInputReflection,
    VertexInputType,
};

pub fn is_shader_valid(data: &str) -> bool {
    match wgpu::naga::front::wgsl::parse_str(data) {
        Ok(module) => {
            let res = parse(module);
            res.is_ok()
        }
        Err(err) => {
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            eprintln!("Shader validation error: {:?}", err);
            false
        }
    }
}

pub struct BinaryShader {
    pub spirv: Vec<u8>,
    pub reflect: ShaderReflect,
}

const BINARY_SHADER_MAGIC: [u8; 20] = *b"est-binary-shader-v1";

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
    cursor
        .read_u32::<LittleEndian>()
        .map_err(|_| "Failed to read u32".to_string())
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, String> {
    cursor
        .read_u64::<LittleEndian>()
        .map_err(|_| "Failed to read u64".to_string())
}

fn read_bytes(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0; len];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| "Failed to read bytes".to_string())?;
    Ok(buf)
}

fn read_utf8_string(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<String, String> {
    let bytes = read_bytes(cursor, len)?;
    String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 string".to_string())
}

pub fn load_binary_shader(data: &[u8]) -> Result<BinaryShader, String> {
    let mut cursor = Cursor::new(data);

    let mut magic = [0; 20];
    cursor
        .read_exact(&mut magic)
        .map_err(|_| "Failed to read magic".to_string())?;
    if magic != BINARY_SHADER_MAGIC {
        return Err("Invalid shader magic".to_string());
    }

    let shader_type_id = read_u32(&mut cursor)?;

    let entry_point_sz = read_u32(&mut cursor)?;
    let entry_point = read_utf8_string(&mut cursor, entry_point_sz as usize)?;

    let binding_count = read_u32(&mut cursor)?;
    let mut bindings = Vec::with_capacity(binding_count as usize);

    for _ in 0..binding_count {
        let group = read_u32(&mut cursor)?;
        let binding = read_u32(&mut cursor)?;
        let name_sz = read_u32(&mut cursor)?;
        let name = read_utf8_string(&mut cursor, name_sz as usize)?;
        let ty = match read_u32(&mut cursor)? {
            0 => ShaderBindingType::UniformBuffer(read_u32(&mut cursor)?),
            1 => {
                let size = read_u32(&mut cursor)?;
                let access = StorageAccess::from_bits(read_u32(&mut cursor)?)
                    .ok_or("Invalid storage access")?;
                ShaderBindingType::StorageBuffer(size, access)
            }
            2 => {
                let access = StorageAccess::from_bits(read_u32(&mut cursor)?)
                    .ok_or("Invalid storage texture access")?;
                ShaderBindingType::StorageTexture(access)
            }
            3 => ShaderBindingType::Sampler(read_u32(&mut cursor)? != 0),
            4 => ShaderBindingType::Texture(read_u32(&mut cursor)? != 0),
            5 => ShaderBindingType::PushConstant(read_u32(&mut cursor)?),
            t => return Err(format!("Unknown binding type ID: {}", t)),
        };

        bindings.push(ShaderBindingInfo {
            binding,
            group,
            name,
            ty,
        });
    }

    let vertex_input = if shader_type_id == 0 || shader_type_id == 2 {
        let name_sz = read_u32(&mut cursor)?;
        let name = read_utf8_string(&mut cursor, name_sz as usize)?;
        let stride = read_u32(&mut cursor)? as u64;
        let attr_count = read_u32(&mut cursor)?;
        let mut attributes = Vec::with_capacity(attr_count as usize);

        for _ in 0..attr_count {
            let location = read_u32(&mut cursor)?;
            let offset = read_u64(&mut cursor)?;
            let ty_id = read_u32(&mut cursor)?;
            let ty = match ty_id {
                0 => VertexInputType::Float32,
                1 => VertexInputType::Float32x2,
                2 => VertexInputType::Float32x3,
                3 => VertexInputType::Float32x4,
                4 => VertexInputType::Sint32,
                5 => VertexInputType::Sint32x2,
                6 => VertexInputType::Sint32x3,
                7 => VertexInputType::Sint32x4,
                8 => VertexInputType::Uint32,
                9 => VertexInputType::Uint32x2,
                10 => VertexInputType::Uint32x3,
                11 => VertexInputType::Uint32x4,
                _ => return Err(format!("Invalid vertex input type: {}", ty_id)),
            };
            attributes.push((location, offset, ty));
        }

        Some(VertexInputReflection {
            name,
            stride,
            attributes,
        })
    } else {
        None
    };

    let reflect = match shader_type_id {
        0 => ShaderReflect::Vertex {
            entry_point,
            input: vertex_input,
            bindings,
        },
        1 => ShaderReflect::Fragment {
            entry_point,
            bindings,
        },
        2 => {
            let parts: Vec<&str> = entry_point.split(',').collect();
            if parts.len() != 2 {
                return Err("Invalid vertex/fragment entry point format".to_string());
            }
            ShaderReflect::VertexFragment {
                vertex_entry_point: parts[0].to_string(),
                vertex_input,
                fragment_entry_point: parts[1].to_string(),
                bindings,
            }
        }
        3 => ShaderReflect::Compute {
            entry_point,
            bindings,
        },
        t => return Err(format!("Unknown shader type ID: {}", t)),
    };

    let spirv_sz = read_u32(&mut cursor)?;
    let spirv = read_bytes(&mut cursor, spirv_sz as usize)?;

    Ok(BinaryShader { spirv, reflect })
}

pub(crate) fn parse(module: Module) -> Result<ShaderReflect, String> {
    let mut bindings = Vec::new();
    for (handle, var) in module.global_variables.iter() {
        if let Some(binding) = &var.binding {
            match var.space {
                AddressSpace::Uniform => {
                    let ty = &module.types[var.ty];
                    let size = get_size(&module, &ty.inner);
                    let var_name = var
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("unnamed_{:?}", handle));

                    if size <= 16 {
                        // Uniforms smaller than 16 bytes are not supported
                        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                        return Err(format!(
                            "Uniform variable '{}' is too small ({} bytes), must be at least 16 bytes",
                            var_name, size
                        ));
                    }

                    let binding_info = ShaderBindingInfo {
                        binding: binding.binding as u32,
                        group: binding.group as u32,
                        name: var_name,
                        ty: ShaderBindingType::UniformBuffer(size as u32),
                    };

                    bindings.push(binding_info);
                }

                AddressSpace::PushConstant => {
                    let ty = &module.types[var.ty];
                    let size = get_size(&module, &ty.inner);
                    let var_name = var
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("unnamed_{:?}", handle));

                    let binding_info = ShaderBindingInfo {
                        binding: binding.binding as u32,
                        group: binding.group as u32,
                        name: var_name,
                        ty: ShaderBindingType::PushConstant(size as u32),
                    };

                    bindings.push(binding_info);
                }

                AddressSpace::Storage { access: _access } => {
                    let ty = &module.types[var.ty];
                    let var_name = var
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("unnamed_{:?}", handle));

                    let mut access = StorageAccess::empty();
                    if _access.contains(wgpu::naga::StorageAccess::LOAD) {
                        access |= StorageAccess::READ
                    }

                    if _access.contains(wgpu::naga::StorageAccess::STORE) {
                        access |= StorageAccess::WRITE;
                    }

                    if _access.contains(wgpu::naga::StorageAccess::ATOMIC) {
                        access |= StorageAccess::ATOMIC;
                    }

                    match &ty.inner {
                        TypeInner::Struct {
                            members: _,
                            span: _,
                        } => {
                            let size = get_size(&module, &ty.inner);

                            let binding_info = ShaderBindingInfo {
                                binding: binding.binding as u32,
                                group: binding.group as u32,
                                name: var_name,
                                ty: ShaderBindingType::StorageBuffer(size as u32, access),
                            };

                            bindings.push(binding_info);
                        }

                        TypeInner::Image {
                            dim: _,
                            arrayed: _,
                            class: _,
                        } => {
                            let binding_info = ShaderBindingInfo {
                                binding: binding.binding as u32,
                                group: binding.group as u32,
                                name: var_name,
                                ty: ShaderBindingType::StorageTexture(access),
                            };

                            bindings.push(binding_info);
                        }

                        TypeInner::Array {
                            base: _,
                            size,
                            stride: _,
                        } => {
                            let count = match size {
                                ArraySize::Constant(size) => size.get(),
                                _ => u32::MAX, // Default with unlimited sizes
                            };

                            let binding_info = ShaderBindingInfo {
                                binding: binding.binding as u32,
                                group: binding.group as u32,
                                name: var_name,
                                ty: ShaderBindingType::StorageBuffer(count, access),
                            };

                            bindings.push(binding_info);
                        }

                        _ => {}
                    }
                }

                AddressSpace::Handle => {
                    // Check if sampler, sampled texture, or storage texture

                    let ty = &module.types[var.ty];
                    let var_name = var
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("unnamed_{:?}", handle));

                    match ty.inner {
                        TypeInner::Sampler { comparison } => {
                            let binding_info = ShaderBindingInfo {
                                binding: binding.binding as u32,
                                group: binding.group as u32,
                                name: var_name,
                                ty: ShaderBindingType::Sampler(comparison),
                            };

                            bindings.push(binding_info);
                        }

                        TypeInner::Image {
                            dim: _,
                            arrayed: _,
                            class,
                        } => {
                            let binding_info = ShaderBindingInfo {
                                binding: binding.binding as u32,
                                group: binding.group as u32,
                                name: var_name,
                                ty: ShaderBindingType::Texture(match class {
                                    wgpu::naga::ImageClass::Sampled { kind: _, multi } => multi,
                                    wgpu::naga::ImageClass::Depth { multi } => multi,
                                    wgpu::naga::ImageClass::Storage {
                                        format: _,
                                        access: _,
                                    } => {
                                        // panic!("Storage image should be handled separately")
                                        return Err("Storage image should be handled separately"
                                            .to_string());
                                    }
                                }),
                            };

                            bindings.push(binding_info);
                        }

                        _ => {}
                    }
                }

                _ => {}
            }
        }
    }

    // sort the bindings by group first, then by binding
    // A: 0, 0
    // B: 0, 1
    // C: 1, 0
    // D: 1, 1
    bindings.sort_by(|a, b| {
        if a.group == b.group {
            a.binding.cmp(&b.binding)
        } else {
            a.group.cmp(&b.group)
        }
    });

    // get entry point
    let mut vertex_entry_point = String::new();
    let mut fragment_entry_point = String::new();
    let mut compute_entry_point = String::new();

    let mut vertex_struct_input = None;

    #[allow(unused)]
    for entry_point in module.entry_points.iter() {
        match entry_point.stage {
            ShaderStage::Vertex => {
                vertex_entry_point = entry_point.name.clone();

                /**
                 * Example:
                 *
                 * struct VertexInput {
                 *   @location(0) position: vec3<f32>,
                 *   @location(1) color: vec4<f32>,
                 *   @location(2) texCoord: vec2<f32>,
                 * };
                 */
                for vertex_input in entry_point.function.arguments.iter() {
                    let ty = &module.types[vertex_input.ty];

                    let struct_name = ty
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("unnamed_{:?}", vertex_input.ty));

                    let mut attributes = Vec::new();
                    let mut total_size = 0;

                    match &ty.inner {
                        TypeInner::Struct { members, span } => {
                            for member in members.iter() {
                                let attribute_name = member
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| format!("unnamed_{:?}", member.ty));

                                let ty = &module.types[member.ty];
                                let size = get_size(&module, &ty.inner);
                                let location = member
                                    .binding
                                    .as_ref()
                                    .and_then(|b| match b {
                                        Binding::Location {
                                            location,
                                            interpolation: _,
                                            sampling: _,
                                            blend_src: _,
                                        } => Some(*location as u32),
                                        _ => None,
                                    })
                                    .unwrap_or_else(|| {
                                        panic!("Vertex input must have a location binding")
                                    });

                                match &ty.inner {
                                    TypeInner::Scalar(scalar) => {
                                        if let Some(vertex_input_type) =
                                            mapping_to_vertex_input(scalar, None)
                                        {
                                            attributes.push((
                                                location,
                                                total_size as u64,
                                                vertex_input_type,
                                            ));

                                            total_size += scalar_size(scalar);
                                        } else {
                                            // #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                                            // panic!(
                                            //     "Unsupported vertex input type: {:?} for member: {}",
                                            //     ty.inner, attribute_name
                                            // );
                                            return Err(format!(
                                                "Unsupported vertex input type: {:?} for member: {}",
                                                ty.inner, attribute_name
                                            ));
                                        }
                                    }

                                    TypeInner::Vector { size, scalar } => {
                                        if let Some(vertex_input_type) =
                                            mapping_to_vertex_input(scalar, Some(size))
                                        {
                                            attributes.push((
                                                location,
                                                total_size as u64,
                                                vertex_input_type,
                                            ));

                                            total_size +=
                                                vectorsize_as_u32(size) * scalar_size(scalar);
                                        } else {
                                            // #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                                            // panic!(
                                            //     "Unsupported vertex vector input type: {:?} for member: {}",
                                            //     ty.inner, attribute_name
                                            // );
                                            return Err(format!(
                                                "Unsupported vertex vector input type: {:?} for member: {}",
                                                ty.inner, attribute_name
                                            ));
                                        }
                                    }

                                    _ => {
                                        // #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                                        // panic!(
                                        //     "Unsupported vertex input type: {:?} for member: {}",
                                        //     ty.inner, attribute_name
                                        // );
                                        return Err(format!(
                                            "Unsupported vertex input type: {:?} for member: {}",
                                            ty.inner, attribute_name
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    vertex_struct_input = Some(VertexInputReflection {
                        name: struct_name,
                        stride: total_size as u64,
                        attributes,
                    });
                }
            }
            ShaderStage::Fragment => fragment_entry_point = entry_point.name.clone(),
            ShaderStage::Compute => compute_entry_point = entry_point.name.clone(),
            _ => {
                // #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
                // panic!("Unsupported shader stage: {:?}", entry_point.stage);
                return Err(format!("Unsupported shader stage: {:?}", entry_point.stage));
            }
        }
    }

    if !vertex_entry_point.is_empty() && !fragment_entry_point.is_empty() {
        return Ok(ShaderReflect::VertexFragment {
            vertex_entry_point,
            vertex_input: vertex_struct_input,
            fragment_entry_point,
            bindings,
        });
    }

    if !vertex_entry_point.is_empty() {
        return Ok(ShaderReflect::Vertex {
            entry_point: vertex_entry_point,
            input: vertex_struct_input,
            bindings,
        });
    }

    if !fragment_entry_point.is_empty() {
        return Ok(ShaderReflect::Fragment {
            entry_point: fragment_entry_point,
            bindings,
        });
    }

    if !compute_entry_point.is_empty() {
        return Ok(ShaderReflect::Compute {
            entry_point: compute_entry_point,
            bindings,
        });
    }

    Err("No valid entry point found in shader module".to_string())
}

pub(crate) fn mapping_to_vertex_input(
    scalar: &Scalar,
    vector: Option<&VectorSize>,
) -> Option<VertexInputType> {
    match scalar.kind {
        ScalarKind::Float => {
            if let Some(vector_size) = vector {
                match vector_size {
                    VectorSize::Bi => Some(VertexInputType::Float32x2),
                    VectorSize::Tri => Some(VertexInputType::Float32x3),
                    VectorSize::Quad => Some(VertexInputType::Float32x4),
                }
            } else {
                Some(VertexInputType::Float32)
            }
        }
        ScalarKind::Sint => {
            if let Some(vector_size) = vector {
                match vector_size {
                    VectorSize::Bi => Some(VertexInputType::Sint32x2),
                    VectorSize::Tri => Some(VertexInputType::Sint32x3),
                    VectorSize::Quad => Some(VertexInputType::Sint32x4),
                }
            } else {
                Some(VertexInputType::Sint32)
            }
        }
        ScalarKind::Uint => {
            if let Some(vector_size) = vector {
                match vector_size {
                    VectorSize::Bi => Some(VertexInputType::Uint32x2),
                    VectorSize::Tri => Some(VertexInputType::Uint32x3),
                    VectorSize::Quad => Some(VertexInputType::Uint32x4),
                }
            } else {
                Some(VertexInputType::Uint32)
            }
        }
        ScalarKind::Bool => {
            if let Some(vector_size) = vector {
                match vector_size {
                    VectorSize::Bi => Some(VertexInputType::Uint32),
                    VectorSize::Tri => Some(VertexInputType::Uint32x3),
                    VectorSize::Quad => Some(VertexInputType::Uint32x4),
                }
            } else {
                Some(VertexInputType::Uint32)
            }
        }
        _ => None,
    }
}

#[allow(unused_variables)]
pub(crate) fn get_size(module: &Module, ty_inner: &TypeInner) -> i32 {
    match ty_inner {
        TypeInner::Scalar(scalar) => scalar_size(scalar) as i32,

        TypeInner::Vector { size, scalar } => {
            let scalar_size = scalar_size(scalar);
            let vec_size = vectorsize_as_u32(size) * scalar_size;
            align_to(vec_size, vector_alignment(size)) as i32 // Ensure correct alignment
        }

        TypeInner::Matrix {
            columns,
            rows,
            scalar,
        } => {
            let scalar_size = scalar_size(scalar);
            let row_size = vectorsize_as_u32(rows) * scalar_size;
            let aligned_row_size = align_to(row_size, 16); // Matrices align to 16 bytes per row
            (vectorsize_as_u32(columns) * aligned_row_size) as i32
        }

        TypeInner::Array { base, size, stride } => {
            let count = match size {
                ArraySize::Constant(size) => size.get(),
                _ => u32::MAX, // Default with unlimited sizes
            };

            if count == u32::MAX {
                -1 // Indicate dynamic array
            } else {
                (count * stride) as i32
            }
        }

        TypeInner::Struct { members, span } => {
            let mut max_alignment = 0;
            let mut size = 0;
            for member in members {
                let ty = &module.types[member.ty];

                let member_size = get_size(module, &ty.inner);
                let alignment = std140_alignment(module, &ty.inner);
                size = align_to(size, alignment) + member_size as u32;
                max_alignment = max_alignment.max(alignment);
            }

            align_to(size, max_alignment) as i32 // Ensure struct is padded to its largest member
        }

        _ => 0, // Other types like images, samplers, and pointers are not sized
    }
}

pub(crate) fn scalar_size(scalar: &Scalar) -> u32 {
    match scalar.kind {
        ScalarKind::Float => 4,
        ScalarKind::Sint => 4,
        ScalarKind::Uint => 4,
        ScalarKind::Bool => 4,
        _ => 0,
    }
}

pub(crate) fn vectorsize_as_u32(size: &VectorSize) -> u32 {
    match size {
        VectorSize::Bi => 2,
        VectorSize::Tri => 3,
        VectorSize::Quad => 4,
    }
}

pub(crate) fn std140_alignment(module: &Module, ty_inner: &TypeInner) -> u32 {
    match ty_inner {
        TypeInner::Scalar(_) => 4,
        TypeInner::Vector { size, .. } => vector_alignment(size),
        TypeInner::Matrix { .. } => 16,
        TypeInner::Struct { members, .. } => members
            .iter()
            .map(|m| {
                let r#type = &module.types[m.ty];
                std140_alignment(module, &r#type.inner)
            })
            .max()
            .unwrap_or(1),
        _ => 1,
    }
}

pub(crate) fn vector_alignment(size: &VectorSize) -> u32 {
    match size {
        VectorSize::Bi => 8,    // vec2 = 8-byte aligned
        VectorSize::Tri => 16,  // vec3 = 16-byte aligned
        VectorSize::Quad => 16, // vec4 = 16-byte aligned
    }
}

pub(crate) fn align_to(size: u32, alignment: u32) -> u32 {
    (size + alignment - 1) & !(alignment - 1)
}
