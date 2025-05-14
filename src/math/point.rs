use winit::dpi::PhysicalSize;

use super::{Vector2, Vector2I};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const ZERO: Self = Self { x: 0, y: 0 };
    pub const ONE: Self = Self { x: 1, y: 1 };
}

impl Default for Point {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Into<wgpu::Extent3d> for Point {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.x as u32,
            height: self.y as u32,
            depth_or_array_layers: 1,
        }
    }
}

impl From<PhysicalSize<u32>> for Point {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self {
            x: size.width as i32,
            y: size.height as i32,
        }
    }
}

impl From<(i32, i32)> for Point {
    fn from(tuple: (i32, i32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl From<(u32, u32)> for Point {
    fn from(tuple: (u32, u32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
        }
    }
}

impl From<Vector2> for Point {
    fn from(vector: Vector2) -> Self {
        Self {
            x: vector.x.floor() as i32,
            y: vector.y.floor() as i32,
        }
    }
}

impl From<Vector2I> for Point {
    fn from(vector: Vector2I) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
        }
    }
}
