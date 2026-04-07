use std::{cell::RefCell, ops::DerefMut, rc::Rc, vec};

use magic_brush::stamp::{self};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

#[wasm_bindgen]
#[derive(Clone)]
pub struct Runtime(Rc<RuntimeInner>);

struct RuntimeInner {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    copy_bind_group: wgpu::BindGroupLayout,
    copy_pipeline: wgpu::RenderPipeline,
}

#[wasm_bindgen]
impl Runtime {
    #[wasm_bindgen]
    pub async fn create() -> Result<Runtime, JsError> {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;

        let copy_shader_module = device.create_shader_module(wgpu::include_wgsl!("./copy.wgsl"));
        let copy_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Copy bind group layout"),
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
        let copy_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Copy pipeline"),
            cache: None,
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&copy_bind_group],
                ..Default::default()
            })),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            vertex: wgpu::VertexState {
                module: &copy_shader_module,
                entry_point: Some("vertexShader"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &copy_shader_module,
                entry_point: Some("fragmentShader"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    write_mask: wgpu::ColorWrites::all(),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::OVER,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                })],
            }),
            depth_stencil: None,
            multiview_mask: None,
            multisample: Default::default(),
        });

        Ok(Runtime(Rc::new(RuntimeInner {
            copy_bind_group,
            copy_pipeline,
            instance,
            device,
            queue,
        })))
    }

    #[wasm_bindgen(js_name = "createDocument")]
    pub fn create_document(&self, name: String, width: u32, height: u32) -> Result<Document, JsError> {
        Ok(Document(Rc::new(DocumentInner {
            runtime: self.clone(),
            data: RefCell::new(DocumentData {
                name,
                size: [width, height],
                layers: vec![],
            }),
        })))
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    #[wasm_bindgen(js_name = "createHtmlSurface")]
    pub fn create_html_surface(&self, html: web_sys::HtmlCanvasElement) -> Result<Surface, JsError> {
        #[cfg(target_arch = "wasm32")]
        let surface = self
            .0
            .instance
            .create_surface(wgpu::SurfaceTarget::Canvas(html.clone()))?;
        #[cfg(not(target_arch = "wasm32"))]
        let surface: wgpu::Surface<'static> = panic!("Only works on browser");

        Ok(Surface {
            runtime: self.clone(),
            kind: SurfaceKind::Html,
            html: Some(html),
            offscreen: None,
            inner: surface,
        })
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    #[wasm_bindgen(js_name = "createOffscreenSurface")]
    pub fn create_offscreen_surface(&self, offscreen: web_sys::OffscreenCanvas) -> Result<Surface, JsError> {
        #[cfg(target_arch = "wasm32")]
        let surface = self
            .0
            .instance
            .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(offscreen.clone()))?;
        #[cfg(not(target_arch = "wasm32"))]
        let surface: wgpu::Surface<'static> = panic!("Only works on browser");

        Ok(Surface {
            runtime: self.clone(),
            kind: SurfaceKind::Offscreen,
            html: None,
            offscreen: Some(offscreen),
            inner: surface,
        })
    }
}

#[wasm_bindgen]
pub struct Surface {
    runtime: Runtime,
    kind: SurfaceKind,
    html: Option<web_sys::HtmlCanvasElement>,
    offscreen: Option<web_sys::OffscreenCanvas>,
    inner: wgpu::Surface<'static>,
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum SurfaceKind {
    Html,
    Offscreen,
}

#[wasm_bindgen]
impl Surface {
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> SurfaceKind {
        self.kind
    }

    /// Reconfigure the surface every time the canvas is resized.
    #[wasm_bindgen]
    pub fn configure(&self) {
        let (width, height) = match self.kind {
            SurfaceKind::Html => self.html.as_ref().map(|c| (c.width(), c.height())).unwrap(),
            SurfaceKind::Offscreen => self.offscreen.as_ref().map(|c| (c.width(), c.height())).unwrap(),
        };

        self.inner.configure(
            &self.runtime.0.device,
            &wgpu::SurfaceConfiguration {
                format: wgpu::TextureFormat::Rgba8Unorm,
                width,
                height,
                alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: vec![],
                present_mode: wgpu::PresentMode::AutoNoVsync,
                desired_maximum_frame_latency: 2,
            },
        );
    }

