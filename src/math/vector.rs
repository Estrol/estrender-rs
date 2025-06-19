use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use bytemuck::{Pod, Zeroable};
use num_traits::ToPrimitive;

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable, Debug)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub fn new<T: ToPrimitive>(x: T, y: T) -> Self {
        Self {
            x: x.to_f32().unwrap_or(0.0),
            y: y.to_f32().unwrap_or(0.0),
        }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let length = self.length();
        Self {
            x: self.x / length,
            y: self.y / length,
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn angle(&self, other: &Self) -> f32 {
        let dot = self.dot(other);
        let len1 = self.length();
        let len2 = other.length();
        (dot / (len1 * len2)).acos()
    }

    pub fn into_vector3(&self) -> Vector3 {
        Vector3 {
            x: self.x,
            y: self.y,
            z: 0.0,
        }
    }

    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const UP: Self = Self { x: 0.0, y: 1.0 };
    pub const DOWN: Self = Self { x: 0.0, y: -1.0 };
    pub const LEFT: Self = Self { x: -1.0, y: 0.0 };
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0 };
}

impl From<[f32; 2]> for Vector2 {
    fn from(array: [f32; 2]) -> Self {
        Self {
            x: array[0],
            y: array[1],
        }
    }
}

impl From<(f32, f32)> for Vector2 {
    fn from(tuple: (f32, f32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl From<Vector3> for Vector2 {
    fn from(vector: Vector3) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
        }
    }
}

impl From<(f32, f32, f32)> for Vector2 {
    fn from(tuple: (f32, f32, f32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl From<(u32, u32)> for Vector2 {
    fn from(tuple: (u32, u32)) -> Self {
        Self {
            x: tuple.0 as f32,
            y: tuple.1 as f32,
        }
    }
}

impl From<(i32, i32)> for Vector2 {
    fn from(tuple: (i32, i32)) -> Self {
        Self {
            x: tuple.0 as f32,
            y: tuple.1 as f32,
        }
    }
}

impl Add for Vector2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Sub<f32> for Vector2 {
    type Output = Self;

    fn sub(self, scalar: f32) -> Self {
        Self {
            x: self.x - scalar,
            y: self.y - scalar,
        }
    }
}

impl Mul<Vector2> for f32 {
    type Output = Vector2;

    fn mul(self, vector: Vector2) -> Vector2 {
        Vector2 {
            x: self * vector.x,
            y: self * vector.y,
        }
    }
}

impl Div<Vector2> for f32 {
    type Output = Vector2;

    fn div(self, vector: Vector2) -> Vector2 {
        Vector2 {
            x: self / vector.x,
            y: self / vector.y,
        }
    }
}

impl Mul<f32> for Vector2 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Div<f32> for Vector2 {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl Div<Vector2> for Vector2 {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl AddAssign for Vector2 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vector2 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for Vector2 {}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new<T: ToPrimitive>(x: T, y: T, z: T) -> Self {
        Self {
            x: x.to_f32().unwrap_or(0.0),
            y: y.to_f32().unwrap_or(0.0),
            z: z.to_f32().unwrap_or(0.0),
        }
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn length(&self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let length = self.length();
        Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
        }
    }

    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    pub const UP: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };

    pub const DOWN: Self = Self {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    };

    pub const LEFT: Self = Self {
        x: -1.0,
        y: 0.0,
        z: 0.0,
    };

    pub const RIGHT: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };

    pub const FORWARD: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
}

impl Default for Vector3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl From<[f32; 3]> for Vector3 {
    fn from(array: [f32; 3]) -> Self {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
        }
    }
}

impl From<(f32, f32, f32)> for Vector3 {
    fn from(tuple: (f32, f32, f32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
        }
    }
}

impl From<(u32, u32, u32)> for Vector3 {
    fn from(tuple: (u32, u32, u32)) -> Self {
        Self {
            x: tuple.0 as f32,
            y: tuple.1 as f32,
            z: tuple.2 as f32,
        }
    }
}

impl From<(i32, i32, i32)> for Vector3 {
    fn from(tuple: (i32, i32, i32)) -> Self {
        Self {
            x: tuple.0 as f32,
            y: tuple.1 as f32,
            z: tuple.2 as f32,
        }
    }
}

impl From<Vector2> for Vector3 {
    fn from(vector: Vector2) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
            z: 0.0,
        }
    }
}

