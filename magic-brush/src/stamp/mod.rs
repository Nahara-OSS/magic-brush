use std::{collections::HashMap, hash::Hash, mem, num::NonZero, result::Result};

use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{
    dynamic::{Dynamic, DynamicArray, DynamicContext},
    graph::Graph,
    input::StylusInput,
    renderer::{self, Rect, RendererError, RendererFactory},
    utils::Vector2Like,
};

/// Stamp-based brush preset. May be serialized or deserialized with [`serde`].
#[derive(Serialize, Deserialize)]
pub struct Brush {
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

impl Default for Brush {
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
        graph: Vec<[f32; 2]>,
    },

    /// Use circle-shaped brush tip.
    #[serde(rename = "circle")]
    Circle {
        /// The depth graph for circular stamp. The input value goes from the center to the edge of circle. The output
        /// value is the grayscale value of the brush tip.
        graph: Vec<[f32; 2]>,
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
            graph: vec![[0.0, 1.0], [1.0, 1.0]],
        }
    }
}

/// Renderer for stamp-based brush presets.
pub struct Renderer<I: Eq + Hash> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,

    circle_pipeline: wgpu::RenderPipeline,
    square_pipeline: wgpu::RenderPipeline,
    custom_pipeline: wgpu::RenderPipeline,
    copy_pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    common_bind_group_layout: wgpu::BindGroupLayout,
    custom_bind_group_layout: wgpu::BindGroupLayout,
    copy_bind_group_layout: wgpu::BindGroupLayout,
    brush_tip_sampler: wgpu::Sampler,
    uniform_pool: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    uniform_align: u32,
    uniform_index: u32,
    instance_buffer: wgpu::Buffer,
    copy_sampler: wgpu::Sampler,

    active_brush: Option<ActiveBrush>,
    last_input: Option<(StylusInput, [f32; 4])>,
    jitter: Jitter,
    stamp_count: u32,
    internal_tiles: HashMap<I, InternalTileData>,
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

struct ActiveBrush {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    spacing: f32,
    size: Dynamic,
    flow: Dynamic,
    opacity: Dynamic,
    offset: [Dynamic; 2],
}

struct InternalTileData {
    color_texture_view: wgpu::TextureView,
    opacity_texture_view: wgpu::TextureView,
    copy_bind_group: wgpu::BindGroup,
}

#[derive(Default)]
struct Jitter {
    rng: rand::rngs::ThreadRng,
    stroke: f32,
}

impl DynamicContext for Jitter {
    fn jitter_stroke(&self) -> f32 {
        self.stroke
    }

    fn jitter_dab(&mut self) -> f32 {
        self.rng.random()
    }
}

impl<I: Eq + Hash> RendererFactory for Renderer<I> {
    fn create<T: Clone + Eq + Hash>(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> impl renderer::Renderer<T> {
        let min_align = device.limits().min_uniform_buffer_offset_alignment;
        let vertex_shader_module = device.create_shader_module(wgpu::include_wgsl!("./vertex.wgsl"));
        let common_shader_module = device.create_shader_module(wgpu::include_wgsl!("./common.wgsl"));
        let custom_shader_module = device.create_shader_module(wgpu::include_wgsl!("./custom.wgsl"));
        let copy_shader_module = device.create_shader_module(wgpu::include_wgsl!("./copy.wgsl"));
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform bind group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true, // Dynamic offset for each tile
                    min_binding_size: None,
                },
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
            label: Some("Copy bind group"),
            entries: &[
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                },
                // Color texture
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
                // Opacity texture
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
        Renderer::<T> {
            format,
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
            copy_pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Copy"),
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Copy pipeline layout"),
                    immediate_size: 0,
                    bind_group_layouts: &[&copy_bind_group_layout],
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
            uniform_bind_group_layout,
            common_bind_group_layout,
            custom_bind_group_layout,
            copy_bind_group_layout,
            brush_tip_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            uniform_pool: Vec::new(),
            uniform_align: if mem::size_of::<[f32; 16]>().is_multiple_of(min_align as usize) {
                mem::size_of::<[f32; 16]>() as u32
            } else {
                (mem::size_of::<[f32; 16]>() as u32 / min_align + 1) * min_align
            },
            uniform_index: 0,
            instance_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Stamps/instance buffer"),
                mapped_at_creation: false,
                size: (mem::size_of::<Stamp>() * 1024) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            }),
            copy_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            active_brush: None,
            last_input: None,
            jitter: Default::default(),
            stamp_count: 0,
            internal_tiles: HashMap::new(),
            device,
            queue,
        }
    }
}

