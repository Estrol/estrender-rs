use crate::math::RectF;
use super::Texture;

pub struct Sprite {
    pub texture: Texture,
    texcoords: Vec<RectF>,
    index: usize,

    delay: f32,
    elapsed: f32,
}

impl Sprite {
    pub fn update(&mut self, dt: f32) {
        self.elapsed += dt;
        if self.elapsed >= self.delay {
            self.elapsed = 0.0;
            self.index = (self.index + 1) % self.texcoords.len();
        }
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.elapsed = 0.0;
    }

    pub fn current_texcoords(&self) -> RectF {
        self.texcoords[self.index]
    }

    pub fn texture(&self) -> &super::Texture {
        &self.texture
    }
}