use std::ops::{Add, Mul, Sub};

use bytemuck::{Pod, Zeroable};
use num_traits::ToPrimitive;

use super::{Vector2, Vector3, Vector4};

#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Matrix4 {
    pub m: [[f32; 4]; 4],
}

impl Matrix4 {
    pub fn new() -> Self {
        Self {
            m: [
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ],
        }
    }

    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn look_at(eye: Vector3, target: Vector3, up: Vector3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(&up.normalize()).normalize();
        let u = s.cross(&f);

        Self {
            m: [
                [s.x, s.y, s.z, -s.dot(&eye)],
                [u.x, u.y, u.z, -u.dot(&eye)],
                [-f.x, -f.y, -f.z, f.dot(&eye)],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn frustum<T: ToPrimitive>(left: T, right: T, bottom: T, top: T, near: T, far: T) -> Self {
        let left = left.to_f32().unwrap();
        let right = right.to_f32().unwrap();
        let bottom = bottom.to_f32().unwrap();
        let top = top.to_f32().unwrap();
        let near = near.to_f32().unwrap();
        let far = far.to_f32().unwrap();

        let rl = 1.0 / (right - left);
        let bt = 1.0 / (top - bottom);
        let nf = 1.0 / (near - far);

        Self {
            m: [
                [2.0 * near * rl, 0.0, 0.0, 0.0],
                [0.0, 2.0 * near * bt, 0.0, 0.0],
                [
                    (right + left) * rl,
                    (top + bottom) * bt,
                    (far + near) * nf,
                    -1.0,
                ],
                [0.0, 0.0, 2.0 * far * near * nf, 0.0],
            ],
        }
    }

    pub fn perspective<T: ToPrimitive>(fov: T, aspect: T, near: T, far: T) -> Self {
        let fov = fov.to_f32().unwrap();
        let aspect = aspect.to_f32().unwrap();
        let near = near.to_f32().unwrap();
        let far = far.to_f32().unwrap();

        let f = 1.0 / (fov / 2.0).tan();
        let nf = 1.0 / (near - far);

        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, 2.0 * far * near * nf],
                [0.0, 0.0, -1.0, 0.0],
            ],
        }
    }