impl<I: Clone + Eq + Hash> renderer::Renderer<I> for Renderer<I> {
    fn try_change_preset(&mut self, preset: &dyn std::any::Any) -> bool {
        let Some(preset) = preset.downcast_ref::<Brush>() else {
            return false;
        };

        let (pipeline, bind_group) = match &preset.tip {
            BrushTip::Circle { graph } => {
                let texture = self.make_texture_from_graph(graph);
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Circle brush tip"),
                    layout: &self.common_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&self.brush_tip_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                        },
                    ],
                });
                (self.circle_pipeline.clone(), bind_group)
            }
            BrushTip::Square { graph } => {
                let texture = self.make_texture_from_graph(graph);
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Square brush tip"),
                    layout: &self.common_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&self.brush_tip_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                        },
                    ],
                });
                (self.square_pipeline.clone(), bind_group)
            }
            BrushTip::Bitmap { width, height, data } => {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Custom brush tip"),
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    size: wgpu::Extent3d {
                        width: *width,
                        height: *height,
                        ..Default::default()
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    view_formats: &[],
                });
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Custom brush tip"),
                    layout: &self.custom_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&self.brush_tip_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                        },
                    ],
                });
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        aspect: wgpu::TextureAspect::All,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    data,
                    wgpu::TexelCopyBufferLayout {
                        bytes_per_row: Some(*width),
                        rows_per_image: Some(*height),
                        offset: 0,
                    },
                    wgpu::Extent3d {
                        width: *width,
                        height: *height,
                        ..Default::default()
                    },
                );
                (self.custom_pipeline.clone(), bind_group)
            }
        };

        self.active_brush = Some(ActiveBrush {
            pipeline,
            bind_group,
            spacing: preset.spacing,
            size: preset.size.clone(),
            flow: preset.flow.clone(),
            opacity: preset.opacity.clone(),
            offset: preset.offset.clone(),
        });

        self.begin_new_stroke();
        true
    }

    fn begin_new_stroke(&mut self) {
        self.uniform_index = 0;
        self.last_input = None;
        self.jitter.stroke = self.jitter.rng.random();
        self.stamp_count = 0;
        self.internal_tiles.clear();
    }

    fn prepare_input(&mut self, input: &StylusInput, color: &[f32; 4]) -> Result<Rect, RendererError> {
        let mut stamps = Vec::<Stamp>::new();
        let Some(brush) = &self.active_brush else {
            return Err(RendererError::NoPreset);
        };

        let area = if let Some((last_input, _)) = &self.last_input {
            let mut last_input = last_input.clone();

            while input.position.vec2_sub(&last_input.position).vec2_len() >= brush.spacing {
                let vector = input.position.vec2_sub(&last_input.position);
                let lerp_fraction = brush.spacing / vector.vec2_len();
                let next_input = StylusInput::lerp(&last_input, input, lerp_fraction);

                stamps.push(Stamp {
                    color: *color,
                    world_coords: last_input.position.vec2_add(&brush.offset.derive(
                        &mut self.jitter,
                        Some(&last_input),
                        &next_input,
                    )),
                    size: brush.size.derive(&mut self.jitter, Some(&last_input), &next_input),
                    flow: brush.flow.derive(&mut self.jitter, Some(&last_input), &next_input),
                    opacity: brush.opacity.derive(&mut self.jitter, Some(&last_input), &next_input),
                });

                last_input = next_input;
            }

            self.last_input = Some((last_input, *color));

            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }
        } else {
            self.last_input = Some((input.clone(), *color));

            stamps.push(Stamp {
                color: *color,
                world_coords: input
                    .position
                    .vec2_add(&brush.offset.derive(&mut self.jitter, None, input)),
                size: brush.size.derive(&mut self.jitter, None, input),
                flow: brush.flow.derive(&mut self.jitter, None, input),
                opacity: brush.opacity.derive(&mut self.jitter, None, input),
            });

            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }
        };

        if (mem::size_of::<Stamp>() * stamps.len()) as u64 > self.instance_buffer.size() {
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Stamps/instance buffer (grown)"),
                mapped_at_creation: false,
                size: (mem::size_of::<Stamp>() * stamps.len() * 2) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            });
        }

        let stamps_as_u8 = bytemuck::cast_slice(&stamps);
        self.queue.write_buffer(&self.instance_buffer, 0, stamps_as_u8);
        self.stamp_count = stamps.len() as u32;
        self.uniform_index = 0;
        Ok(area)
    }

    fn prepare_tile(
        &mut self,
        tile_id: &I,
        tile_rect: &Rect,
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Result<(), RendererError> {
        let Some(brush) = &self.active_brush else {
            return Err(RendererError::NoPreset);
        };

        if !self.internal_tiles.contains_key(tile_id) {
            let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Internal stroke color layer"),
                format: self.format,
                dimension: wgpu::TextureDimension::D2,
                size: wgpu::Extent3d {
                    width: tile_rect.width,
                    height: tile_rect.height,
                    ..Default::default()
                },
                view_formats: &[],
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                sample_count: 1,
                mip_level_count: 1,
            });
            let opacity_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Internal stroke opacity layer"),
                format: wgpu::TextureFormat::Depth16Unorm,
                dimension: wgpu::TextureDimension::D2,
                size: wgpu::Extent3d {
                    width: tile_rect.width,
                    height: tile_rect.height,
                    ..Default::default()
                },
                view_formats: &[],
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                sample_count: 1,
                mip_level_count: 1,
            });
            let color_texture_view = color_texture.create_view(&Default::default());
            let opacity_texture_view = opacity_texture.create_view(&Default::default());
            let copy_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Internal stroke layer"),
                layout: &self.copy_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.copy_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&color_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&opacity_texture_view),
                    },
                ],
            });
            let tile_data = InternalTileData {
                color_texture_view,
                opacity_texture_view,
                copy_bind_group,
            };
            self.internal_tiles.insert(tile_id.clone(), tile_data);
        }

        let world_to_clip: [f32; 16] = [
            2.0 / tile_rect.width as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / tile_rect.height as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            -1.0,
            1.0,
            0.0,
            1.0,
        ];

        const UNIFORMS_PER_BUFFER: u32 = 16;
        let uniform_buffer_index = self.uniform_index / UNIFORMS_PER_BUFFER;
        let uniform_data = bytemuck::bytes_of(&world_to_clip);
        let uniform_offset = ((self.uniform_index % UNIFORMS_PER_BUFFER) * self.uniform_align) as u64;

        while uniform_buffer_index as usize >= self.uniform_pool.len() {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Uniform buffer"),
                mapped_at_creation: false,
                size: (self.uniform_align * UNIFORMS_PER_BUFFER) as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Uniform"),
                layout: &self.uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: Some(NonZero::new(mem::size_of::<[f32; 16]>() as u64).unwrap()),
                    }),
                }],
            });
            self.uniform_pool.push((buffer, bind_group));
        }

        let (uniform_buffer, uniform_bind_group) = &self.uniform_pool[uniform_buffer_index as usize];
        self.queue.write_buffer(uniform_buffer, uniform_offset, uniform_data);
        let tile_data = &self.internal_tiles[tile_id];
        let render_pass_desc = wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &tile_data.color_texture_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &tile_data.opacity_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        };

        let mut managed_encoder = match &encoder {
            Some(_) => None,
            None => Some(self.device.create_command_encoder(&Default::default())),
        };

        let mut render_pass = match &mut managed_encoder {
            Some(encoder) => encoder.begin_render_pass(&render_pass_desc),
            None => encoder.unwrap().begin_render_pass(&render_pass_desc),
        };

        render_pass.set_pipeline(&brush.pipeline);
        render_pass.set_bind_group(0, uniform_bind_group, &[uniform_offset as wgpu::DynamicOffset]);
        render_pass.set_bind_group(1, Some(&brush.bind_group), &[]);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(0..)); // TODO: Only draw visible stamps
        render_pass.draw(0..4, 0..self.stamp_count);
        drop(render_pass);

        if let Some(encoder) = managed_encoder {
            self.queue.submit([encoder.finish()]);
        }

        Ok(())
    }

    fn render_tile(&self, tile_id: &I, render_pass: &mut wgpu::RenderPass) -> Result<(), RendererError> {
        let Some(tile_data) = self.internal_tiles.get(tile_id) else {
            return Err(RendererError::IdNotProcessed);
        };

        render_pass.set_pipeline(&self.copy_pipeline);
        render_pass.set_bind_group(0, &tile_data.copy_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        Ok(())
    }
}

impl<I: Eq + Hash> Renderer<I> {
    fn make_texture_from_graph(&self, g: &[[f32; 2]]) -> wgpu::Texture {
        let data: [u8; 256] = g.make_1d_data();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Graph"),
            format: wgpu::TextureFormat::R8Unorm,
            dimension: wgpu::TextureDimension::D1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            size: wgpu::Extent3d {
                width: data.len() as u32,
                ..Default::default()
            },
            view_formats: &[],
            mip_level_count: 1,
            sample_count: 1,
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                aspect: wgpu::TextureAspect::All,
                origin: wgpu::Origin3d::ZERO,
                mip_level: 0,
            },
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(data.len() as u32),
                ..Default::default()
            },
            wgpu::Extent3d {
                width: data.len() as u32,
                ..Default::default()
            },
        );
        texture
    }
}
