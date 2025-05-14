#[derive(Debug, Clone, Copy)]
pub struct Size {
    width: i32,
    height: i32,
}

impl Size {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    pub fn one() -> Self {
        Self {
            width: 1,
            height: 1,
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::zero()
    }
}

impl Into<wgpu::Extent3d> for Size {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width as u32,
            height: self.height as u32,
            depth_or_array_layers: 1,
        }
    }
}

use std::ops::*;

use winit::dpi::PhysicalSize;

impl Add for Size {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}

impl AddAssign for Size {
    fn add_assign(&mut self, rhs: Self) {
        self.width += rhs.width;
        self.height += rhs.height;
    }
}

impl Sub for Size {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl SubAssign for Size {
    fn sub_assign(&mut self, rhs: Self) {
        self.width -= rhs.width;
        self.height -= rhs.height;
    }
}

impl Mul<i32> for Size {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self {
        Self {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl MulAssign<i32> for Size {
    fn mul_assign(&mut self, rhs: i32) {
        self.width *= rhs;
        self.height *= rhs;
    }
}

impl Div<i32> for Size {
    type Output = Self;

    fn div(self, rhs: i32) -> Self {
        Self {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

impl DivAssign<i32> for Size {
    fn div_assign(&mut self, rhs: i32) {
        self.width /= rhs;
        self.height /= rhs;
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            width: -self.width,
            height: -self.height,
        }
    }
}

impl PartialEq for Size {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }
}

impl Eq for Size {}

impl Into<PhysicalSize<u32>> for Size {
    fn into(self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.width as u32, self.height as u32)
    }
}

impl Into<PhysicalSize<i32>> for Size {
    fn into(self) -> PhysicalSize<i32> {
        PhysicalSize::new(self.width, self.height)
    }
}

impl From<PhysicalSize<i32>> for Size {
    fn from(pos: PhysicalSize<i32>) -> Self {
        Self {
            width: pos.width,
            height: pos.height,
        }
    }
}

impl From<PhysicalSize<u32>> for Size {
    fn from(pos: PhysicalSize<u32>) -> Self {
        Self {
            width: pos.width as i32,
            height: pos.height as i32,
        }
    }
}
