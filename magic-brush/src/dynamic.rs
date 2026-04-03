use serde::{Deserialize, Serialize};

use crate::{graph::Graph, input::StylusInput, utils::Vector2Like};

/// Brush dynamic settings controls the brush parameter based on stylus input events, like changing the size of the
/// brush based on stylus pressure value for example.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Dynamic {
    /// The base value of this dynamic settings. The value will then be passed through multiple modifiers.
    pub base: f32,
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
    pub graph: Vec<[f32; 2]>,
}

impl Modifier {
    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        self.graph.sample_graph(self.sensor.derive(ctx, a, b))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Sensor {
    /// Use logical pressure in normalized range (0.00 to 1.00).
    #[serde(rename = "pressure")]
    Pressure,

    /// The stylus orientation/facing direction, normalized from 0 -> 360 to 0.00 -> 1.00 range.
    #[serde(rename = "azimuth")]
    Azimuth,

    /// The angle of stylus to the drawing plane, normalized from 0 -> 90 to 0.00 -> 1.00 range.
    #[serde(rename = "altitude")]
    Altitude,

    /// The tilt angle of the stylus along X axis, normalized from -90 -> +90 to 0.00 -> 1.00 range.
    #[serde(rename = "tiltX")]
    TiltX,

    /// The tilt angle of the stylus along Y axis, normalized from -90 -> +90 to 0.00 -> 1.00 range.
    #[serde(rename = "tiltY")]
    TiltY,

    /// The twist/rotation of the stylus around its main axis, normalized from 0 -> 360 to 0.00 -> 1.00 range.
    #[serde(rename = "twist")]
    Twist,

    /// The travelled distance of the stylus since the start of the stroke, divided by maximum value and clamped between
    /// 0.00 -> 1.00 range.
    #[serde(rename = "distance")]
    Distance { max: f32 },

    /// The movement speed of the stylus, divded by maximum speed and clamped between 0.00 -> 1.00 range.
    #[serde(rename = "speed")]
    Speed { max: f32 },

    /// The time elapsed since the start of the stroke, divided by maximum value and clamped between 0.00 -> 1.00 range.
    #[serde(rename = "time")]
    Time { max: f32 },

    /// A random value in 0.00 -> 1.00 range selected at the start of the stroke.
    #[serde(rename = "jitterStroke")]
    JitterStroke,

    /// A random value in 0.00 -> 1.00 range for each time this sensor is sampled.
    #[serde(rename = "jitterDab")]
    JitterDab,
}

impl Sensor {
    /// Derive sensor value that's guaranteed to be inside 0.00 -> 1.00 range from either a single or a pair of stylus
    /// input and context. The result is usually passed through a graph before multiplying with base value (or previous
    /// value if there are multiple modifiers).
    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        match self {
            Sensor::Pressure => b.pressure,
            Sensor::Azimuth => todo!("azimuth"),
            Sensor::Altitude => todo!("altitude"),
            Sensor::TiltX => (b.tilt[0] + 90.0) / 180.0,
            Sensor::TiltY => (b.tilt[1] + 90.0) / 180.0,
            Sensor::Twist => b.twist / 360.0,
            Sensor::Distance { .. } => todo!("distance"),
            Sensor::Speed { max } => {
                let Some(a) = a else { return 0.0 };
                let distance = b.position.vec2_sub(&a.position).vec2_len();
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
