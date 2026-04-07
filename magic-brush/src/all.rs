//! Module combining all brush types.
//!
//! The purpose of this module is to combine all sort of brush types implemented in Nahara's Magic Brush library into a
//! single enum/renderer.

use std::hash::Hash;

use serde::{Deserialize, Serialize};

use crate::{
    input::StylusInput,
    renderer::{Error, Renderer},
    stamp::{StampBrush, StampBrushRenderer},
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

impl<I: Clone + Eq + Hash> Renderer<Brush, I> for BrushRenderer<I> {
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
            #[allow(unreachable_patterns)]
            _ => todo!(),
        }
    }

    fn new_stroke(&mut self) -> Result<(), Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.new_stroke(),
        }
    }

    fn next_input(&mut self, input: &StylusInput, color: [f32; 3]) -> Result<Rect, Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.next_input(input, color),
        }
    }

    fn render_begin(&mut self) -> Result<(), Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.render_begin(),
        }
    }

    fn render_input(&mut self, id: &I, rect: &Rect, encoder: &mut wgpu::CommandEncoder) -> Result<(), Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.render_input(id, rect, encoder),
        }
    }

    fn render_tile(
        &mut self,
        id: &I,
        transform: &[f32; 16],
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.render_tile(id, transform, target, encoder),
        }
    }

    fn render_finish(&mut self) -> Result<(), Error> {
        match self.active {
            ActiveRenderer::None => Err(Error::NoPreset),
            ActiveRenderer::Stamp => self.stamp.render_finish(),
        }
    }
}
