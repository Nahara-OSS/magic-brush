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
