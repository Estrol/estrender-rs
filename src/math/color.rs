use std::ops::*;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    pub const ALICEBLUE: Color = Self::new(0.941, 0.973, 1.0, 1.0);
    pub const ANTIQUEWHITE: Color = Self::new(0.98, 0.922, 0.843, 1.0);
    pub const AQUA: Color = Self::new(0.0, 1.0, 1.0, 1.0);
    pub const AQUAMARINE: Color = Self::new(0.498, 1.0, 0.831, 1.0);
    pub const AZURE: Color = Self::new(0.941, 1.0, 1.0, 1.0);
    pub const BEIGE: Color = Self::new(0.961, 0.961, 0.863, 1.0);
    pub const BISQUE: Color = Self::new(1.0, 0.894, 0.769, 1.0);
    pub const BLACK: Color = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const BLANCHEDALMOND: Color = Self::new(1.0, 0.922, 0.804, 1.0);
    pub const BLUE: Color = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const BLUEVIOLET: Color = Self::new(0.541, 0.169, 0.886, 1.0);
    pub const BROWN: Color = Self::new(0.647, 0.165, 0.165, 1.0);
    pub const BURLYWOOD: Color = Self::new(0.871, 0.722, 0.529, 1.0);
    pub const CADETBLUE: Color = Self::new(0.373, 0.62, 0.627, 1.0);
    pub const CHARTREUSE: Color = Self::new(0.498, 1.0, 0.0, 1.0);
    pub const CHOCOLATE: Color = Self::new(0.824, 0.412, 0.118, 1.0);
    pub const CORAL: Color = Self::new(1.0, 0.498, 0.314, 1.0);
    pub const CORNFLOWERBLUE: Color = Self::new(0.392, 0.584, 0.929, 1.0);
    pub const CORNSILK: Color = Self::new(1.0, 0.973, 0.863, 1.0);
    pub const CRIMSON: Color = Self::new(0.863, 0.078, 0.235, 1.0);
    pub const CYAN: Color = Self::new(0.0, 1.0, 1.0, 1.0);
    pub const DARKBLUE: Color = Self::new(0.0, 0.0, 0.545, 1.0);
    pub const DARKCYAN: Color = Self::new(0.0, 0.545, 0.545, 1.0);
    pub const DARKGOLDENROD: Color = Self::new(0.722, 0.525, 0.043, 1.0);
    pub const DARKGRAY: Color = Self::new(0.663, 0.663, 0.663, 1.0);
    pub const DARKGREEN: Color = Self::new(0.0, 0.392, 0.0, 1.0);
    pub const DARKKHAKI: Color = Self::new(0.741, 0.718, 0.42, 1.0);
    pub const DARKMAGENTA: Color = Self::new(0.545, 0.0, 0.545, 1.0);
    pub const DARKOLIVEGREEN: Color = Self::new(0.333, 0.42, 0.184, 1.0);
    pub const DARKORANGE: Color = Self::new(1.0, 0.549, 0.0, 1.0);
    pub const DARKORCHID: Color = Self::new(0.6, 0.196, 0.8, 1.0);
    pub const DARKRED: Color = Self::new(0.545, 0.0, 0.0, 1.0);
    pub const DARKSALMON: Color = Self::new(0.914, 0.588, 0.478, 1.0);
    pub const DARKSEAGREEN: Color = Self::new(0.561, 0.737, 0.561, 1.0);
    pub const DARKSLATEBLUE: Color = Self::new(0.282, 0.239, 0.545, 1.0);
    pub const DARKSLATEGRAY: Color = Self::new(0.184, 0.31, 0.31, 1.0);
    pub const DARKTURQUOISE: Color = Self::new(0.0, 0.808, 0.82, 1.0);
    pub const DARKVIOLET: Color = Self::new(0.58, 0.0, 0.827, 1.0);
    pub const DEEPPINK: Color = Self::new(1.0, 0.078, 0.576, 1.0);
    pub const DEEPSKYBLUE: Color = Self::new(0.0, 0.749, 1.0, 1.0);
    pub const DIMGRAY: Color = Self::new(0.412, 0.412, 0.412, 1.0);
    pub const DODGERBLUE: Color = Self::new(0.118, 0.565, 1.0, 1.0);
    pub const FIREBRICK: Color = Self::new(0.698, 0.133, 0.133, 1.0);
    pub const FLORALWHITE: Color = Self::new(1.0, 0.98, 0.941, 1.0);
    pub const FORESTGREEN: Color = Self::new(0.133, 0.545, 0.133, 1.0);
    pub const FUCHSIA: Color = Self::new(1.0, 0.0, 1.0, 1.0);
    pub const GAINSBORO: Color = Self::new(0.863, 0.863, 0.863, 1.0);
    pub const GHOSTWHITE: Color = Self::new(0.973, 0.973, 1.0, 1.0);
    pub const GOLD: Color = Self::new(1.0, 0.843, 0.0, 1.0);
    pub const GOLDENROD: Color = Self::new(0.855, 0.647, 0.125, 1.0);
    pub const GRAY: Color = Self::new(0.502, 0.502, 0.502, 1.0);
    pub const GREEN: Color = Self::new(0.0, 0.502, 0.0, 1.0);
    pub const GREENYELLOW: Color = Self::new(0.678, 1.0, 0.184, 1.0);
    pub const HONEYDEW: Color = Self::new(0.941, 1.0, 0.941, 1.0);
    pub const HOTPINK: Color = Self::new(1.0, 0.412, 0.706, 1.0);
    pub const INDIANRED: Color = Self::new(0.804, 0.361, 0.361, 1.0);
    pub const INDIGO: Color = Self::new(0.294, 0.0, 0.51, 1.0);
    pub const IVORY: Color = Self::new(1.0, 1.0, 0.941, 1.0);
    pub const KHAKI: Color = Self::new(0.941, 0.902, 0.549, 1.0);
    pub const LAVENDER: Color = Self::new(0.902, 0.902, 0.98, 1.0);
    pub const LAVENDERBLUSH: Color = Self::new(1.0, 0.941, 0.961, 1.0);
    pub const LAWNGREEN: Color = Self::new(0.486, 0.988, 0.0, 1.0);
    pub const LEMONCHIFFON: Color = Self::new(1.0, 0.98, 0.804, 1.0);
    pub const LIGHTBLUE: Color = Self::new(0.678, 0.847, 0.902, 1.0);
    pub const LIGHTCORAL: Color = Self::new(0.941, 0.502, 0.502, 1.0);
    pub const LIGHTCYAN: Color = Self::new(0.878, 1.0, 1.0, 1.0);
    pub const LIGHTGOLDENRODYELLOW: Color = Self::new(0.98, 0.98, 0.824, 1.0);
    pub const LIGHTGRAY: Color = Self::new(0.827, 0.827, 0.827, 1.0);
    pub const LIGHTGREEN: Color = Self::new(0.565, 0.933, 0.565, 1.0);
    pub const LIGHTPINK: Color = Self::new(1.0, 0.714, 0.757, 1.0);
    pub const LIGHTSALMON: Color = Self::new(1.0, 0.627, 0.478, 1.0);
    pub const LIGHTSEAGREEN: Color = Self::new(0.125, 0.698, 0.667, 1.0);
    pub const LIGHTSKYBLUE: Color = Self::new(0.529, 0.808, 0.98, 1.0);
    pub const LIGHTSLATEGRAY: Color = Self::new(0.467, 0.533, 0.6, 1.0);
    pub const LIGHTSTEELBLUE: Color = Self::new(0.69, 0.769, 0.871, 1.0);
    pub const LIGHTYELLOW: Color = Self::new(1.0, 1.0, 0.878, 1.0);
    pub const LIME: Color = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const LIMEGREEN: Color = Self::new(0.196, 0.804, 0.196, 1.0);
    pub const LINEN: Color = Self::new(0.98, 0.941, 0.902, 1.0);
    pub const MAGENTA: Color = Self::new(1.0, 0.0, 1.0, 1.0);
    pub const MAROON: Color = Self::new(0.502, 0.0, 0.0, 1.0);
    pub const MEDIUMAQUAMARINE: Color = Self::new(0.4, 0.804, 0.667, 1.0);
    pub const MEDIUMBLUE: Color = Self::new(0.0, 0.0, 0.804, 1.0);
    pub const MEDIUMORCHID: Color = Self::new(0.729, 0.333, 0.827, 1.0);
    pub const MEDIUMPURPLE: Color = Self::new(0.576, 0.439, 0.859, 1.0);
    pub const MEDIUMSEAGREEN: Color = Self::new(0.235, 0.702, 0.443, 1.0);
    pub const MEDIUMSLATEBLUE: Color = Self::new(0.482, 0.408, 0.933, 1.0);
    pub const MEDIUMSPRINGGREEN: Color = Self::new(0.0, 0.98, 0.604, 1.0);
    pub const MEDIUMTURQUOISE: Color = Self::new(0.282, 0.82, 0.8, 1.0);
    pub const MEDIUMVIOLETRED: Color = Self::new(0.78, 0.082, 0.522, 1.0);
    pub const MIDNIGHTBLUE: Color = Self::new(0.098, 0.098, 0.439, 1.0);
    pub const MINTCREAM: Color = Self::new(0.961, 1.0, 0.98, 1.0);
    pub const MISTYROSE: Color = Self::new(1.0, 0.894, 0.882, 1.0);
    pub const MOCCASIN: Color = Self::new(1.0, 0.894, 0.71, 1.0);
    pub const NAVAJOWHITE: Color = Self::new(1.0, 0.871, 0.678, 1.0);
    pub const NAVY: Color = Self::new(0.0, 0.0, 0.502, 1.0);
    pub const OLDLACE: Color = Self::new(0.992, 0.961, 0.902, 1.0);
    pub const OLIVE: Color = Self::new(0.502, 0.502, 0.0, 1.0);
    pub const OLIVEDRAB: Color = Self::new(0.42, 0.557, 0.137, 1.0);
    pub const ORANGE: Color = Self::new(1.0, 0.647, 0.0, 1.0);
    pub const ORANGERED: Color = Self::new(1.0, 0.271, 0.0, 1.0);
    pub const ORCHID: Color = Self::new(0.855, 0.439, 0.839, 1.0);
    pub const PALEGOLDENROD: Color = Self::new(0.933, 0.91, 0.667, 1.0);
    pub const PALEGREEN: Color = Self::new(0.596, 0.984, 0.596, 1.0);
    pub const PALETURQUOISE: Color = Self::new(0.686, 0.933, 0.933, 1.0);
    pub const PALEVIOLETRED: Color = Self::new(0.859, 0.439, 0.576, 1.0);
    pub const PAPAYAWHIP: Color = Self::new(1.0, 0.937, 0.835, 1.0);
    pub const PEACHPUFF: Color = Self::new(1.0, 0.855, 0.725, 1.0);
    pub const PERU: Color = Self::new(0.804, 0.522, 0.247, 1.0);
    pub const PINK: Color = Self::new(1.0, 0.753, 0.796, 1.0);
    pub const PLUM: Color = Self::new(0.867, 0.627, 0.867, 1.0);
    pub const POWDERBLUE: Color = Self::new(0.69, 0.878, 0.902, 1.0);
    pub const PURPLE: Color = Self::new(0.502, 0.0, 0.502, 1.0);
    pub const RED: Color = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const ROSYBROWN: Color = Self::new(0.737, 0.561, 0.561, 1.0);
    pub const ROYALBLUE: Color = Self::new(0.255, 0.412, 0.882, 1.0);
    pub const SADDLEBROWN: Color = Self::new(0.545, 0.271, 0.075, 1.0);
    pub const SALMON: Color = Self::new(0.98, 0.502, 0.447, 1.0);
    pub const SANDYBROWN: Color = Self::new(0.957, 0.643, 0.376, 1.0);
    pub const SEAGREEN: Color = Self::new(0.18, 0.545, 0.341, 1.0);
    pub const SEASHELL: Color = Self::new(1.0, 0.961, 0.933, 1.0);
    pub const SIENNA: Color = Self::new(0.627, 0.322, 0.176, 1.0);
    pub const SILVER: Color = Self::new(0.753, 0.753, 0.753, 1.0);
    pub const SKYBLUE: Color = Self::new(0.529, 0.808, 0.922, 1.0);
    pub const SLATEBLUE: Color = Self::new(0.416, 0.353, 0.804, 1.0);
    pub const SLATEGRAY: Color = Self::new(0.439, 0.502, 0.565, 1.0);
    pub const SNOW: Color = Self::new(1.0, 0.98, 0.98, 1.0);
    pub const SPRINGGREEN: Color = Self::new(0.0, 1.0, 0.498, 1.0);
    pub const STEELBLUE: Color = Self::new(0.275, 0.51, 0.706, 1.0);
    pub const TAN: Color = Self::new(0.824, 0.706, 0.549, 1.0);
    pub const TEAL: Color = Self::new(0.0, 0.502, 0.502, 1.0);
    pub const THISTLE: Color = Self::new(0.847, 0.749, 0.847, 1.0);
    pub const TOMATO: Color = Self::new(1.0, 0.388, 0.278, 1.0);
    pub const TURQUOISE: Color = Self::new(0.251, 0.878, 0.816, 1.0);
    pub const VIOLET: Color = Self::new(0.933, 0.51, 0.933, 1.0);
    pub const WHEAT: Color = Self::new(0.961, 0.871, 0.702, 1.0);
    pub const WHITE: Color = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const WHITESMOKE: Color = Self::new(0.961, 0.961, 0.961, 1.0);
    pub const YELLOW: Color = Self::new(1.0, 1.0, 0.0, 1.0);
    pub const YELLOWGREEN: Color = Self::new(0.604, 0.804, 0.196, 1.0);
    pub const TRANSPARENT: Color = Self::new(0.0, 0.0, 0.0, 0.0);
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        self.r == other.r && self.g == other.g && self.b == other.b && self.a == other.a
    }
}

