use magic_brush::{
    dynamic::{Dynamic, Modifier, Sensor},
    input::StylusInput,
    renderer::{self, Rect, Renderer, RendererFactory},
    stamp::{self, BrushTip},
};
use wasm_bindgen::{JsError, JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
pub struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    html: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    brush: stamp::Brush,
    color: [f32; 4],
    renderer: Box<dyn renderer::Renderer<()>>,
    copy_bind_group_layout: wgpu::BindGroupLayout,
    copy_pipeline: wgpu::RenderPipeline,
    canvas: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)>,
    pen_id: Option<i32>,
}

#[wasm_bindgen]
impl App {
    #[wasm_bindgen]
    pub async fn create(canvas: web_sys::HtmlCanvasElement) -> Result<App, JsError> {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let brush = stamp::Brush {
            tip: BrushTip::Circle {
                graph: vec![[0.0, 1.0], [0.2, 1.0], [1.0, 0.0]],
            },
            size: Dynamic {
                base: 6.0,
                modifiers: vec![Modifier {
                    sensor: Sensor::Pressure,
                    graph: vec![[0.0, 0.8], [1.0, 1.0]],
                }],
            },
            flow: Dynamic {
                base: 1.0,
                modifiers: vec![Modifier {
                    sensor: Sensor::Pressure,
                    graph: vec![[0.0, 0.1], [1.0, 0.6]],
                }],
            },
            opacity: Dynamic {
                base: 1.0,
                modifiers: vec![Modifier {
                    sensor: Sensor::Pressure,
                    graph: vec![],
                }],
            },
            offset: [
                Dynamic {
                    base: 1.0,
                    modifiers: vec![
                        Modifier {
                            sensor: Sensor::JitterDab,
                            graph: vec![[0.0, -1.0], [1.0, 1.0]],
                        },
                        Modifier {
                            sensor: Sensor::Pressure,
                            graph: vec![[0.0, 1.0], [1.0, 0.0]],
                        },
                    ],
                },
                Dynamic {
                    base: 1.0,
                    modifiers: vec![
                        Modifier {
                            sensor: Sensor::JitterDab,
                            graph: vec![[0.0, -1.0], [1.0, 1.0]],
                        },
                        Modifier {
                            sensor: Sensor::Pressure,
                            graph: vec![[0.0, 1.0], [1.0, 0.0]],
                        },
                    ],
                },
            ],
            ..Default::default()
        };
        let mut renderer = Box::new(stamp::Renderer::<()>::create(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
        ));
        renderer.try_change_preset(&brush);

        #[cfg(target_arch = "wasm32")]
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))?;
        #[cfg(not(target_arch = "wasm32"))]
        let surface = panic!("magic-brush-demo must be packaged using wasm-pack");

        let copy_shader_module = device.create_shader_module(wgpu::include_wgsl!("./copy.wgsl"));
        let copy_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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

        Ok(App {
            surface,
            brush,
            color: [0.0, 0.0, 0.0, 1.0],
            renderer,
            canvas: None,
            pen_id: None,
            copy_pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                cache: None,
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    immediate_size: 0,
                    bind_group_layouts: &[&copy_bind_group_layout],
                })),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                vertex: wgpu::VertexState {
                    module: &copy_shader_module,
                    entry_point: Some("vertexShader"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &copy_shader_module,
                    entry_point: Some("fragmentShader"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        write_mask: wgpu::ColorWrites::all(),
                        blend: None,
                    })],
                    compilation_options: Default::default(),
                }),
                multisample: Default::default(),
                depth_stencil: None,
                multiview_mask: None,
            }),
            copy_bind_group_layout,
            device,
            queue,
            html: canvas,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn preset(&self) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(&self.brush)?)
    }

    #[wasm_bindgen(setter)]
    pub fn set_preset(&mut self, preset: JsValue) -> Result<(), JsError> {
        self.brush = serde_wasm_bindgen::from_value(preset)?;
        Ok(())
    }

    #[wasm_bindgen(getter)]
    pub fn color(&self) -> Vec<JsValue> {
        self.color.map(|v| JsValue::from_f64(v as f64)).into()
    }

    #[wasm_bindgen(setter)]
    pub fn set_color(&mut self, value: Vec<JsValue>) -> Result<(), JsError> {
        self.color = value
            .into_iter()
            .map(|v| v.as_f64().map(|v| v as f32))
            .collect::<Option<Vec<f32>>>()
            .ok_or(JsError::new("The array contains non-numerical value"))?
            .try_into()
            .map_err(|v: Vec<f32>| JsError::new(format!("Length mismatch: {} != 4", v.len()).as_str()))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn configure(&mut self, width: u32, height: u32) -> Result<(), JsError> {
        if width * height == 0 {
            return Err(JsError::new("Either width or height is zero"));
        }

        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                width,
                height,
                format: wgpu::TextureFormat::Rgba8Unorm,
                view_formats: vec![],
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                present_mode: wgpu::PresentMode::AutoNoVsync,
                desired_maximum_frame_latency: 2,
            },
        );

        let canvas = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Canvas texture"),
            format: wgpu::TextureFormat::Rgba8Unorm,
            dimension: wgpu::TextureDimension::D2,
            size: wgpu::Extent3d {
                width,
                height,
                ..Default::default()
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
        });
        let canvas_view = canvas.create_view(&Default::default());

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.copy_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.device.create_sampler(&Default::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&canvas_view),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &canvas_view,
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
        self.queue.submit([encoder.finish()]);
        self.canvas = Some((canvas, canvas_view, bind_group));
        Ok(())
    }

    #[wasm_bindgen(js_name = "penDown")]
    pub fn pen_down(&mut self, event: web_sys::PointerEvent) -> Result<(), JsError> {
        if let None = self.canvas {
            return Err(JsError::new("Canvas must be configured first"));
        }

        let None = self.pen_id else {
            return Ok(());
        };

        self.pen_id = Some(event.pointer_id());
        self.renderer.begin_new_stroke();
        self.pen_move(event)
    }

    #[wasm_bindgen(js_name = "penMove")]
    pub fn pen_move(&mut self, event: web_sys::PointerEvent) -> Result<(), JsError> {
        let Some((canvas, _, copy_bind_group)) = &self.canvas else {
            return Err(JsError::new("Canvas must be configured first"));
        };

        let Some(pen_id) = self.pen_id else {
            return Ok(());
        };

        if pen_id != event.pointer_id() {
            return Ok(());
        }

        let bounds = self.html.get_bounding_client_rect();
        self.renderer.prepare_input(
            &StylusInput {
                timestamp: event.time_stamp() as f32,
                position: [
                    event.client_x() as f32 - bounds.x() as f32,
                    event.client_y() as f32 - bounds.y() as f32,
                ],
                pressure: event.pressure(),
                tilt: [event.tilt_x() as f32, event.tilt_y() as f32],
                twist: event.twist() as f32,
            },
            &self.color,
        )?;

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let surface_texture = self.surface.get_current_texture()?;

        self.renderer.prepare_tile(
            &(),
            &Rect {
                x: 0,
                y: 0,
                width: canvas.width(),
                height: canvas.height(),
            },
            &mut encoder,
        )?;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture.texture.create_view(&Default::default()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(&self.copy_pipeline);
        render_pass.set_bind_group(0, copy_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        self.renderer.render_tile(&(), &mut render_pass)?;
        drop(render_pass);

        self.queue.submit([encoder.finish()]);
        surface_texture.present();
        Ok(())
    }

    #[wasm_bindgen(js_name = "penUp")]
    pub fn pen_up(&mut self, event: web_sys::PointerEvent) -> Result<(), JsError> {
        let Some((canvas, canvas_view, copy_bind_group)) = &self.canvas else {
            return Err(JsError::new("Canvas must be configured first"));
        };

        let Some(pen_id) = self.pen_id else {
            return Ok(());
        };

        if pen_id != event.pointer_id() {
            return Ok(());
        }

        let bounds = self.html.get_bounding_client_rect();
        self.renderer.prepare_input(
            &StylusInput {
                timestamp: event.time_stamp() as f32,
                position: [
                    event.client_x() as f32 - bounds.x() as f32,
                    event.client_y() as f32 - bounds.y() as f32,
                ],
                pressure: event.pressure(),
                tilt: [event.tilt_x() as f32, event.tilt_y() as f32],
                twist: event.twist() as f32,
            },
            &self.color,
        )?;

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let surface_texture = self.surface.get_current_texture()?;

        self.renderer.prepare_tile(
            &(),
            &Rect {
                x: 0,
                y: 0,
                width: canvas.width(),
                height: canvas.height(),
            },
            &mut encoder,
        )?;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: canvas_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            ..Default::default()
        });
        self.renderer.render_tile(&(), &mut render_pass)?;
        drop(render_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture.texture.create_view(&Default::default()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(&self.copy_pipeline);
        render_pass.set_bind_group(0, copy_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        drop(render_pass);

        self.queue.submit([encoder.finish()]);
        surface_texture.present();
        self.pen_id = None;
        Ok(())
    }
}
