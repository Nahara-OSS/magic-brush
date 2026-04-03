use std::{any::Any, error::Error, fmt::Display, hash::Hash};

use crate::input::StylusInput;

pub trait RendererFactory {
    /// [`wgpu::Device`] and [`wgpu::Queue`] are reference counted internally, so you'd clone them in order to move to
    /// this create function.
    fn create<I: Clone + Eq + Hash>(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> impl Renderer<I>;
}

pub trait Renderer<I: Clone + Eq + Hash> {
    /// Attempt to change preset. Always assume this function will reset the internal state of renderer upon calling for
    /// any kind of input. Once this function returns `true`, the other functions can be called. Calling other functions
    /// without having a preset will return [`RendererError::NoPreset`].
    ///
    /// This function also calls [`Renderer::begin_new_stroke`] when preset is changed successfully, so there is no need
    /// to begin new stroke manually.
    fn try_change_preset(&mut self, preset: &dyn Any) -> bool;

    /// Reset internal states and prepare to begin new stroke. Does nothing when there is no active preset in this brush
    /// renderer.
    fn begin_new_stroke(&mut self);

    /// Read stylus input and parameters then update the internal states. This function may only be called once for each
    /// render. Returns the rectangle area on the canvas that intersects with the stroke. The returned area should be
    /// used to select intersecting tiles for use in [`Renderer::prepare_tile`].
    fn prepare_input(&mut self, input: &StylusInput, color: &[f32; 4]) -> Result<Rect, RendererError>;

    /// Prepare tile data for rendering. This function may only be called once for each unique tile that is intersecting
    /// the rectangular area returned from [`Renderer::prepare_input`].
    /// 
    /// The command encoder is optional, but highly recommended if there are multiple tiles that need to be prepared, or
    /// the tile preparation phase is immediately followed by tile rendering phase.
    fn prepare_tile(
        &mut self,
        tile_id: &I,
        tile_rect: &Rect,
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Result<(), RendererError>;

    /// Render the content of the tile to render pass. The render pass should only have a single color attachment and no
    /// stencil or depth attachments, and the only color attachment must have the same texture format provided from
    /// [`RendererFactory::create`].
    fn render_tile(&self, tile_id: &I, render_pass: &mut wgpu::RenderPass) -> Result<(), RendererError>;
}

#[derive(Debug)]
pub enum RendererError {
    /// No preset current active in renderer. Usually caused by [`Renderer::try_change_preset`] not yet executed or
    /// returning false.
    NoPreset,

    /// The provided tile ID is not yet used in [`Renderer::prepare_tile`].
    IdNotProcessed,
}

impl Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::NoPreset => write!(f, "No preset active in renderer"),
            RendererError::IdNotProcessed => write!(f, "Tile with given ID is not processed in process()"),
        }
    }
}

impl Error for RendererError {}

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