impl Eq for Color {}

impl Add for Color {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
            a: self.a + other.a,
        }
    }
}

impl Add<f32> for Color {
    type Output = Self;

    fn add(self, other: f32) -> Self {
        Self {
            r: self.r + other,
            g: self.g + other,
            b: self.b + other,
            a: self.a + other,
        }
    }
}

impl Add<Color> for f32 {
    type Output = Color;

    fn add(self, other: Color) -> Color {
        Color {
            r: self + other.r,
            g: self + other.g,
            b: self + other.b,
            a: self + other.a,
        }
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, other: Self) {
        self.r += other.r;
        self.g += other.g;
        self.b += other.b;
        self.a += other.a;
    }
}

impl AddAssign<f32> for Color {
    fn add_assign(&mut self, other: f32) {
        self.r += other;
        self.g += other;
        self.b += other;
        self.a += other;
    }
}

impl AddAssign<Color> for f32 {
    fn add_assign(&mut self, other: Color) {
        *self += other.r;
        *self += other.g;
        *self += other.b;
        *self += other.a;
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
            a: self.a - other.a,
        }
    }
}

impl Sub<f32> for Color {
    type Output = Self;

    fn sub(self, other: f32) -> Self {
        Self {
            r: self.r - other,
            g: self.g - other,
            b: self.b - other,
            a: self.a - other,
        }
    }
}

