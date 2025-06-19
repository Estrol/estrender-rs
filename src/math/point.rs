use num_traits::ToPrimitive;
use winit::dpi::PhysicalSize;

use super::{Vector2, Vector2I};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point2 {
    pub x: i32,
    pub y: i32,
}

impl Point2 {
    pub fn new<T: ToPrimitive>(x: T, y: T) -> Self {
        Self {
            x: x.to_i32().unwrap_or(0),
            y: y.to_i32().unwrap_or(0),
        }
    }

    pub const ZERO: Self = Self { x: 0, y: 0 };
    pub const ONE: Self = Self { x: 1, y: 1 };
}

impl Default for Point2 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Into<wgpu::Extent3d> for Point2 {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.x as u32,
            height: self.y as u32,
            depth_or_array_layers: 1,
        }
    }
}

impl From<PhysicalSize<u32>> for Point2 {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self {
            x: size.width as i32,
            y: size.height as i32,
        }
    }
}

impl From<(i32, i32)> for Point2 {
    fn from(tuple: (i32, i32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl From<(u32, u32)> for Point2 {
    fn from(tuple: (u32, u32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
        }
    }
}

impl From<Vector2> for Point2 {
    fn from(vector: Vector2) -> Self {
        Self {
            x: vector.x.floor() as i32,
            y: vector.y.floor() as i32,
        }
    }
}

impl From<Vector2I> for Point2 {
    fn from(vector: Vector2I) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Point3 {
    pub fn new<T: ToPrimitive>(x: T, y: T, z: T) -> Self {
        Self {
            x: x.to_i32().unwrap_or(0),
            y: y.to_i32().unwrap_or(0),
            z: z.to_i32().unwrap_or(0),
        }
    }

    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };
    pub const ONE: Self = Self { x: 1, y: 1, z: 1 };
}

impl Default for Point3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<(i32, i32, i32)> for Point3 {
    fn from(tuple: (i32, i32, i32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
        }
    }
}

impl From<(u32, u32, u32)> for Point3 {
    fn from(tuple: (u32, u32, u32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
            z: tuple.2 as i32,
        }
    }
}

impl From<Vector2> for Point3 {
    fn from(vector: Vector2) -> Self {
        Self {
            x: vector.x.floor() as i32,
            y: vector.y.floor() as i32,
            z: 0, // Default z to 0
        }
    }
}

impl From<Vector2I> for Point3 {
    fn from(vector: Vector2I) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
            z: 0, // Default z to 0
        }
    }
}

impl Into<wgpu::Extent3d> for Point3 {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.x as u32,
            height: self.y as u32,
            depth_or_array_layers: self.z as u32,
        }
    }
}

impl From<Point2> for Point3 {
    fn from(point: Point2) -> Self {
        Self {
            x: point.x,
            y: point.y,
            z: 0, // Default z to 0
        }
    }
}

impl From<Point3> for Point2 {
    fn from(point: Point3) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<Point2> for Vector2 {
    fn from(point: Point2) -> Self {
        Self {
            x: point.x as f32,
            y: point.y as f32,
        }
    }
}

impl From<Point3> for Vector2 {
    fn from(point: Point3) -> Self {
        Self {
            x: point.x as f32,
            y: point.y as f32,
        }
    }
}

impl From<Point2> for Vector2I {
    fn from(point: Point2) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<Point3> for Vector2I {
    fn from(point: Point3) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}