    #[wasm_bindgen(getter)]
    pub fn html(&self) -> Option<web_sys::HtmlCanvasElement> {
        self.html.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn offscreen(&self) -> Option<web_sys::OffscreenCanvas> {
        self.offscreen.clone()
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Document(Rc<DocumentInner>);

struct DocumentInner {
    runtime: Runtime,
    data: RefCell<DocumentData>,
}

struct DocumentData {
    name: String,
    size: [u32; 2],
    layers: Vec<Layer>,
}

#[derive(Serialize, Deserialize)]
pub enum BrushPreset {
    Stamp(stamp::StampBrush),
}

#[wasm_bindgen]
impl Document {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.0.data.borrow().name.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_name(&self, name: String) {
        let mut data = self.0.data.borrow_mut();
        data.deref_mut().name = name;
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.0.data.borrow().size[0]
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.0.data.borrow().size[1]
    }

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> Vec<u32> {
        self.0.data.borrow().size.clone().into()
    }

    #[wasm_bindgen(setter)]
    pub fn set_size(&self, size: Vec<u32>) -> Result<(), JsError> {
        let size: [u32; 2] = size.try_into().map_err(|_| JsError::new("Array length must be 2"))?;
        let mut encoder = self.0.runtime.0.device.create_command_encoder(&Default::default());
        let mut data = self.0.data.borrow_mut();
        let old_size = data.size;
        data.size = size;

        for layer in &mut data.layers {
            let mut layer_data = layer.0.data.borrow_mut();
            let new_texture = self.0.runtime.0.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                format: wgpu::TextureFormat::Rgba8Unorm,
                dimension: wgpu::TextureDimension::D2,
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    ..Default::default()
                },
                mip_level_count: 1,
                sample_count: 1,
                view_formats: &[],
                usage: wgpu::TextureUsages::empty()
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            });
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &layer_data.texture,
                    aspect: wgpu::TextureAspect::All,
                    origin: wgpu::Origin3d::ZERO,
                    mip_level: 0,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &new_texture,
                    aspect: wgpu::TextureAspect::All,
                    origin: wgpu::Origin3d::ZERO,
                    mip_level: 0,
                },
                wgpu::Extent3d {
                    width: old_size[0].min(size[0]),
                    height: old_size[1].min(size[1]),
                    ..Default::default()
                },
            );
            layer_data.copy_bind_group = self.0.runtime.0.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.0.runtime.0.copy_bind_group,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.0.runtime.0.device.create_sampler(
                            &wgpu::SamplerDescriptor {
                                min_filter: wgpu::FilterMode::Linear,
                                mag_filter: wgpu::FilterMode::Linear,
                                ..Default::default()
                            },
                        )),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&new_texture.create_view(&Default::default())),
                    },
                ],
            });
            layer_data.texture = new_texture;
        }

        self.0.runtime.0.queue.submit([encoder.finish()]);
        Ok(())
    }

    #[wasm_bindgen(getter, js_name = "layers")]
    pub fn layer_count(&self) -> usize {
        self.0.data.borrow().layers.len()
    }

    /// Once you are done with the layer, don't forget to `free()` it.
    #[wasm_bindgen(js_name = "layerAt")]
    pub fn layer_at(&self, index: usize) -> Option<Layer> {
        self.0.data.borrow().layers.get(index).cloned()
    }

    /// Make sure to `free()` the return value.
    #[wasm_bindgen(js_name = "insertLayer")]
    pub fn insert_layer(&self, index: usize, name: String) -> Result<Layer, JsError> {
        let mut data = self.0.data.borrow_mut();

        if index > data.layers.len() {
            return Err(JsError::new("Index out of bounds"));
        }

        let texture = self.0.runtime.0.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            format: wgpu::TextureFormat::Rgba8Unorm,
            dimension: wgpu::TextureDimension::D2,
            size: wgpu::Extent3d {
                width: data.size[0],
                height: data.size[1],
                ..Default::default()
            },
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
            usage: wgpu::TextureUsages::empty()
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
        });
        let copy_bind_group = self.0.runtime.0.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.0.runtime.0.copy_bind_group,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.0.runtime.0.device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            min_filter: wgpu::FilterMode::Linear,
                            mag_filter: wgpu::FilterMode::Linear,
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                },
            ],
        });
        let layer = Layer(Rc::new(LayerInner {
            document: self.clone(),
            data: RefCell::new(LayerData {
                name,
                texture,
                copy_bind_group,
            }),
        }));

        data.layers.insert(index, layer.clone());
        Ok(layer)
    }

    #[wasm_bindgen(js_name = "deleteLayer")]
    pub fn delete_layer(&self, index: usize) -> Result<(), JsError> {
        let mut data = self.0.data.borrow_mut();

        if index >= data.layers.len() {
            return Err(JsError::new("Index out of bounds"));
        }

        data.layers.remove(index);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn render(&self, surface: &Surface) -> Result<(), JsError> {
        let data = self.0.data.borrow();
        let surface_texture = surface.inner.get_current_texture()?;
        let mut encoder = self.0.runtime.0.device.create_command_encoder(&Default::default());
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        render_pass.set_pipeline(&self.0.runtime.0.copy_pipeline);

        for layer in &data.layers {
            let layer_data = layer.0.data.borrow();
            render_pass.set_bind_group(0, &layer_data.copy_bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }

        drop(render_pass);
        self.0.runtime.0.queue.submit([encoder.finish()]);
        surface_texture.present();
        Ok(())
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Layer(Rc<LayerInner>);

struct LayerInner {
    document: Document,
    data: RefCell<LayerData>,
}

struct LayerData {
    name: String,
    texture: wgpu::Texture,
    copy_bind_group: wgpu::BindGroup,
}

#[wasm_bindgen]
impl Layer {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.0.data.borrow().name.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_name(&self, name: String) {
        let mut data = self.0.data.borrow_mut();
        data.name = name;
    }
}
