use bytemuck::{Pod, Zeroable};

use super::{Color, Vector2, Vector3};

/// To use this vertex struct in your shader, you need to use this WGSL code as your vertex type:
/// ```wgsl
/// struct VertexInput {
///     @location(0) position: vec3<f32>,
///     @location(1) color: vec4<f32>,
///     @location(2) texCoord: vec2<f32>,
/// };
/// ```
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
            color: Color::new_const(color[0], color[1], color[2], color[3]),
            texcoord: Vector2::new(texcoord[0], texcoord[1]),
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
            color: Color::new_const(data.1.0, data.1.1, data.1.2, data.1.3),
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
            color: Color::new_const(data.1.0, data.1.1, data.1.2, data.1.3),
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

impl From<[f32; 8]> for Vertex {
    fn from(data: [f32; 8]) -> Self {
        Self {
            position: Vector3::new(data[0], data[1], data[2]),
            color: Color::new_const(data[3], data[4], data[5], data[6]),
            texcoord: Vector2::new(data[7], 0.0),
        }
    }
}

impl From<[f32; 6]> for Vertex {
    fn from(data: [f32; 6]) -> Self {
        Self {
            position: Vector3::new(data[0], data[1], 0.0),
            color: Color::new_const(data[2], data[3], data[4], 1.0),
            texcoord: Vector2::new(data[5], 0.0),
        }
    }
}
