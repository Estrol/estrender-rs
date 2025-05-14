use super::Matrix4;

pub struct ModelViewProjection {
    pub model: Matrix4,
    pub view: Matrix4,
    pub projection: Matrix4,
}

impl ModelViewProjection {
    pub fn matrix4(&self) -> Matrix4 {
        self.projection * self.view * self.model
    }

    pub fn set_model(&mut self, model: Matrix4) {
        self.model = model;
    }

    pub fn set_view(&mut self, view: Matrix4) {
        self.view = view;
    }

    pub fn set_projection(&mut self, projection: Matrix4) {
        self.projection = projection;
    }

    pub unsafe fn address_of(&self) -> *const f32 {
        &self.model.m[0][0] as *const f32
    }
}
