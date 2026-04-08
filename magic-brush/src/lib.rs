//! Nahara's Magic Brush is a library implementing the stamp-based brush engine for [`wgpu`] applications, with primary
//! focus on tile-based drawing applications.
//!
//! This library is not meant to be a plug and play solution (as in you just create a "project" and build your
//! application around Magic Brush). Your application is supposed to manage the project, resources and layers, while
//! Magic Brush's purpose is to implement renderers that consume stylus input events, then draw the brush stroke using
//! [`wgpu::CommandEncoder`].

pub mod all;
pub mod dynamic;
pub mod input;
pub mod renderer;
pub mod stamp;
pub mod utils;
