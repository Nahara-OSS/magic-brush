//! Nahara's Magic Brush is a library implementing the stamp-based brush engine for [`wgpu`] applications, with primary
//! focus on tile-based drawing applications.
//!
//! This library is not meant to be a plug and play solution (as in you just create a "project" and build your
//! application around Magic Brush). Your application is supposed to manage the project, resources and layers, while
//! Magic Brush's purpose is to implement renderers that consume stylus input events, then draw the brush stroke using
//! [`wgpu::CommandEncoder`].
//!
//! A typical usage of this library is as follows:
//!
//! - Create [`wgpu::Texture`] to store the result.
//! - Create brush preset and brush renderer. [`all::Brush`] and [`all::BrushRenderer`] is a good staring point.
//!   - To use a specific brush type, check out [`stamp`] (more will come in the future).
//! - Assign the preset to brush renderer with [`renderer::Renderer::use_preset`].
//! - To draw a stroke:
//!   - Call [`renderer::Renderer::new_stroke`] to reset everything but active preset.
//!   - Call [`renderer::Renderer::next_input`] when received an input event. May be called multiple times
//!     consecutively.
//!   - Call [`renderer::Renderer::render_begin`] to begin rendering.
//!   - Call [`renderer::Renderer::render_input`] on each affected tiles to render the input to internal texture/buffer.
//!   - Call [`renderer::Renderer::render_finish`] to finalize everything.
//!   - Submit the encoded command buffer. This must be done before calling [`renderer::Renderer::render_begin`] again.
//! - To display (or draw) current stroke to target [`wgpu::TextureView`]:
//!   - Call [`renderer::Renderer::render_begin`] to begin rendering.
//!   - Call [`renderer::Renderer::render_tile`] to draw to target texture view.
//!   - Call [`renderer::Renderer::render_finish`] to finalize everything.
//!   - Submit the encoded command buffer. Just like drawing a stroke, this must be done before begin rendering again.

pub mod all;
pub mod dynamic;
pub mod input;
pub mod renderer;
pub mod stamp;
pub mod utils;
