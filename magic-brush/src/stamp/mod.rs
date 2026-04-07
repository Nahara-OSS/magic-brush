//! The module for stamp-based brush.
//!
//! The concept of stamp-based brush is simple: stamp the brush tips along the path with small enough spacing so that
//! the stroke becomes continuous. This technique has been used by popular drawing applications like Krita (with Pixel
//! Brush Engine), Adobe Photoshop or Clip Studio Paint.
//!
//! This stamp-based brush supports **partially drawn stroke** rendering method. Specifically, everytime the renderer
//! read an input event and draw to texture internally, it will only draw the new part of the stroke from such input,
//! instead of drawing entire stroke all over again. This allows the stamp-based brush engine to be efficient in
//! rendering the brush without filling the GPU memory, since the content of the stroke is accumulated on internal
//! "stroke layer".
//!
//! The stroke layer contains 2 different textures:
//!
//! - **Color texture**: Just like the name suggested, this texture is for storing the stroke color only. If the bitmap
//!   is a premultiplied image, the bitmap will be unpremultiplied first, then multiplied by flow value before stamping
//!   to color texture.
//! - **Opacity texture**: This texture contains the opacity data (well it's actually alpha mask) in a form of depth
//!   buffer with depth test function [`wgpu::CompareFunction::GreaterEqual`].

use std::{collections::HashMap, hash::Hash, result::Result};

use rand::RngExt;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::{
    dynamic::{Dynamic, DynamicArray, DynamicContext},
    input::StylusInput,
    renderer::{Error, Renderer},
    utils::{
        graph::Graph,
        lnag::{Rect, Vec2},
    },
};

/// Stamp-based brush preset. May be serialized or deserialized with [`serde`].
#[derive(Serialize, Deserialize)]
pub struct StampBrush {
    /// The brush tip that will be used for stamping to stroke layer.
    pub tip: BrushTip,

    /// The spacing between stamps.
    pub spacing: f32,

    /// The size of the stamp.
    pub size: Dynamic,

    /// Opacity of each individual stamp.
    pub flow: Dynamic,

    /// Opacity of entire stroke.
    pub opacity: Dynamic,

    /// The offset along X and Y axes.
    pub offset: [Dynamic; 2],
}

impl Default for StampBrush {
    fn default() -> Self {
        Self {
            tip: Default::default(),
            spacing: 1.0,
            size: Dynamic::constant(12.0),
            flow: Dynamic::constant(1.0),
            opacity: Dynamic::constant(1.0),
            offset: [Dynamic::constant(0.0), Dynamic::constant(0.0)],
        }
    }
}

/// The shape of brush tip.
#[derive(Serialize, Deserialize)]
pub enum BrushTip {
    /// Use square-shaped brush tip.
    #[serde(rename = "square")]
    Square {
        /// The depth graph for square stamp. The input value goes from the center to the edge of square. The output
        /// value is the grayscale value of the brush tip.
        graph: Vec<Vec2>,
    },

    /// Use circle-shaped brush tip.
    #[serde(rename = "circle")]
    Circle {
        /// The depth graph for circular stamp. The input value goes from the center to the edge of circle. The output
        /// value is the grayscale value of the brush tip.
        graph: Vec<Vec2>,
    },

    /// Use brush tip with custom shape defined in grayscale bitmap data. The size of bitmap data will be scaled so that
    /// it matches with size parameter of the brush. In other words, the size of bitmap defines the resolution of the
    /// brush tip, not the size of it.
    #[serde(rename = "bitmap")]
    Bitmap {
        /// The width of bitmap data.
        width: u32,

        /// The height of bitmap data.
        height: u32,

        /// The grayscale bitmap data for bitmap-based brush tip. The length of data must be equals to
        /// [`BrushTip::Bitmap::width`] * [`BrushTip::Bitmap::height`].
        #[serde(with = "serde_bytes")]
        data: Box<[u8]>,
    },
}

impl Default for BrushTip {
    fn default() -> Self {
        BrushTip::Circle {
            graph: vec![Vec2(0.0, 1.0), Vec2(1.0, 1.0)],
        }
    }
}

