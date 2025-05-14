#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn with_pos(x: i32, y: i32) -> Self {
        Self { x, y, w: 0, h: 0 }
    }

    pub fn with_size(w: i32, h: i32) -> Self {
        Self { x: 0, y: 0, w, h }
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
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

impl PartialEq for RectF {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.w == other.w && self.h == other.h
    }
}

impl Eq for RectF {}
