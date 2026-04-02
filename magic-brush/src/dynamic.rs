use serde::{Deserialize, Serialize};

use crate::{graph::Graph, input::StylusInput, utils::Vector2Like};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Dynamic {
    pub base: f32,
    pub modifiers: Vec<Modifier>,
}

impl Dynamic {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Modifier {
    pub sensor: Sensor,
    pub graph: Vec<[f32; 2]>,
}

impl Modifier {
    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        self.graph.sample_graph(self.sensor.derive(ctx, a, b))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Sensor {
    Pressure,
    Azimuth,
    Altitude,
    TiltX,
    TiltY,
    Twist,
    Distance { max: f32 },
    Speed { max: f32 },
    Time { max: f32 },
    JitterStroke,
    JitterDab,
}

impl Sensor {
    pub fn derive(&self, ctx: &mut dyn DynamicContext, a: Option<&StylusInput>, b: &StylusInput) -> f32 {
        match self {
            Sensor::Pressure => b.pressure,
            Sensor::Azimuth => todo!(),
            Sensor::Altitude => todo!(),
            Sensor::TiltX => (b.tilt[0] + 90.0) / 180.0,
            Sensor::TiltY => (b.tilt[1] + 90.0) / 180.0,
            Sensor::Twist => b.twist / 360.0,
            Sensor::Distance { .. } => todo!(),
            Sensor::Speed { max } => {
                let Some(a) = a else { return 0.0 };
                let distance = b.position.vec2_sub(&a.position).vec2_len();
                let duration = b.timestamp - a.timestamp;
                let raw_speed = distance / duration;
                (raw_speed / *max).clamp(0.0, 1.0)
            }
            Sensor::Time { .. } => todo!(),
            Sensor::JitterStroke => ctx.jitter_stroke(),
            Sensor::JitterDab => ctx.jitter_dab(),
        }
    }
}
