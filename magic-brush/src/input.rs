#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::utils::{
    lerp::{Lerpable, lerp_angle},
    lnag::Vec2,
};

/// Struct for stylus input event data
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StylusInput {
    /// Stylus event timestamp
    ///
    /// Timestamp of the stylus input event, measured in seconds since the start of the brush stroke.
    pub timestamp: f32,

    /// Stylus position
    ///
    /// The position of the stylus input in canvas coordinates.
    pub position: Vec2,

    /// Stylus normalized pressure
    ///
    /// The normalized logical pressure in 0.00 -> 1.00 range.
    pub pressure: f32,

    /// Stylus XY tilt
    ///
    /// The tilt of the stylus along X and Y axes, respectively. Each value is measured in degrees and clamped between
    /// -90 -> +90 degrees.
    pub tilt: Vec2,

    /// Stylus twist/barrel rotation
    ///
    /// The twist/barrel rotation angle measured in degrees, clamped between 0 -> 360 degrees.
    pub twist: f32,
}

impl StylusInput {
    /// Interpolate between 2 stylus input events by some amount.
    pub fn lerp(a: &StylusInput, b: &StylusInput, fraction: f32) -> StylusInput {
        StylusInput {
            timestamp: f32::lerp(&a.timestamp, &b.timestamp, fraction),
            position: Vec2::lerp(&a.position, &b.position, fraction),
            pressure: f32::lerp(&a.pressure, &b.pressure, fraction),
            tilt: Vec2::lerp(&a.tilt, &b.tilt, fraction),
            twist: lerp_angle(a.twist, b.twist, fraction),
        }
    }
}