pub struct StampBrushRenderer<I: Clone + Eq + Hash> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,

    uniform_buffer: wgpu::Buffer,
    uniform_staging: wgpu::util::StagingBelt,
    uniform_bind_group: wgpu::BindGroup,

    common_bind_group_layout: wgpu::BindGroupLayout,
    custom_bind_group_layout: wgpu::BindGroupLayout,
    circle_pipeline: wgpu::RenderPipeline,
    square_pipeline: wgpu::RenderPipeline,
    custom_pipeline: wgpu::RenderPipeline,
    tip_sampler: wgpu::Sampler,

    copy_bind_group_layout: wgpu::BindGroupLayout,
    copy_pipeline: wgpu::RenderPipeline,
    copy_sampler: wgpu::Sampler,

    tiles: HashMap<I, InternalTileData>,
    brush: Option<ActiveBrush>,
    last_input: Option<StylusInput>,
    stamp_queue: Vec<Stamp>,
    stamp_buffer: Option<wgpu::Buffer>,
    stamp_count: u32,
    jitter: StampBrushDynamicContext,
}

impl<I: Clone + Eq + Hash> Renderer<StampBrush, I> for StampBrushRenderer<I> {
    fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let vertex_shader_module = device.create_shader_module(wgpu::include_wgsl!("./vertex.wgsl"));
        let common_shader_module = device.create_shader_module(wgpu::include_wgsl!("./common.wgsl"));
        let custom_shader_module = device.create_shader_module(wgpu::include_wgsl!("./custom.wgsl"));
        let copy_shader_module = device.create_shader_module(wgpu::include_wgsl!("./copy.wgsl"));

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform buffer"),
            size: size_of::<[f32; 16]>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let uniform_staging = wgpu::util::StagingBelt::new(device.clone(), uniform_buffer.size() * 16);
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let common_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Common brush tip bind group"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D1,
                        multisampled: false,
                    },
                },
            ],
        });
        let custom_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Custom brush tip bind group"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
            ],
        });

        let copy_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Copy bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
            ],
        });

        let common_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Circle/square pipeline layout"),
            immediate_size: 0,
            bind_group_layouts: &[&uniform_bind_group_layout, &common_bind_group_layout],
        });
        let custom_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Custom pipeline layout"),
            immediate_size: 0,
            bind_group_layouts: &[&uniform_bind_group_layout, &custom_bind_group_layout],
        });
        let primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: None,
            strip_index_format: None,
            unclipped_depth: false,
            conservative: false,
            front_face: wgpu::FrontFace::Ccw,
        };
        let vertex_state = wgpu::VertexState {
            module: &vertex_shader_module,
            entry_point: Some("vertexShader"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                step_mode: wgpu::VertexStepMode::Instance,
                array_stride: 36,
                attributes: &[
                    wgpu::VertexAttribute {
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                    },
                    wgpu::VertexAttribute {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 16,
                    },
                    wgpu::VertexAttribute {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32,
                        offset: 24,
                    },
                    wgpu::VertexAttribute {
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32,
                        offset: 28,
                    },
                    wgpu::VertexAttribute {
                        shader_location: 4,
                        format: wgpu::VertexFormat::Float32,
                        offset: 32,
                    },
                ],
            }],
        };
        let depth_stencil_state = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth16Unorm,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::GreaterEqual,
            stencil: Default::default(),
            bias: Default::default(),
        };
        let color_target_state = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::OVER,
                alpha: wgpu::BlendComponent::OVER,
            }),
            write_mask: wgpu::ColorWrites::all(),
        };

        Self {
            uniform_buffer,
            uniform_staging,
            uniform_bind_group,

            circle_pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Circle brush tip"),
                layout: Some(&common_pipeline_layout),
                primitive: primitive_state,
                vertex: vertex_state.clone(),
                fragment: Some(wgpu::FragmentState {
                    module: &common_shader_module,
                    entry_point: Some("circleFragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target_state.clone())],
                }),
                depth_stencil: Some(depth_stencil_state.clone()),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            }),
            square_pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Square brush tip"),
                layout: Some(&common_pipeline_layout),
                primitive: primitive_state,
                vertex: vertex_state.clone(),
                fragment: Some(wgpu::FragmentState {
                    module: &common_shader_module,
                    entry_point: Some("squareFragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target_state.clone())],
                }),
                depth_stencil: Some(depth_stencil_state.clone()),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            }),
            custom_pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Custom brush tip"),
                layout: Some(&custom_pipeline_layout),
                primitive: primitive_state,
                vertex: vertex_state.clone(),
                fragment: Some(wgpu::FragmentState {
                    module: &custom_shader_module,
                    entry_point: Some("customFragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target_state.clone())],
                }),
                depth_stencil: Some(depth_stencil_state.clone()),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            }),
            common_bind_group_layout,
            custom_bind_group_layout,
            tip_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),

            copy_pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Copy"),
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Copy pipeline layout"),
                    immediate_size: 0,
                    bind_group_layouts: &[&uniform_bind_group_layout, &copy_bind_group_layout],
                })),
                primitive: primitive_state,
                vertex: wgpu::VertexState {
                    module: &copy_shader_module,
                    entry_point: Some("vertexShader"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &copy_shader_module,
                    entry_point: Some("fragmentShader"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target_state.clone())],
                }),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            }),
            copy_bind_group_layout,
            copy_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),

            tiles: HashMap::new(),
            brush: None,
            last_input: None,
            stamp_queue: Vec::with_capacity(1024),
            stamp_buffer: None,
            stamp_count: 0,
            jitter: StampBrushDynamicContext::new(),

            device,
            queue,
            format,
        }
    }

    fn use_preset(&mut self, preset: &StampBrush) -> Result<(), Error> {
        self.brush = Some(ActiveBrush::new(self, preset));
        self.new_stroke()
    }

    fn new_stroke(&mut self) -> Result<(), Error> {
        if self.brush.is_none() {
            return Err(Error::NoPreset);
        }

        self.last_input = None;
        self.stamp_queue.clear();
        self.jitter.roll_stroke();
        Ok(())
    }

    fn next_input(&mut self, input: &StylusInput, color: [f32; 3]) -> Result<Rect, Error> {
        let Some(brush) = &self.brush else {
            return Err(Error::NoPreset);
        };

        if let Some(last_input) = &self.last_input {
            let mut last_input = last_input.clone();
            let mut bounds: Option<Rect> = None;

            while (input.position - last_input.position).len() >= brush.spacing {
                let vector = input.position - last_input.position;
                let lerp_fraction = brush.spacing / vector.len();
                let next_input = StylusInput::lerp(&last_input, input, lerp_fraction);
                let stamp = Stamp {
                    color: [color[0], color[1], color[2], 1.0],
                    world_coords: (next_input.position
                        + brush
                            .offset
                            .derive(&mut self.jitter, Some(&last_input), &next_input)
                            .into())
                    .into(),
                    size: brush.size.derive(&mut self.jitter, Some(&last_input), &next_input),
                    flow: brush.flow.derive(&mut self.jitter, Some(&last_input), &next_input),
                    opacity: brush.opacity.derive(&mut self.jitter, Some(&last_input), &next_input),
                };
                match &mut bounds {
                    Some(bounds) => bounds.expand_mut(stamp.rect()),
                    None => bounds = Some(stamp.rect()),
                };
                self.stamp_queue.push(stamp);
                last_input = next_input;
            }

            self.last_input = Some(last_input);
            Ok(bounds.unwrap_or(Default::default()))
        } else {
            let stamp = Stamp {
                color: [color[0], color[1], color[2], 1.0],
                world_coords: (input.position + brush.offset.derive(&mut self.jitter, None, input).into()).into(),
                size: brush.size.derive(&mut self.jitter, None, input),
                flow: brush.flow.derive(&mut self.jitter, None, input),
                opacity: brush.opacity.derive(&mut self.jitter, None, input),
            };

            let rect = stamp.rect();
            self.last_input = Some(input.clone());
            self.stamp_queue.push(stamp);
            Ok(rect)
        }
    }

    fn render_begin(&mut self) -> Result<(), Error> {
        self.uniform_staging.recall();

        // We use the emptiness of stamp queue to determine whether we need to write instance buffer
        if !self.stamp_queue.is_empty() {
            let required_size = size_of::<Stamp>() * self.stamp_queue.len();
            let current_size = self.stamp_buffer.as_ref().map(|b| b.size()).unwrap_or(0) as usize;

            if required_size > current_size {
                let new_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Instance buffer"),
                    size: (required_size * 2) as u64,
                    mapped_at_creation: true,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
                });

                let mut mapped_range = new_buffer.get_mapped_range_mut(..);
                let casted_mapped_range: &mut [Stamp] = bytemuck::cast_slice_mut(&mut mapped_range);
                casted_mapped_range[0..self.stamp_queue.len()].copy_from_slice(&self.stamp_queue);
                drop(mapped_range);
                new_buffer.unmap();
                self.stamp_buffer = Some(new_buffer);
            } else {
                // unwrap():
                // - current_size must not be zero
                // - !stamp_queue.is_empty() so required_size must not be zero
                let stamp_buffer = self.stamp_buffer.as_ref().unwrap();
                let write_data = bytemuck::cast_slice(&self.stamp_queue);
                self.queue.write_buffer(stamp_buffer, 0, write_data);
            }
        }

        self.stamp_count = self.stamp_queue.len() as u32;
        self.stamp_queue.clear();
        Ok(())
    }

    fn render_input(&mut self, id: &I, rect: &Rect, encoder: &mut wgpu::CommandEncoder) -> Result<(), Error> {
        let Some(brush) = &self.brush else {
            return Err(Error::NoPreset);
        };

        if !self.tiles.contains_key(id) {
            self.tiles.insert(id.clone(), InternalTileData::new(self, rect));
        }

        {
            let mut uniform_data = self.uniform_staging.write_buffer(
                encoder,
                &self.uniform_buffer,
                0,
                (size_of::<[f32; 16]>() as u64).try_into()?,
            );
            let uniform_data: &mut [f32] = bytemuck::cast_slice_mut(&mut uniform_data);
            bytemuck::fill_zeroes(uniform_data);
            uniform_data[0] = 2.0 / rect.size().0;
            uniform_data[5] = -2.0 / rect.size().1;
            uniform_data[10] = 1.0;
            uniform_data[12] = -1.0;
            uniform_data[13] = 1.0;
            uniform_data[15] = 1.0;
        }

        let tile = &self.tiles[id];
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &tile.color_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &tile.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        render_pass.set_pipeline(&brush.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &brush.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.stamp_buffer.as_ref().expect("buffer").slice(..));
        render_pass.draw(0..4, 0..self.stamp_count);
        drop(render_pass);
        Ok(())
    }

    fn render_tile(
        &mut self,
        id: &I,
        transform: &[f32; 16],
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), Error> {
        let Some(tile) = self.tiles.get(id) else {
            return Err(Error::NoTile);
        };

        {
            let mut uniform_data = self.uniform_staging.write_buffer(
                encoder,
                &self.uniform_buffer,
                0,
                (size_of::<[f32; 16]>() as u64).try_into()?,
            );
            let uniform_data: &mut [f32] = bytemuck::cast_slice_mut(&mut uniform_data);
            uniform_data[0..16].copy_from_slice(transform);
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(&self.copy_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &tile.copy_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        drop(render_pass);
        Ok(())
    }

    fn render_finish(&mut self) -> Result<(), Error> {
        self.uniform_staging.finish();
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Stamp {
    color: [f32; 4],
    world_coords: [f32; 2],
    size: f32,
    flow: f32,
    opacity: f32,
}

impl Stamp {
    fn rect(&self) -> Rect {
        let min = Vec2(
            self.world_coords[0] - self.size / 2.0,
            self.world_coords[1] - self.size / 2.0,
        );

        let max = Vec2(
            self.world_coords[0] + self.size / 2.0,
            self.world_coords[1] + self.size / 2.0,
        );

        Rect { min, max }
    }
}

struct ActiveBrush {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    spacing: f32,
    size: Dynamic,
    flow: Dynamic,
    opacity: Dynamic,
    offset: [Dynamic; 2],
}

impl ActiveBrush {
    fn new<I: Clone + Eq + Hash>(renderer: &StampBrushRenderer<I>, brush: &StampBrush) -> ActiveBrush {
        let (pipeline, bind_group) = match &brush.tip {
            BrushTip::Square { graph } | BrushTip::Circle { graph } => {
                let texture = renderer.device.create_texture_with_data(
                    &renderer.queue,
                    &wgpu::TextureDescriptor {
                        label: Some("Tip depth map"),
                        dimension: wgpu::TextureDimension::D1,
                        format: wgpu::TextureFormat::R8Unorm,
                        mip_level_count: 1,
                        sample_count: 1,
                        size: wgpu::Extent3d {
                            width: 256,
                            ..Default::default()
                        },
                        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &graph.make_1d_data::<u8, 256>(),
                );
                let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Square brush bind group"),
                    layout: &renderer.common_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&renderer.tip_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                        },
                    ],
                });
                let pipeline = match &brush.tip {
                    BrushTip::Square { .. } => renderer.square_pipeline.clone(),
                    BrushTip::Circle { .. } => renderer.circle_pipeline.clone(),
                    _ => unreachable!(),
                };
                (pipeline, bind_group)
            }
            BrushTip::Bitmap { width, height, data } => {
                let texture = renderer.device.create_texture_with_data(
                    &renderer.queue,
                    &wgpu::TextureDescriptor {
                        label: Some("Tip depth map"),
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::R8Unorm,
                        mip_level_count: 1,
                        sample_count: 1,
                        size: wgpu::Extent3d {
                            width: *width,
                            height: *height,
                            ..Default::default()
                        },
                        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &data,
                );
                let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Custom brush tip"),
                    layout: &renderer.custom_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&renderer.tip_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                        },
                    ],
                });
                (renderer.custom_pipeline.clone(), bind_group)
            }
        };

        Self {
            pipeline,
            bind_group,
            spacing: brush.spacing,
            size: brush.size.clone(),
            flow: brush.flow.clone(),
            opacity: brush.opacity.clone(),
            offset: brush.offset.clone(),
        }
    }
}

