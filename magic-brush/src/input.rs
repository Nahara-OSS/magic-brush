use serde::{Deserialize, Serialize};

use crate::utils::Lerpable;

/// Struct for stylus input event data. May be serialized or deserialized with [`serde`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StylusInput {
    /// Timestamp of the stylus input event, measured in seconds since the start of the brush stroke.
    pub timestamp: f32,

    /// The position of the stylus input in canvas coordinates.
    pub position: [f32; 2],

    /// The normalized logical pressure in 0.00 -> 1.00 range.
    pub pressure: f32,

    /// The tilt of the stylus along X and Y axes, respectively. Each value is measured in degrees and clamped between
    /// -90 -> +90 degrees.
    pub tilt: [f32; 2],

    /// The twist/barrel rotation angle measured in degrees, clamped between 0 -> 360 degrees.
    pub twist: f32,
}

impl StylusInput {
    /// Interpolate between 2 stylus input events by some amount.
    pub fn lerp(a: &StylusInput, b: &StylusInput, fraction: f32) -> StylusInput {
        StylusInput {
            timestamp: f32::lerp(&a.timestamp, &b.timestamp, fraction),
            position: <[f32; 2]>::lerp(&a.position, &b.position, fraction),
            pressure: f32::lerp(&a.pressure, &b.pressure, fraction),
            tilt: <[f32; 2]>::lerp(&a.tilt, &b.tilt, fraction),
            twist: lerp_angle(a.twist, b.twist, fraction),
        }
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
