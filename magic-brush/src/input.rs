use crate::utils::Lerpable;

#[derive(Clone, Debug)]
pub struct StylusInput {
    pub timestamp: f32,
    pub position: [f32; 2],
    pub pressure: f32,
    pub tilt: [f32; 2],
    pub twist: f32,
}

pub fn lerp_stylus_input(a: &StylusInput, b: &StylusInput, fraction: f32) -> StylusInput {
    StylusInput {
        timestamp: f32::lerp(&a.timestamp, &b.timestamp, fraction),
        position: <[f32; 2]>::lerp(&a.position, &b.position, fraction),
        pressure: f32::lerp(&a.pressure, &b.pressure, fraction),
        tilt: <[f32; 2]>::lerp(&a.tilt, &b.tilt, fraction),
        twist: lerp_angle(a.twist, b.twist, fraction),
    }
}

fn lerp_angle(a: f32, b: f32, fraction: f32) -> f32 {
    let mut diff = (b - a) % 360.0;

    if diff > 180.0 {
        diff -= 360.0;
    } else if diff < -180.0 {
        diff += 360.0;
    }

    let result = (a + diff * fraction) % 360.0;
    if result < 0.0 { result + 360.0 } else { result }
}
