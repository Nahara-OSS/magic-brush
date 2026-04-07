use std::{fmt::Display, hash::Hash, num::TryFromIntError};

use crate::{input::StylusInput, utils::lnag::Rect};

/// Trait for implementing brush renderers.
///
/// This trait is meant to be used as "template" for brush renderer implementations. The way API consumer would use the
/// brush renderers is to create enum variants for each brush type, then dispatch the brush type to correct
/// [`Renderer::use_preset`].
///
/// The order of functions to be called in this renderer is as follows:
///
/// 1. [`Renderer::use_preset`] to change brush preset, or [`Renderer::new_stroke`] to begin new stroke.
/// 1. When received new input event:
///    1. [`Renderer::next_input`] to read input event.
///    1. [`Renderer::render_begin`] to prepare for rendering to internal textures/buffers.
///    1. [`Renderer::render_input`] to render the input that [`Renderer::next_input`] just received.
///    1. [`Renderer::render_finish`] to finish rendering phase.
/// 1. When the stroke need to be drawn to view:
///    1. [`Renderer::render_begin`] to prepare for rendering to view.
///    1. [`Renderer::render_tile`] to render to view.
///    1. [`Renderer::render_finish`] to finish rendering phase.
///
/// The `P` generic parameter is for brush preset type, while the `I` generic parameter will be used as key for uniquely
/// identifying the tiles.
///
/// # Single canvas rendering
///
/// In case of single canvas (a.k.a not using tiles), the `I` generic can jsut become `()`. In this case,
/// [`Renderer::render_input`] and [`Renderer::render_tile`] may only be called once.
///
/// # Tile-based rendering
///
/// Nahara's Magic Brush was originally designed for Nahara's Sketchpad, which utilize tile-based system for
/// near-infinite drawing canvas. In this case, [`Renderer::render_input`] may be called for any tiles affected by
/// [`Renderer::next_input`] (which are the tiles intersecting the rectangle returned by this function), and
/// [`Renderer::render_tile`] may only be called for any tiles that are actually visible in viewport.
///
/// This is why the whole [`Renderer`] trait is so complicated.
pub trait Renderer<P, I: Clone + Eq + Hash> {
    /// Create new brush renderer.
    fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self;

    /// Set preset for this brush renderer.
    fn use_preset(&mut self, preset: &P) -> Result<(), Error>;

    /// Begin new stroke.
    ///
    /// Begin a new stroke by resetting everything associated with last stroke. This includes:
    ///
    /// - Dropping allocated/clearing textures/buffers with data from previous stroke.
    /// - Reset the renderer state that's associated with last stroke.
    ///
    /// Calling this function multiple times is harmless, since calling the second time will do nothing.
    fn new_stroke(&mut self) -> Result<(), Error>;

    /// Read next input event.
    ///
    /// Read the next input event and update the renderer state accordingly. This must be called before entering
    /// rendering phase.
    fn next_input(&mut self, input: &StylusInput) -> Result<Rect, Error>;

    /// Begin rendering phase.
    ///
    /// Calling this function will begin the rendering phase, which is the part where the renderer actually draw/compute
    /// something (either draw to internal texture or to texture view). Only call this once the command buffer from
    /// previous invocation is submitted.
    fn render_begin(&mut self) -> Result<(), Error>;

    /// Draw or compute internally.
    ///
    /// This function will draw or perform computation internally. The command encoder will be used to issue new render
    /// or compute passes, as well as performing copies from staging to working (uniform or storage) buffer.
    /// [`Renderer::render_begin`] must be called before calling this function.
    fn render_input(&mut self, id: &I, rect: &Rect, encoder: &mut wgpu::CommandEncoder) -> Result<(), Error>;

    /// Draw current stroke to texture view.
    ///
    /// This function basically take whatever stored to internal texture/buffer from [`Renderer::render_input`] and
    /// render it to [`wgpu::TextureView`]. The 4x4 transformation matrix can be used to transform the quad that is
    /// covering entire viewport (which is useful for displaying the stroke preview to surface for example).
    fn render_tile(
        &mut self,
        id: &I,
        transform: &[f32; 16],
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), Error>;

    /// Finish rendering phase.
    ///
    /// Calling this function will finalize the rendering phase. The rendering phase must not be re-entered after
    /// calling this function until the command buffer is submitted using [`wgpu::Queue::submit`].
    fn render_finish(&mut self) -> Result<(), Error>;
}

#[derive(Debug)]
pub enum Error {
    NoPreset,
    NoTile,
    External(Box<dyn std::error::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPreset => write!(f, "No preset assigned"),
            Self::NoTile => write!(f, "Tile with given ID is not yet rendered internally"),
            Self::External(e) => e.fmt(f),
            #[allow(unreachable_patterns)]
            _ => todo!(),
        }
    }
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::External(value)
    }
}

impl From<TryFromIntError> for Error {
    fn from(value: TryFromIntError) -> Self {
        Self::External(value.into())
    }
}