    pub fn translate<T: ToPrimitive>(x: T, y: T, z: T) -> Self {
        let x = x.to_f32().unwrap();
        let y = y.to_f32().unwrap();
        let z = z.to_f32().unwrap();

        Self {
            m: [
                [1.0, 0.0, 0.0, x],
                [0.0, 1.0, 0.0, y],
                [0.0, 0.0, 1.0, z],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn scale<T: ToPrimitive>(x: T, y: T, z: T) -> Self {
        let x = x.to_f32().unwrap();
        let y = y.to_f32().unwrap();
        let z = z.to_f32().unwrap();

        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn orthographic<T: ToPrimitive>(
        left: T,
        right: T,
        bottom: T,
        top: T,
        near: T,
        far: T,
    ) -> Self {
        let left = left.to_f32().unwrap();
        let right = right.to_f32().unwrap();
        let bottom = bottom.to_f32().unwrap();
        let top = top.to_f32().unwrap();
        let near = near.to_f32().unwrap();
        let far = far.to_f32().unwrap();

        let lr = 1.0 / (left - right);
        let bt = 1.0 / (bottom - top);
        let nf = 1.0 / (near - far);

        Self {
            m: [
                [-2.0 * lr, 0.0, 0.0, (left + right) * lr],
                [0.0, -2.0 * bt, 0.0, (top + bottom) * bt],
                [0.0, 0.0, 2.0 * nf, (far + near) * nf],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotate<T: ToPrimitive>(angle: T, x: T, y: T, z: T) -> Self {
        let angle = angle.to_f32().unwrap();
        let x = x.to_f32().unwrap();
        let y = y.to_f32().unwrap();
        let z = z.to_f32().unwrap();

        let c = angle.cos();
        let s = angle.sin();
        let len = (x * x + y * y + z * z).sqrt();
        let (x, y, z) = if len == 0.0 {
            (1.0, 0.0, 0.0)
        } else {
            (x / len, y / len, z / len)
        };
        let omc = 1.0 - c;

        Self {
            m: [
                [
                    x * x * omc + c,
                    x * y * omc - z * s,
                    x * z * omc + y * s,
                    0.0,
                ],
                [
                    y * x * omc + z * s,
                    y * y * omc + c,
                    y * z * omc - x * s,
                    0.0,
                ],
                [
                    z * x * omc - y * s,
                    z * y * omc + x * s,
                    z * z * omc + c,
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn transform_point(&self, point: Vector3) -> Vector3 {
        let mut result = Vector3::new(0.0, 0.0, 0.0);

        result.x = self.m[0][0] * point.x + self.m[0][1] * point.y + self.m[0][2] * point.z;
        result.y = self.m[1][0] * point.x + self.m[1][1] * point.y + self.m[1][2] * point.z;
        result.z = self.m[2][0] * point.x + self.m[2][1] * point.y + self.m[2][2] * point.z;

        result
    }

    pub unsafe fn address_of(&self) -> *const f32 {
        &self.m[0][0] as *const f32
    }

    pub fn get_fov(&self) -> f32 {
        let f = self.m[1][1];
        1.0 / f.atan() * 2.0
    }

    pub fn get_aspect(&self) -> f32 {
        self.m[0][0] / self.m[1][1]
    }

    pub fn get_near(&self) -> f32 {
        let nf = 1.0 / self.m[2][2];
        (2.0 * self.m[3][2]) / (self.m[2][2] - nf)
    }

    pub fn inverse(&self) -> Matrix4 {
        let m = &self.m;

        let mut inv = [[0.0; 4]; 4];

        inv[0][0] =
            m[1][1] * m[2][2] * m[3][3] - m[1][1] * m[2][3] * m[3][2] - m[2][1] * m[1][2] * m[3][3]
                + m[2][1] * m[1][3] * m[3][2]
                + m[3][1] * m[1][2] * m[2][3]
                - m[3][1] * m[1][3] * m[2][2];
        inv[0][1] = -m[0][1] * m[2][2] * m[3][3]
            + m[0][1] * m[2][3] * m[3][2]
            + m[2][1] * m[0][2] * m[3][3]
            - m[2][1] * m[0][3] * m[3][2]
            - m[3][1] * m[0][2] * m[2][3]
            + m[3][1] * m[0][3] * m[2][2];
        inv[0][2] =
            m[0][1] * m[1][2] * m[3][3] - m[0][1] * m[1][3] * m[3][2] - m[1][1] * m[0][2] * m[3][3]
                + m[1][1] * m[0][3] * m[3][2]
                + m[3][1] * m[0][2] * m[1][3]
                - m[3][1] * m[0][3] * m[1][2];
        inv[0][3] = -m[0][1] * m[1][2] * m[2][3]
            + m[0][1] * m[1][3] * m[2][2]
            + m[1][1] * m[0][2] * m[2][3]
            - m[1][1] * m[0][3] * m[2][2]
            - m[2][1] * m[0][2] * m[1][3]
            + m[2][1] * m[0][3] * m[1][2];

        inv[1][0] = -m[1][0] * m[2][2] * m[3][3]
            + m[1][0] * m[2][3] * m[3][2]
            + m[2][0] * m[1][2] * m[3][3]
            - m[2][0] * m[1][3] * m[3][2]
            - m[3][0] * m[1][2] * m[2][3]
            + m[3][0] * m[1][3] * m[2][2];
        inv[1][1] =
            m[0][0] * m[2][2] * m[3][3] - m[0][0] * m[2][3] * m[3][2] - m[2][0] * m[0][2] * m[3][3]
                + m[2][0] * m[0][3] * m[3][2]
                + m[3][0] * m[0][2] * m[2][3]
                - m[3][0] * m[0][3] * m[2][2];
        inv[1][2] = -m[0][0] * m[1][2] * m[3][3]
            + m[0][0] * m[1][3] * m[3][2]
            + m[1][0] * m[0][2] * m[3][3]
            - m[1][0] * m[0][3] * m[3][2]
            - m[3][0] * m[0][2] * m[1][3]
            + m[3][0] * m[0][3] * m[1][2];
        inv[1][3] =
            m[0][0] * m[1][2] * m[2][3] - m[0][0] * m[1][3] * m[2][2] - m[1][0] * m[0][2] * m[2][3]
                + m[1][0] * m[0][3] * m[2][2]
                + m[2][0] * m[0][2] * m[1][3]
                - m[2][0] * m[0][3] * m[1][2];

        inv[2][0] =
            m[1][0] * m[2][1] * m[3][3] - m[1][0] * m[2][3] * m[3][1] - m[2][0] * m[1][1] * m[3][3]
                + m[2][0] * m[1][3] * m[3][1]
                + m[3][0] * m[1][1] * m[2][3]
                - m[3][0] * m[1][3] * m[2][1];
        inv[2][1] = -m[0][0] * m[2][1] * m[3][3]
            + m[0][0] * m[2][3] * m[3][1]
            + m[2][0] * m[0][1] * m[3][3]
            - m[2][0] * m[0][3] * m[3][1]
            - m[3][0] * m[0][1] * m[2][3]
            + m[3][0] * m[0][3] * m[2][1];
        inv[2][2] =
            m[0][0] * m[1][1] * m[3][3] - m[0][0] * m[1][3] * m[3][1] - m[1][0] * m[0][1] * m[3][3]
                + m[1][0] * m[0][3] * m[3][1]
                + m[3][0] * m[0][1] * m[1][3]
                - m[3][0] * m[0][3] * m[1][1];
        inv[2][3] = -m[0][0] * m[1][1] * m[2][3]
            + m[0][0] * m[1][3] * m[2][1]
            + m[1][0] * m[0][1] * m[2][3]
            - m[1][0] * m[0][3] * m[2][1]
            - m[2][0] * m[0][1] * m[1][3]
            + m[2][0] * m[0][3] * m[1][1];

        inv[3][0] = -m[1][0] * m[2][1] * m[3][2]
            + m[1][0] * m[2][2] * m[3][1]
            + m[2][0] * m[1][1] * m[3][2]
            - m[2][0] * m[1][2] * m[3][1]
            - m[3][0] * m[1][1] * m[2][2]
            + m[3][0] * m[1][2] * m[2][1];
        inv[3][1] =
            m[0][0] * m[2][1] * m[3][2] - m[0][0] * m[2][2] * m[3][1] - m[2][0] * m[0][1] * m[3][2]
                + m[2][0] * m[0][2] * m[3][1]
                + m[3][0] * m[0][1] * m[2][2]
                - m[3][0] * m[0][2] * m[2][1];
        inv[3][2] = -m[0][0] * m[1][1] * m[3][2]
            + m[0][0] * m[1][2] * m[3][1]
            + m[1][0] * m[0][1] * m[3][2]
            - m[1][0] * m[0][2] * m[3][1]
            - m[3][0] * m[0][1] * m[1][2]
            + m[3][0] * m[0][2] * m[1][1];
        inv[3][3] =
            m[0][0] * m[1][1] * m[2][2] - m[0][0] * m[1][2] * m[2][1] - m[1][0] * m[0][1] * m[2][2]
                + m[1][0] * m[0][2] * m[2][1]
                + m[2][0] * m[0][1] * m[1][2]
                - m[2][0] * m[0][2] * m[1][1];

        let det =
            m[0][0] * inv[0][0] + m[0][1] * inv[1][0] + m[0][2] * inv[2][0] + m[0][3] * inv[3][0];

        if det == 0.0 {
            return Matrix4::identity();
        }

        let det = 1.0 / det;

        for i in 0..4 {
            for j in 0..4 {
                inv[i][j] *= det;
            }
        }

        Matrix4 { m: inv }
    }

    pub const OPENGL_TO_WGPU_MATRIX: Self = Self {
        m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.5],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
}

impl PartialEq for Matrix4 {
    fn eq(&self, other: &Self) -> bool {
        self.m[0] == other.m[0]
            && self.m[1] == other.m[1]
            && self.m[2] == other.m[2]
            && self.m[3] == other.m[3]
    }
}

impl Eq for Matrix4 {}

impl Mul for Matrix4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut result = Self::new();

        for i in 0..4 {
            for j in 0..4 {
                result.m[i][j] = self.m[i][0] * rhs.m[0][j]
                    + self.m[i][1] * rhs.m[1][j]
                    + self.m[i][2] * rhs.m[2][j]
                    + self.m[i][3] * rhs.m[3][j];
            }
        }

        result
    }
}

impl Add for Matrix4 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let mut result = Self::new();

        for i in 0..4 {
            for j in 0..4 {
                result.m[i][j] = self.m[i][j] + rhs.m[i][j];
            }
        }

        result
    }
}

impl Sub for Matrix4 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        let mut result = Self::new();

        for i in 0..4 {
            for j in 0..4 {
                result.m[i][j] = self.m[i][j] - rhs.m[i][j];
            }
        }

        result
    }
}

impl Mul<Vector2> for Matrix4 {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Vector2 {
        let mut result = Vector2::new(0.0, 0.0);

        result.x = self.m[0][0] * rhs.x + self.m[0][1] * rhs.y + self.m[0][3];
        result.y = self.m[1][0] * rhs.x + self.m[1][1] * rhs.y + self.m[1][3];

        result
    }
}

impl Mul<Vector3> for Matrix4 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Vector3 {
        let mut result = Vector3::new(0.0, 0.0, 0.0);

        result.x =
            self.m[0][0] * rhs.x + self.m[0][1] * rhs.y + self.m[0][2] * rhs.z + self.m[0][3];
        result.y =
            self.m[1][0] * rhs.x + self.m[1][1] * rhs.y + self.m[1][2] * rhs.z + self.m[1][3];
        result.z =
            self.m[2][0] * rhs.x + self.m[2][1] * rhs.y + self.m[2][2] * rhs.z + self.m[2][3];

        result
    }
}

impl Mul<Vector4> for Matrix4 {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Vector4 {
        let mut result = Vector4::new(0.0, 0.0, 0.0, 0.0);

        result.x = self.m[0][0] * rhs.x
            + self.m[0][1] * rhs.y
            + self.m[0][2] * rhs.z
            + self.m[0][3] * rhs.w;
        result.y = self.m[1][0] * rhs.x
            + self.m[1][1] * rhs.y
            + self.m[1][2] * rhs.z
            + self.m[1][3] * rhs.w;
        result.z = self.m[2][0] * rhs.x
            + self.m[2][1] * rhs.y
            + self.m[2][2] * rhs.z
            + self.m[2][3] * rhs.w;
        result.w = self.m[3][0] * rhs.x
            + self.m[3][1] * rhs.y
            + self.m[3][2] * rhs.z
            + self.m[3][3] * rhs.w;

        result
    }
}
