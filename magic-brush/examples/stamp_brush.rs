//! Demo the stamp brush engine.

use std::{error::Error, fs::File, io::Write};

use magic_brush::{
    all::{Brush, BrushRenderer},
    dynamic::{Dynamic, Modifier, Sensor},
    input::StylusInput,
    renderer::Renderer,
    stamp::{BrushTip, StampBrush},
    utils::lnag::{Rect, Vec2},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Assume you already know how to use wgpu
    // Here we just make a texture to draw into, and a buffer to copy result from that texture
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;

    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Output texture"),
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        mip_level_count: 1,
        sample_count: 1,
        size: wgpu::Extent3d {
            width: 1024,
            height: 1024,
            ..Default::default()
        },
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let staging_output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output staging buffer"),
        size: (output_texture.width() * output_texture.height() * 4) as u64,
        mapped_at_creation: false,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    });

    // Make a new brush preset here
    // Or maybe load the brush preset from JSON with serde_json
    let brush = Brush::Stamp(StampBrush {
        tip: BrushTip::Circle {
            graph: vec![Vec2(0.0, 1.0), Vec2(0.8, 1.0), Vec2(1.0, 0.0)],
        },
        size: Dynamic {
            base: 24.0,
            modifiers: vec![Modifier {
                sensor: Sensor::Pressure,
                graph: vec![],
            }],
        },
        ..Default::default()
    });

    // Make a new brush renderer
    // wgpu::Device and wgpu::Queue are reference counters so it's fine to clone here (maybe)
    let mut renderer = BrushRenderer::<()>::new(device.clone(), queue.clone(), output_texture.format());

    // Make sure to assign preset to the renderer first
    renderer.use_preset(&brush)?;

    // Begin a new stroke with Renderer::new_stroke()
    // use_preset() indlrectly calls new_stroke() anyways so there is no need to call it here
    // renderer.new_stroke();

    // Supply one or more stylus inputs to the renderer
    // Here we just draw from (100; 100) to (1000; 1000) while increasing the logical pressure value from 0 to 1
    // We also pick pure red color for this one
    renderer.next_input(
        &StylusInput {
            timestamp: 0.0,
            position: Vec2(100.0, 100.0),
            pressure: 0.0,
            tilt: Vec2(0.0, 0.0),
            twist: 0.0,
        },
        [1.0, 0.0, 0.0],
    )?;
    renderer.next_input(
        &StylusInput {
            timestamp: 2.0,
            position: Vec2(1000.0, 1000.0),
            pressure: 1.0,
            tilt: Vec2(0.0, 0.0),
            twist: 0.0,
        },
        [1.0, 0.0, 0.0],
    )?;

    // This part is where we encode command buffer and submit it to actually draw something
    let mut encoder = device.create_command_encoder(&Default::default());

    // First we clear our output texture with solid white color
    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &output_texture.create_view(&Default::default()),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
            resolve_target: None,
        })],
        ..Default::default()
    });
    drop(render_pass);

    // Here we begin putting our brush renderer to rendering phase
    renderer.render_begin()?;

    // Then we get the renderer to prepare to render to 1024x1024 texture
    renderer.render_input(
        &(),
        &Rect {
            min: Vec2(0.0, 0.0),
            max: Vec2(1024.0, 1024.0),
        },
        &mut encoder,
    )?;

    // And finally, we draw the stroke to texture view of output texture
    // In reality, applications would call render_finish(), then submit the command buffer and done here.
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    renderer.render_tile(
        &(),
        &identity,
        &output_texture.create_view(&Default::default()),
        &mut encoder,
    )?;

    // Finalize the renderer.
    // After this function is called, render_begin() must not be called until the command buffer is submitted.
    renderer.render_finish()?;

    // Here we just copy content of texture to output buffer so we can read the image data.
    // Typically it would be something like presenting the surface texture.
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output_texture,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
            mip_level: 0,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging_output,
            layout: wgpu::TexelCopyBufferLayout {
                bytes_per_row: Some(output_texture.width() * 4),
                rows_per_image: Some(output_texture.height()),
                ..Default::default()
            },
        },
        wgpu::Extent3d {
            width: output_texture.width(),
            height: output_texture.height(),
            ..Default::default()
        },
    );

    // Submit the command buffer.
    queue.submit([encoder.finish()]);

    // This part is for obtaining the content of texture and output it as PPM image.
    // Use PPM image viewer to open result.ppm
    staging_output.clone().map_async(wgpu::MapMode::Read, .., move |r| {
        r.unwrap();
        let data = staging_output.get_mapped_range(..);
        let mut file = File::create("result.ppm").unwrap();
        let width = output_texture.width();
        let height = output_texture.height();
        file.write(format!("P6 {} {} 255\n", width, height).as_bytes()).unwrap();

        for y in 0..height {
            for x in 0..width {
                let address = ((x + y * width) * 4) as usize;
                file.write(&data[address..address + 3]).unwrap();
            }
        }
    });

    Ok(())
}
