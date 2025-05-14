use winit::dpi::PhysicalPosition;

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn one() -> Self {
        Self { x: 1, y: 1 }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::zero()
    }
}

use std::ops::*;

impl Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Position {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Position {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<i32> for Position {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl MulAssign<i32> for Position {
    fn mul_assign(&mut self, rhs: i32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<i32> for Position {
    type Output = Self;

    fn div(self, rhs: i32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl DivAssign<i32> for Position {
    fn div_assign(&mut self, rhs: i32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for Position {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for Position {}

impl Into<PhysicalPosition<u32>> for Position {
    fn into(self) -> PhysicalPosition<u32> {
        PhysicalPosition::new(self.x as u32, self.y as u32)
    }
}

impl Into<PhysicalPosition<i32>> for Position {
    fn into(self) -> PhysicalPosition<i32> {
        PhysicalPosition::new(self.x, self.y)
    }
}

impl From<PhysicalPosition<i32>> for Position {
    fn from(pos: PhysicalPosition<i32>) -> Self {
        Self { x: pos.x, y: pos.y }
    }
}

impl From<PhysicalPosition<u32>> for Position {
    fn from(pos: PhysicalPosition<u32>) -> Self {
        Self {
            x: pos.x as i32,
            y: pos.y as i32,
        }
    }
}
