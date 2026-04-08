use std::{fmt::Display, hash::Hash, num::TryFromIntError};

use crate::{input::StylusInput, utils::lnag::Rect};

/// Trait for implementing brush renderers.
///
/// This trait is meant to be used as "template" for brush renderer implementations. The way API consumer would use the
/// brush renderers is to create enum variants for each brush type, then dispatch the brush type to correct
/// [`Renderer::use_preset`]. Additionally, [`crate::all::BrushRenderer`] may be used if the plan is to use all
/// available brush types in Nahara's Magic Brush.
pub trait Renderer {
    /// The type for presets.
    ///
    /// This type is for presets that this renderer can accept in [`Renderer::use_preset`].
    type Preset;

    /// The type of tile ID.
    ///
    /// The tile ID is used for associating a tile with temporary resources that are allocated internally in this
    /// renderer. The resources are cleared upon calling [`Renderer::new_stroke`].
    type Id: Clone + Eq + Hash;

    /// The type for render phase.
    ///
    /// Render phase guards the renderer from being used/mutated (until the phase is dropped), as well as automatically
    /// recall/finish staging belts.
    type Phase<'phase>: RenderPhase<'phase, Id = Self::Id>
    where
        Self: 'phase;

    /// Create new brush renderer.
    fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self;

    /// Set preset for this brush renderer.
    fn use_preset(&mut self, preset: &Self::Preset) -> Result<(), Error>;

    /// Begin new stroke.
    ///
    /// Begin a new stroke by resetting everything associated with last stroke. This includes:
    ///
    /// - Dropping allocated/clearing textures/buffers with data from previous stroke.
    /// - Reset the renderer state that's associated with last stroke.
    ///
    /// Calling this function multiple times is harmless, since calling the second time will do nothing.
    fn new_stroke(&mut self) -> Result<(), Error>;

    /// Begin rendering the brush.
    ///
    /// This function begin the rendering of the brush stroke to either the internal textures/buffers or output to
    /// texture view (or both).
    ///
    /// Reason for why `color` only have 3 components is: Because each brush preset have 2 kinds of "opacity", a single
    /// opacity channel in color can be quite confusing. The opacity values are also controllable by brush dynamics, so
    /// a fixed value is not suitable.
    fn begin_render<'phase, 'input, T: IntoIterator<Item = &'input StylusInput>>(
        &'phase mut self,
        encoder: &'phase mut wgpu::CommandEncoder,
        color: &[f32; 3],
        inputs: T,
    ) -> Result<Self::Phase<'phase>, Error>;
}

/// A trait for rendering phase.
///
/// A rendering phase can be used for 2 things: to process the content of a tile or to draw the tile to views. The
/// purpose of rendering phase is to guard the renderer from being mutated during the rendering phase.
///
/// Dropping the render phase value will automatically finish the staging belt, which means once the phase is dropped,
/// [`Renderer::begin_render`] should not be called again until the command buffer is submitted.
pub trait RenderPhase<'phase> {
    type Id: Clone + Eq + Hash;

    /// The area on the canvas affected by partial stroke.
    ///
    /// This function obtains the area on the canvas that is affected by the partial stroke generated from
    /// [`Renderer::begin_render`] invocation. Use this rectangle area to determine which tiles should be provided to
    /// [`RenderPhase::process`]. May return [`None`] if `inputs` is empty when calling [`Renderer::begin_render`].
    fn bounds(&self) -> Option<Rect>;

    /// Process the tile.
    ///
    /// Internally, this function allocates temporary resources associated with tile ID (if it is not existed before),
    /// then draw whatever stored inside input buffer (usually vertex/instance buffer) to.the temporary tile resources.
    fn process(&mut self, id: &Self::Id, rect: &Rect) -> Result<(), Error>;

    /// Draw the tile content.
    ///
    /// Draw the content of the tile to texture view. The transform matrix can be used to transform the quad that is
    /// covering entire screen. When drawing the final stroke, the transform matrix is typically an identity matrix,
    /// which covers entire texture view. When drawing stroke preview, the transform matrix moves the preview to correct
    /// location, taking canvas pan, zoom or rotate into account.
    fn draw(&mut self, id: &Self::Id, transform: &[f32; 16], target: &wgpu::TextureView) -> Result<(), Error>;
}

#[derive(Debug)]
pub enum Error {
    /// Error when no preset is active.
    ///
    /// Except for [`Renderer::use_preset`], any functions may return this error when there is no preset assigned to the
    /// renderer.
    NoPreset,

    /// Error when no tile resource associated with ID found.
    ///
    /// This error occur when [`RenderPhase::process`] is not called for given tile ID throughout the stroke's lifetime.
    /// If this error occurred when using [`RenderPhase::draw`], it can safely be ignored (nothing will be drawn to the
    /// texture view).
    NoTile,

    /// Error occured outside what [`Error`] can covers.
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
