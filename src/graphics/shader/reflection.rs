use wgpu::naga::{
    AddressSpace, ArraySize, Module, Scalar, ScalarKind, ShaderStage, TypeInner, VectorSize,
};

use super::{ShaderBindingInfo, ShaderBindingType, ShaderReflect, StorageAccess};

pub(crate) fn parse(module: Module) -> ShaderReflect {
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

                        TypeInner::Image { .. } => {
                            let binding_info = ShaderBindingInfo {
                                binding: binding.binding as u32,
                                group: binding.group as u32,
                                name: var_name,
                                ty: ShaderBindingType::Texture,
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

    for entry_point in module.entry_points.iter() {
        match entry_point.stage {
            ShaderStage::Vertex => vertex_entry_point = entry_point.name.clone(),
            ShaderStage::Fragment => fragment_entry_point = entry_point.name.clone(),
            ShaderStage::Compute => compute_entry_point = entry_point.name.clone(),
        }
    }

    ShaderReflect {
        vertex_entry_point,
        fragment_entry_point,
        compute_entry_point,
        bindings,
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
