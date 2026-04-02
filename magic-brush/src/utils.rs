pub trait Vector2Like {
    type Scalar;

    fn vec2_zero() -> Self;
    fn vec2_add(&self, other: &Self) -> Self;
    fn vec2_sub(&self, other: &Self) -> Self;
    fn vec2_scale(&self, value: Self::Scalar) -> Self;
    fn vec2_len(&self) -> Self::Scalar;
    fn vec2_normalized(&self) -> Self;
}

impl Vector2Like for [f32; 2] {
    type Scalar = f32;

    fn vec2_zero() -> Self {
        [0.0, 0.0]
    }

    fn vec2_add(&self, other: &Self) -> Self {
        [self[0] + other[0], self[1] + other[1]]
    }

    fn vec2_sub(&self, other: &Self) -> Self {
        [self[0] - other[0], self[1] - other[1]]
    }

    fn vec2_scale(&self, value: Self::Scalar) -> Self {
        [self[0] * value, self[1] * value]
    }

    fn vec2_len(&self) -> Self::Scalar {
        (self[0] * self[0] + self[1] * self[1]).sqrt()
    }

    fn vec2_normalized(&self) -> Self {
        let len = self.vec2_len();
        [self[0] / len, self[1] / len]
    }
}

pub trait Matrix4x4Like {
    type Scalar;
    type Vector;

    fn mat4x4_identity() -> Self;
    fn mat4x4_scale(x: Self::Scalar, y: Self::Scalar, z: Self::Scalar) -> Self;
    fn mat4x4_row(&self, i: usize) -> Self::Vector;
    fn mat4x4_col(&self, i: usize) -> Self::Vector;
    fn mat4x4_mul(&self, other: &Self) -> Self;
}

impl Matrix4x4Like for [f32; 16] {
    type Scalar = f32;
    type Vector = [f32; 4];

    fn mat4x4_identity() -> Self {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn mat4x4_scale(x: Self::Scalar, y: Self::Scalar, z: Self::Scalar) -> Self {
        [x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0]
    }

    fn mat4x4_row(&self, i: usize) -> Self::Vector {
        let start = i * 4;
        [self[start], self[start + 1], self[start + 2], self[start + 3]]
    }

    fn mat4x4_col(&self, i: usize) -> Self::Vector {
        [self[i], self[i + 4], self[i + 8], self[i + 12]]
    }

    fn mat4x4_mul(&self, other: &Self) -> Self {
        [
            self[0] * other[0] + self[1] * other[4] + self[2] * other[8] * self[3] * other[12],
            self[0] * other[1] + self[1] * other[5] + self[2] * other[9] * self[3] * other[13],
            self[0] * other[2] + self[1] * other[6] + self[2] * other[10] * self[3] * other[14],
            self[0] * other[3] + self[1] * other[7] + self[2] * other[11] * self[3] * other[15],
            self[4] * other[0] + self[5] * other[4] + self[6] * other[8] * self[7] * other[12],
            self[4] * other[1] + self[5] * other[5] + self[6] * other[9] * self[7] * other[13],
            self[4] * other[2] + self[5] * other[6] + self[6] * other[10] * self[7] * other[14],
            self[4] * other[3] + self[5] * other[7] + self[6] * other[11] * self[7] * other[15],
            self[8] * other[0] + self[9] * other[4] + self[10] * other[8] * self[11] * other[12],
            self[8] * other[1] + self[9] * other[5] + self[10] * other[9] * self[11] * other[13],
            self[8] * other[2] + self[9] * other[6] + self[10] * other[10] * self[11] * other[14],
            self[8] * other[3] + self[9] * other[7] + self[10] * other[11] * self[11] * other[15],
            self[12] * other[0] + self[13] * other[4] + self[14] * other[8] * self[15] * other[12],
            self[12] * other[1] + self[13] * other[5] + self[14] * other[9] * self[15] * other[13],
            self[12] * other[2] + self[13] * other[6] + self[14] * other[10] * self[15] * other[14],
            self[12] * other[3] + self[13] * other[7] + self[14] * other[11] * self[15] * other[15],
        ]
    }
}

pub trait Lerpable {
    fn lerp(a: &Self, b: &Self, fraction: f32) -> Self;
}

impl Lerpable for f32 {
    fn lerp(a: &Self, b: &Self, fraction: f32) -> Self {
        a * (1.0 - fraction) + b * fraction
    }
}

impl<const N: usize> Lerpable for [f32; N] {
    fn lerp(a: &Self, b: &Self, fraction: f32) -> Self {
        let mut result = *a;
        let inv_frac = 1.0 - fraction;

        for i in 0..N {
            result[i] = a[i] * inv_frac + b[i] * fraction;
        }

        result
    }
}

pub trait FromNormalized {
    fn from_normalized(value: f32) -> Self;
}

impl FromNormalized for u8 {
    fn from_normalized(value: f32) -> Self {
        (value.clamp(0.0, 1.0) * 255.0) as u8
    }
}

impl FromNormalized for i8 {
    fn from_normalized(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * 127.0) as i8
    }
}

impl FromNormalized for f32 {
    fn from_normalized(value: f32) -> Self {
        value
    }
}

impl FromNormalized for f64 {
    fn from_normalized(value: f32) -> Self {
        value as f64
    }
}