struct InternalTileData {
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    copy_bind_group: wgpu::BindGroup,
}

impl InternalTileData {
    fn new<I: Clone + Eq + Hash>(renderer: &StampBrushRenderer<I>, rect: &Rect) -> InternalTileData {
        let extent = wgpu::Extent3d {
            width: rect.size().0 as u32,
            height: rect.size().1 as u32,
            ..Default::default()
        };
        let color_texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Color texture"),
            dimension: wgpu::TextureDimension::D2,
            format: renderer.format,
            mip_level_count: 1,
            sample_count: 1,
            size: extent,
            usage: wgpu::TextureUsages::empty()
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth16Unorm,
            mip_level_count: 1,
            sample_count: 1,
            size: extent,
            usage: wgpu::TextureUsages::empty()
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&Default::default());
        let depth_view = depth_texture.create_view(&Default::default());
        Self {
            copy_bind_group: renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Stroke layer copy bind group"),
                layout: &renderer.copy_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&renderer.copy_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&depth_view),
                    },
                ],
            }),
            color_view,
            depth_view,
        }
    }
}

struct StampBrushDynamicContext {
    stroke_jitter: f32,
    generator: rand::rngs::ThreadRng,
}

impl StampBrushDynamicContext {
    fn new() -> StampBrushDynamicContext {
        Self {
            stroke_jitter: 0.0,
            generator: rand::rng(),
        }
    }

    fn roll_stroke(&mut self) {
        self.stroke_jitter = self.generator.random();
    }
}

impl DynamicContext for StampBrushDynamicContext {
    fn jitter_stroke(&self) -> f32 {
        todo!()
    }

    fn jitter_dab(&mut self) -> f32 {
        self.generator.random()
    }
}
