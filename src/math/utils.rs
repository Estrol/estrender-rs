use super::Color;

pub fn rgb_to_srgb(color: &mut Color) {
    color.r = linear_to_srgb(color.r);
    color.g = linear_to_srgb(color.g);
    color.b = linear_to_srgb(color.b);
    color.a = linear_to_srgb(color.a);
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}
