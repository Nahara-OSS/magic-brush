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

    let mut renderer = BrushRenderer::<()>::new(device.clone(), queue.clone(), output_texture.format());
    renderer.use_preset(&brush)?;
    renderer.next_input(&StylusInput {
        timestamp: 0.0,
        position: Vec2(100.0, 100.0),
        pressure: 0.0,
        tilt: Vec2(0.0, 0.0),
        twist: 0.0,
    })?;
    renderer.next_input(&StylusInput {
        timestamp: 2.0,
        position: Vec2(1000.0, 1000.0),
        pressure: 1.0,
        tilt: Vec2(0.0, 0.0),
        twist: 0.0,
    })?;

    let mut encoder = device.create_command_encoder(&Default::default());
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
    renderer.render_begin()?;
    renderer.render_input(
        &(),
        &Rect {
            min: Vec2(0.0, 0.0),
            max: Vec2(1024.0, 1024.0),
        },
        &mut encoder,
    )?;
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    renderer.render_tile(
        &(),
        &identity,
        &output_texture.create_view(&Default::default()),
        &mut encoder,
    )?;
    renderer.render_finish()?;
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
    queue.submit([encoder.finish()]);
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
