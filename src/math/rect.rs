use num_traits::ToPrimitive;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new<T: ToPrimitive>(x: T, y: T, w: T, h: T) -> Self {
        Self {
            x: x.to_i32().unwrap_or(0),
            y: y.to_i32().unwrap_or(0),
            w: w.to_i32().unwrap_or(0),
            h: h.to_i32().unwrap_or(0),
        }
    }

    pub fn with_pos(x: i32, y: i32) -> Self {
        Self { x, y, w: 0, h: 0 }
    }

    pub fn with_size(w: i32, h: i32) -> Self {
        Self { x: 0, y: 0, w, h }
    }

    pub fn is_touch(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }
}

impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.w == other.w && self.h == other.h
    }
}

impl Into<wgpu::Extent3d> for Rect {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.w as u32,
            height: self.h as u32,
            depth_or_array_layers: 1,
        }
    }
}

impl Eq for Rect {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[allow(dead_code)]
impl RectF {
    pub fn new<T: ToPrimitive>(x: T, y: T, w: T, h: T) -> Self {
        Self {
            x: x.to_f32().unwrap_or(0.0),
            y: y.to_f32().unwrap_or(0.0),
            w: w.to_f32().unwrap_or(0.0),
            h: h.to_f32().unwrap_or(0.0),
        }
    }

    pub fn with_pos(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            w: 0.0,
            h: 0.0,
        }
    }

    pub fn with_size(w: f32, h: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w,
            h,
        }
    }

    pub fn is_touch(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }
}

impl PartialEq for RectF {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.w == other.w && self.h == other.h
    }
}

impl Eq for RectF {}
