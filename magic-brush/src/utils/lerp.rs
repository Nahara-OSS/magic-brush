pub trait Lerpable {
    fn lerp(a: &Self, b: &Self, fraction: f32) -> Self;
}

impl Lerpable for f32 {
    fn lerp(a: &Self, b: &Self, fraction: f32) -> Self {
        a * (1.0 - fraction) + b * fraction
    }
}

pub fn lerp_angle(a: f32, b: f32, fraction: f32) -> f32 {
    let mut diff = (b - a) % 360.0;

    if diff > 180.0 {
        diff -= 360.0;
    } else if diff < -180.0 {
        diff += 360.0;
    }

    let result = (a + diff * fraction) % 360.0;
    if result < 0.0 { result + 360.0 } else { result }
}