impl From<(f32, f32)> for Vector3 {
    fn from(tuple: (f32, f32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: 0.0,
        }
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl PartialEq for Vector3 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl Eq for Vector3 {}

impl AddAssign for Vector3 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vector3 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4 {
    pub fn new<T: ToPrimitive>(x: T, y: T, z: T, w: T) -> Self {
        Self {
            x: x.to_f32().unwrap_or(0.0),
            y: y.to_f32().unwrap_or(0.0),
            z: z.to_f32().unwrap_or(0.0),
            w: w.to_f32().unwrap_or(1.0), // Default w to 1.0
        }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let length = self.length();
        Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
            w: self.w / length,
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
            w: 0.0, // Cross product in 4D space is not well-defined, set w to 0
        }
    }

    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0, // Default w to 1.0
    };

    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
        w: 1.0, // Default w to 1.0
    };

    pub const UP: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
        w: 1.0, // Default w to 1.0
    };

    pub const DOWN: Self = Self {
        x: 0.0,
        y: -1.0,
        z: 0.0,
        w: 1.0, // Default w to 1.0
    };
}

impl Default for Vector4 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0, // Default w to 1.0
        }
    }
}

impl PartialEq for Vector4 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z && self.w == other.w
    }
}

impl Eq for Vector4 {}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vector2I {
    pub x: i32,
    pub y: i32,
}

#[allow(dead_code)]
impl Vector2I {
    pub fn new<T: ToPrimitive>(x: T, y: T) -> Self {
        Self {
            x: x.to_i32().unwrap_or(0),
            y: y.to_i32().unwrap_or(0),
        }
    }

    pub const ZERO: Self = Self { x: 0, y: 0 };
    pub const ONE: Self = Self { x: 1, y: 1 };
    pub const UP: Self = Self { x: 0, y: 1 };
    pub const DOWN: Self = Self { x: 0, y: -1 };
    pub const LEFT: Self = Self { x: -1, y: 0 };
    pub const RIGHT: Self = Self { x: 1, y: 0 };
    pub const FORWARD: Self = Self { x: 0, y: 1 };
}

#[allow(dead_code)]
impl Vector2I {
    pub fn length(&self) -> f32 {
        ((self.x * self.x + self.y * self.y) as f32).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let length = self.length();
        Self {
            x: (self.x as f32 / length) as i32,
            y: (self.y as f32 / length) as i32,
        }
    }
}

impl From<(i32, i32)> for Vector2I {
    fn from(tuple: (i32, i32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl From<(u32, u32)> for Vector2I {
    fn from(tuple: (u32, u32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
        }
    }
}

impl From<(f32, f32)> for Vector2I {
    fn from(tuple: (f32, f32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vector3I {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[allow(dead_code)]
impl Vector3I {
    pub fn new<T: ToPrimitive>(x: T, y: T, z: T) -> Self {
        Self {
            x: x.to_i32().unwrap_or(0),
            y: y.to_i32().unwrap_or(0),
            z: z.to_i32().unwrap_or(0),
        }
    }

    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };
    pub const ONE: Self = Self { x: 1, y: 1, z: 1 };
    pub const UP: Self = Self { x: 0, y: 1, z: 0 };
    pub const DOWN: Self = Self { x: 0, y: -1, z: 0 };
    pub const LEFT: Self = Self { x: -1, y: 0, z: 0 };
    pub const RIGHT: Self = Self { x: 1, y: 0, z: 0 };
}

#[allow(dead_code)]
impl Vector3I {
    pub fn length(&self) -> f32 {
        ((self.x * self.x + self.y * self.y + self.z * self.z) as f32).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let length = self.length();
        Self {
            x: (self.x as f32 / length) as i32,
            y: (self.y as f32 / length) as i32,
            z: (self.z as f32 / length) as i32,
        }
    }
}

impl From<(i32, i32, i32)> for Vector3I {
    fn from(tuple: (i32, i32, i32)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
        }
    }
}

impl From<(u32, u32, u32)> for Vector3I {
    fn from(tuple: (u32, u32, u32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
            z: tuple.2 as i32,
        }
    }
}

impl From<(f32, f32, f32)> for Vector3I {
    fn from(tuple: (f32, f32, f32)) -> Self {
        Self {
            x: tuple.0 as i32,
            y: tuple.1 as i32,
            z: tuple.2 as i32,
        }
    }
}
