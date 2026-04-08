use core::f32;

use serde::{Deserialize, Serialize};

use crate::{
    input::StylusInput,
    utils::{graph::Graph, lnag::Vec2},
};

/// Brush dynamic settings for dynamically changing parameters.
///
/// Brush dymamic controls the brush parameter based on stylus input events, like changing the size of the brush based
/// on stylus pressure value for example. Each dynamic have base value, which will be modified by a list of modifiers.
/// Note that unless the modifier graph is a linear graph, the order of modifiers is important - the first modifier will
/// be applied to base value first.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Dynamic {
    /// The base value of this dynamic settings.
    pub base: f32,

    /// A list of modifiers to apply.
    ///
    /// The modifiers are applied from the first to last element.
    pub modifiers: Vec<Modifier>,
}

impl Dynamic {
    /// Make a new [`Dynamic`] that always returns a constant value.
    pub fn constant(base: f32) -> Dynamic {
        Dynamic {
            base,
            modifiers: Vec::new(),
        }
    }

    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        let mut base = self.base;

        for modifier in &self.modifiers {
            base *= modifier.derive(ctx, a, b);
        }

        base
    }
}

pub trait DynamicArray<const N: usize> {
    fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> [f32; N];
}

impl<const N: usize> DynamicArray<N> for [Dynamic; N] {
    fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> [f32; N] {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = self[i].derive(ctx, a, b);
        }

        result
    }
}

/// Context for obtaining random values for brush dynamics.
pub trait DynamicContext {
    /// Obtain the jitter value generated for entire stroke.
    fn jitter_stroke(&self) -> f32;

    /// Obtain a new random jitter value on each invocation.
    fn jitter_dab(&mut self) -> f32;
}

/// Modify dynamic value based on stylus input events (eg: mapping brush size to stylus' logical pressure).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Modifier {
    /// The sensor type that will be used to extract value from stylus input events.
    pub sensor: Sensor,

    /// The graph that maps the sensor value. See [`Graph`] for more details.
    pub graph: Vec<Vec2>,
}

impl Modifier {
    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        self.graph.sample_graph(self.sensor.derive(ctx, a, b))
    }
}

/// Stylus sensor type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Sensor {
    /// Normalized logical pressure
    ///
    /// Use logical pressure in normalized range (0.00 to 1.00).
    #[serde(rename = "pressure")]
    Pressure,

    /// Azimuth of stylus
    ///
    /// The stylus orientation/facing direction, normalized from 0 -> 360 to 0.00 -> 1.00 range.
    #[serde(rename = "azimuth")]
    Azimuth,

    /// Altitude of stylus
    ///
    /// The angle of stylus to the drawing plane, normalized from 0 -> 90 to 0.00 -> 1.00 range.
    #[serde(rename = "altitude")]
    Altitude,

    /// Tilt X angle of stylus
    ///
    /// The tilt angle of the stylus along X axis, normalized from -90 -> +90 to 0.00 -> 1.00 range.
    #[serde(rename = "tiltX")]
    TiltX,

    /// Tilt Y angle of stylus
    ///
    /// The tilt angle of the stylus along Y axis, normalized from -90 -> +90 to 0.00 -> 1.00 range.
    #[serde(rename = "tiltY")]
    TiltY,

    /// Twist/barrel rotation of stylus
    ///
    /// The twist/rotation of the stylus around its main axis, normalized from 0 -> 360 to 0.00 -> 1.00 range.
    #[serde(rename = "twist")]
    Twist,

    /// Travelling distance from the start of stroke
    ///
    /// The travelled distance of the stylus since the start of the stroke, divided by maximum value and clamped between
    /// 0.00 -> 1.00 range.
    #[serde(rename = "distance")]
    Distance { max: f32 },

    /// Stylus movement speed
    ///
    /// The movement speed of the stylus, divded by maximum speed and clamped between 0.00 -> 1.00 range.
    #[serde(rename = "speed")]
    Speed { max: f32 },

    /// Stylus elapsed time
    ///
    /// The time elapsed since the start of the stroke, divided by maximum value and clamped between 0.00 -> 1.00 range.
    #[serde(rename = "time")]
    Time { max: f32 },

    /// Random value per stroke
    ///
    /// A random value in 0.00 -> 1.00 range selected at the start of the stroke. This value is picked randomly at the
    /// start of a stroke, and it applies to entire stroke.
    #[serde(rename = "jitterStroke")]
    JitterStroke,

    /// Random value per dab/stamp
    ///
    /// A random value in 0.00 -> 1.00 range for each time this sensor is sampled. In other words, everytime the brush
    /// renderer requested a value from this sensor, it will returns a random value.
    #[serde(rename = "jitterDab")]
    JitterDab,
}

impl Sensor {
    /// Derive sensor value from stylus events
    ///
    /// Derive sensor value that's guaranteed to be inside 0.00 -> 1.00 range from either a single or a pair of stylus
    /// input and context. The result is usually passed through a graph before multiplying with base value (or previous
    /// value if there are multiple modifiers).
    ///
    /// If there is no information on previous stylus event, the parameter `a` can be [`None`].
    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        match self {
            Sensor::Pressure => b.pressure,
            Sensor::Azimuth | Sensor::Altitude => {
                let tan_x = b.tilt.0.to_radians().tan();
                let tan_y = b.tilt.1.to_radians().tan();
                let zenith = (tan_x * tan_x + tan_y * tan_y).sqrt().atan();
                let al = 90.0 - zenith.to_degrees();
                let az = tan_x.atan2(tan_y).to_degrees();
                let az = if az < 0.0 { az + 360.0 } else { az };

                let az = if az.is_nan() || (az - 360.0).abs() <= f32::EPSILON {
                    0.0
                } else {
                    az
                };

                match self {
                    Sensor::Altitude => al / 90.0,
                    Sensor::Azimuth => az / 360.0,
                    _ => unreachable!(),
                }
            }
            Sensor::TiltX => (b.tilt.0 + 90.0) / 180.0,
            Sensor::TiltY => (b.tilt.1 + 90.0) / 180.0,
            Sensor::Twist => b.twist / 360.0,
            Sensor::Distance { .. } => todo!("distance"),
            Sensor::Speed { max } => {
                let Some(a) = a else { return 0.0 };
                let distance = (b.position - a.position).len();
                let duration = b.timestamp - a.timestamp;
                let raw_speed = distance / duration;
                (raw_speed / *max).clamp(0.0, 1.0)
            }
            Sensor::Time { .. } => todo!("time"),
            Sensor::JitterStroke => ctx.jitter_stroke(),
            Sensor::JitterDab => ctx.jitter_dab(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dynamic::{DynamicContext, Sensor},
        input::StylusInput,
        utils::lnag::Vec2,
    };

    struct TestContext;

    impl DynamicContext for TestContext {
        fn jitter_stroke(&self) -> f32 {
            0.42
        }

        fn jitter_dab(&mut self) -> f32 {
            0.67
        }
    }

    #[test]
    fn sensor_values_tilt() {
        let mut ctx = TestContext;
        let input = StylusInput {
            timestamp: 0.0,
            position: Vec2(0.0, 0.0),
            pressure: 0.0,
            tilt: Vec2(60.0, 0.0),
            twist: 0.0,
        };

        println!("{}", Sensor::Azimuth.derive(&mut ctx, None, &input));
        println!("{}", Sensor::Altitude.derive(&mut ctx, None, &input) * 90.0);
    }
}
