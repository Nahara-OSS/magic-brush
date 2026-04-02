# Nahara's Magic Brush Engine

## Introduction

Nahara's Magic Brush is a brush engine for [wgpu][wgpu]-based drawing apps, such
as Nahara's Sketchpad or Nahara's Canvas.

Magic Brush is meant to be used as a library: Your app is supposed to handle the
device initialization and managing textures/artwork data, while Magic Brush only
handle the part that draws the stroke by writing to command buffer encoder.

## Packages overview

- `magic-brush`: The main Rust library
- `magic-brush-demo`: Demo web application (ideally it should be part of
  `examples/`)

## Backends

Magic Brush only supports the backends that have first-class support in wgpu.
This includes Vulkan (Windows, Linux and Android), Direct3D 12 (Windows), Metal
(macOS) and browser WebGPU. Any "best effort support" platforms are not
supported.

| Operating system | Suggested backend | Supported backends       |
| ---------------- | ----------------- | ------------------------ |
| Windows          | Direct3D 12       | Direct3D 12, Vulkan      |
| Linux            | Vulkan            | Vulkan                   |
| Android          | Vulkan            | Vulkan                   |
| macOS/iOS/iPadOS | Metal             | Metal, Vulkan (MoltenVK) |
| (browser)        | WebGPU            | WebGPU                   |

OpenGL/WebGL is not supported on any platforms due to unexpected issues during
testing. However this does not stop the library from trying to run on
OpenGL/WebGL platforms.

## Brushes

Magic Brush contains a single brush type at current moment:

- **Stamp**: Stamp-based brush basically stamp a bunch of images to texture
  view.

### Stamp-based brushes

The idea of stamp-based brush is that the stroke can be created by stamping a
bunch of images along the path, each spaced by fixed amount, and small enough
spacing can create a consistent stroke. This is also the most popular brush type
implemented in most professional drawing apps.

Magic Brush achieves this by using instancing: each instance represent a stamp,
and all instances are stored inside instance buffer.

### Strip-based brushes (Work in progress)

Strip-based brush is like sticking textured tape along the path. The image for
strip-based brush is a long (or wide) image that will be bent to fit along the
path. This brush type is suitable for strip-like objects like chains, frills or
grid.

Currently figuring out the way to generate the triangle mesh for this brush type
(ideally using triangle strip).

## License

Nahara's Magic Brush is licensed under [MIT License][license].

[wgpu]: https://github.com/gfx-rs/wgpu
[license]: LICENSE
