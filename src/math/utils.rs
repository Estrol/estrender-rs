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

pub fn srgb_to_rgb(color: &mut Color) {
    color.r = srgb_to_linear(color.r);
    color.g = srgb_to_linear(color.g);
    color.b = srgb_to_linear(color.b);
    color.a = srgb_to_linear(color.a);
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
