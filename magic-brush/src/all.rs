//! Module combining all brush types.
//!
//! The purpose of this module is to combine all sort of brush types implemented in Nahara's Magic Brush library into a
//! single enum/renderer.

use std::hash::Hash;

use serde::{Deserialize, Serialize};

use crate::{
    input::StylusInput,
    renderer::{Error, RenderPhase, Renderer},
    stamp::{StampBrush, StampBrushRenderer, StampRenderPhase},
    utils::lnag::Rect,
};

/// An enum with all brush types implemented in Nahara's Magic Brush.
#[derive(Serialize, Deserialize)]
pub enum Brush {
    #[serde(rename = "stamp")]
    Stamp(StampBrush),
}

pub struct BrushRenderer<I: Clone + Eq + Hash> {
    active: ActiveRenderer,
    stamp: StampBrushRenderer<I>,
}

enum ActiveRenderer {
    None,
    Stamp,
}

impl<I: Clone + Eq + Hash> Renderer for BrushRenderer<I> {
    type Preset = Brush;
    type Id = I;
    type Phase<'phase>
        = BrushRenderPhase<'phase, Self::Id>
    where
        Self: 'phase;

    fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self {
            active: ActiveRenderer::None,
            stamp: StampBrushRenderer::new(device.clone(), queue.clone(), format),
        }
    }

    fn use_preset(&mut self, preset: &Brush) -> Result<(), Error> {
        match preset {
            Brush::Stamp(preset) => {
                self.active = ActiveRenderer::Stamp;
                self.stamp.use_preset(preset)
            }
        }
    }

    fn new_stroke(&mut self) -> Result<(), Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.new_stroke(),
        }
    }

    fn begin_render<'phase, 'input, T: IntoIterator<Item = &'input StylusInput>>(
        &'phase mut self,
        encoder: &'phase mut wgpu::CommandEncoder,
        color: &[f32; 3],
        inputs: T,
    ) -> Result<Self::Phase<'phase>, Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self
                .stamp
                .begin_render(encoder, color, inputs)
                .map(BrushRenderPhase::Stamp),
        }
    }
}

pub enum BrushRenderPhase<'phase, I: Clone + Eq + Hash> {
    Stamp(StampRenderPhase<'phase, I>),
}

impl<'phase, I: Clone + Eq + Hash> RenderPhase<'phase> for BrushRenderPhase<'phase, I> {
    type Id = I;

    fn bounds(&self) -> Option<Rect> {
        match self {
            BrushRenderPhase::Stamp(phase) => phase.bounds(),
        }
    }

    fn process(&mut self, id: &I, rect: &Rect) -> Result<(), Error> {
        match self {
            BrushRenderPhase::Stamp(phase) => phase.process(id, rect),
        }
    }

    fn draw(&mut self, id: &I, transform: &[f32; 16], target: &wgpu::TextureView) -> Result<(), Error> {
        match self {
            BrushRenderPhase::Stamp(phase) => phase.draw(id, transform, target),
        }
    }
}