impl Sub<Color> for f32 {
    type Output = Color;

    fn sub(self, other: Color) -> Color {
        Color {
            r: self - other.r,
            g: self - other.g,
            b: self - other.b,
            a: self - other.a,
        }
    }
}

impl SubAssign for Color {
    fn sub_assign(&mut self, other: Self) {
        self.r -= other.r;
        self.g -= other.g;
        self.b -= other.b;
        self.a -= other.a;
    }
}

impl SubAssign<f32> for Color {
    fn sub_assign(&mut self, other: f32) {
        self.r -= other;
        self.g -= other;
        self.b -= other;
        self.a -= other;
    }
}

impl SubAssign<Color> for f32 {
    fn sub_assign(&mut self, other: Color) {
        *self -= other.r;
        *self -= other.g;
        *self -= other.b;
        *self -= other.a;
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
            a: self.a * other.a,
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;

    fn mul(self, other: f32) -> Self {
        Self {
            r: self.r * other,
            g: self.g * other,
            b: self.b * other,
            a: self.a * other,
        }
    }
}

impl Mul<Color> for f32 {
    type Output = Color;

    fn mul(self, other: Color) -> Color {
        Color {
            r: self * other.r,
            g: self * other.g,
            b: self * other.b,
            a: self * other.a,
        }
    }
}

impl MulAssign for Color {
    fn mul_assign(&mut self, other: Self) {
        self.r *= other.r;
        self.g *= other.g;
        self.b *= other.b;
        self.a *= other.a;
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, other: f32) {
        self.r *= other;
        self.g *= other;
        self.b *= other;
        self.a *= other;
    }
}

impl MulAssign<Color> for f32 {
    fn mul_assign(&mut self, other: Color) {
        *self *= other.r;
        *self *= other.g;
        *self *= other.b;
        *self *= other.a;
    }
}

impl Div for Color {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self {
            r: self.r / other.r,
            g: self.g / other.g,
            b: self.b / other.b,
            a: self.a / other.a,
        }
    }
}

impl Div<f32> for Color {
    type Output = Self;

    fn div(self, other: f32) -> Self {
        Self {
            r: self.r / other,
            g: self.g / other,
            b: self.b / other,
            a: self.a / other,
        }
    }
}

impl Div<Color> for f32 {
    type Output = Color;

    fn div(self, other: Color) -> Color {
        Color {
            r: self / other.r,
            g: self / other.g,
            b: self / other.b,
            a: self / other.a,
        }
    }
}

impl DivAssign for Color {
    fn div_assign(&mut self, other: Self) {
        self.r /= other.r;
        self.g /= other.g;
        self.b /= other.b;
        self.a /= other.a;
    }
}

impl DivAssign<f32> for Color {
    fn div_assign(&mut self, other: f32) {
        self.r /= other;
        self.g /= other;
        self.b /= other;
        self.a /= other;
    }
}

impl DivAssign<Color> for f32 {
    fn div_assign(&mut self, other: Color) {
        *self /= other.r;
        *self /= other.g;
        *self /= other.b;
        *self /= other.a;
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self { r, g, b, a }
    }
}
