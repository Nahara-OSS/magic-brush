//! Nahara's Magic Brush is a library implementing the stamp-based brush engine for [`wgpu`] applications, with primary
//! focus on tile-based drawing applications.
//!
//! This library is not meant to be a plug and play solution (as in you just create a "project" and build your
//! application around Magic Brush). Your application is supposed to manage the project, resources and layers, while
//! Magic Brush's purpose is to implement renderers that consume stylus input events, then draw the brush stroke using
//! [`wgpu::RenderPass`].
//!
//! The common way you'd use this library is as follows:
//!
//! - Create [`wgpu::Texture`] for storing the stroke. The texture can be a raster layer in your drawing application, or
//!   it can be your entire canvas.
//! - Create one or more of brush renderers. Magic Brush may support other type of brushes in the future, but for now,
//!   you might be able to get away with initializing [`stamp::Renderer`] only. The proper way is to make a list of
//!   [`Box<renderer::Renderer>`] ([`std::rc::Rc`] may also be used for storing active renderer).
//! - Prepare the renderer with [`renderer::Renderer::try_change_preset`]. Keep going through all brush renderers until
//!   the function returns [`true`], which at this point, you've identified the brush renderer to use.
//! - Reset the state of renderer with [`renderer::Renderer::begin_new_stroke`]. Call this everytime a new stroke need
//!   to be drawn.
//! - Every time a new [`input::StylusInput`] is received:
//!   - Update the state of renderer by using [`renderer::Renderer::prepare_input`] with received stylus input event.
//!     Note that this will also discard the state that was prepared for previous input. The returned [`renderer::Rect`]
//!     rectangular area will be used for selecting which tiles should be passed to
//!     [`renderer::Renderer::prepare_tile`].
//!   - Call [`renderer::Renderer::prepare_tile`] for each tile intersecting the [`renderer::Rect`] returned from
//!     [`renderer::Renderer::prepare_input`]. The tile ID will be used to distinguish between each tile (and may be
//!     used by brush renderer to store the tile-specific state internally).
//!   - If the stroke is still being drawn, [`renderer::Renderer::render_tile`] may be used on surface texture to
//!     preview the content.
//! - At the end of the stroke (eg: on stylus up), [`renderer::Renderer::render_tile`] should be used to draw the
//!   content to texture for storage.

pub mod dynamic;
pub mod graph;
pub mod input;
pub mod renderer;
pub mod stamp;
pub mod utils;
