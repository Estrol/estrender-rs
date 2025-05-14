use bytemuck::{Pod, Zeroable};

use crate::graphics::shader::{VertexInputAttribute, VertexInputDesc, VertexInputType};

use super::{Color, Vector2, Vector3};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vector3,
    pub color: Color,
    pub texcoord: Vector2,
}

#[allow(dead_code)]
impl Vertex {
    pub fn new(position: Vector3, color: Color, texcoord: Vector2) -> Self {
        Self {
            position,
            color,
            texcoord,
        }
    }

    pub fn new_slice(position: [f32; 3], color: [f32; 4], texcoord: [f32; 2]) -> Self {
        Self {
            position: Vector3::new(position[0], position[1], position[2]),
            color: Color::new(color[0], color[1], color[2], color[3]),
            texcoord: Vector2::new(texcoord[0], texcoord[1]),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn desc() -> VertexInputDesc<'static> {
        VertexInputDesc {
            stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            attributes: &[
                VertexInputAttribute {
                    shader_location: 0,
                    offset: 0,
                    format: VertexInputType::Float32x3,
                },
                VertexInputAttribute {
                    shader_location: 1,
                    offset: 12,
                    format: VertexInputType::Float32x4,
                },
                VertexInputAttribute {
                    shader_location: 2,
                    offset: 28,
                    format: VertexInputType::Float32x2,
                },
            ],
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        // SAFETY: We are using `std::mem::size_of::<Self>()` to get the size of the struct,
        // and we are not using any uninitialized memory.
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.color == other.color
            && self.texcoord == other.texcoord
    }
}

impl Eq for Vertex {}

impl From<((f32, f32, f32), (f32, f32, f32, f32), (f32, f32))> for Vertex {
    fn from(data: ((f32, f32, f32), (f32, f32, f32, f32), (f32, f32))) -> Self {
        Self {
            position: Vector3::new(data.0.0, data.0.1, data.0.2),
            color: Color::new(data.1.0, data.1.1, data.1.2, data.1.3),
            texcoord: Vector2::new(data.2.0, data.2.1),
        }
    }
}

impl From<(Vector3, Color, Vector2)> for Vertex {
    fn from(data: (Vector3, Color, Vector2)) -> Self {
        Self {
            position: data.0,
            color: data.1,
            texcoord: data.2,
        }
    }
}

impl From<((f32, f32), (f32, f32, f32, f32), (f32, f32))> for Vertex {
    fn from(data: ((f32, f32), (f32, f32, f32, f32), (f32, f32))) -> Self {
        Self {
            position: Vector3::new(data.0.0, data.0.1, 0.0),
            color: Color::new(data.1.0, data.1.1, data.1.2, data.1.3),
            texcoord: Vector2::new(data.2.0, data.2.1),
        }
    }
}

impl From<(Vector2, Color, Vector2)> for Vertex {
    fn from(data: (Vector2, Color, Vector2)) -> Self {
        Self {
            position: data.0.into(),
            color: data.1,
            texcoord: data.2,
        }
    }
}
