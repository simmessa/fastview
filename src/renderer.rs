use wgpu;
use image::RgbaImage;
use bytemuck::{Pod, Zeroable};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Params {
    pub image_size: [f32; 2],
    pub window_size: [f32; 2],
    pub pan: [f32; 2],
    pub zoom: f32,
    pub is_grid_item: f32, // 0.0 for single view, 1.0 for grid
    pub is_selected: f32,
    pub _pad: f32,
    pub _pad2: [f32; 2], // Pad to 48 bytes (12 floats)
}

pub struct GridItem {
    pub path: PathBuf,
    pub is_directory: bool,
    pub texture_bind_group: wgpu::BindGroup,
    pub params_buffer: wgpu::Buffer,
    pub params_bind_group: wgpu::BindGroup,
    pub image_size: [f32; 2],
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    
    // Single view state
    diffuse_bind_group: wgpu::BindGroup,
    
    // Common resources
    texture_bind_group_layout: wgpu::BindGroupLayout,
    params_bind_group_layout: wgpu::BindGroupLayout,
    
    // Global params (used for single view)
    params_buffer: wgpu::Buffer,
    params_bind_group: wgpu::BindGroup,
    params: Params,

    // Grid view state
    pub grid_items: Vec<GridItem>,
    pub grid_scroll: f32,

    // Samplers
    sampler_linear: wgpu::Sampler,
    sampler_nearest: wgpu::Sampler,
    pub is_nearest: bool,
}

impl Renderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: wgpu::Adapter,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Self {
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())            
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let params_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("params_bind_group_layout"),
        });

        let params = Params {
            image_size: [1.0, 1.0],
            window_size: [width as f32, height as f32],
            pan: [0.0, 0.0],
            zoom: 1.0,
            is_grid_item: 0.0,
            is_selected: 0.0,
            _pad: 0.0,
            _pad2: [0.0; 2],
        };

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Params Buffer"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &params_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                }
            ],
            label: Some("params_bind_group"),
        });

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let empty_img = RgbaImage::new(1, 1);
        let diffuse_bind_group = Self::create_texture_bind_group(&device, &queue, &texture_bind_group_layout, &empty_img, &sampler_linear);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &params_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
            cache: None,
        });

        Renderer {
            device,
            queue,
            surface,
            config,
            render_pipeline,
            diffuse_bind_group,
            texture_bind_group_layout,
            params_bind_group_layout,
            params,
            params_buffer,
            params_bind_group,
            grid_items: Vec::new(),
            grid_scroll: 0.0,
            sampler_linear,
            sampler_nearest,
            is_nearest: false,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.config.width = width;
        self.config.height = height;
        self.params.window_size = [width as f32, height as f32];
        self.surface.configure(&self.device, &self.config);
    }

    pub fn update_texture(&mut self, img: &RgbaImage) {
        let dimensions = img.dimensions();
        self.params.image_size = [dimensions.0 as f32, dimensions.1 as f32];
        self.params.pan = [0.0, 0.0];
        
        let sampler = if self.is_nearest { &self.sampler_nearest } else { &self.sampler_linear };
        self.diffuse_bind_group = Self::create_texture_bind_group(&self.device, &self.queue, &self.texture_bind_group_layout, img, sampler);
    }

    pub fn set_filtering(&mut self, nearest: bool, img: Option<&RgbaImage>) {
        self.is_nearest = nearest;
        if let Some(image) = img {
            self.update_texture(image);
        }
    }

    pub fn add_grid_item(&mut self, path: PathBuf, is_directory: bool, img: Option<&RgbaImage>) {
        let placeholder = RgbaImage::new(1, 1);
        let actual_img = img.unwrap_or(&placeholder);
        
        let sampler = &self.sampler_linear;
        let texture_bind_group = Self::create_texture_bind_group(&self.device, &self.queue, &self.texture_bind_group_layout, actual_img, sampler);
        
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Item Params Buffer"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.params_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                }
            ],
            label: Some("grid_item_params_bind_group"),
        });

        self.grid_items.push(GridItem {
            path,
            is_directory,
            texture_bind_group,
            params_buffer,
            params_bind_group,
            image_size: [actual_img.width() as f32, actual_img.height() as f32],
        });
    }

    pub fn clear_grid(&mut self) {
        self.grid_items.clear();
        self.grid_scroll = 0.0;
    }

    pub fn scroll_grid(&mut self, dy: f32) {
        let grid_size = 250.0;
        let spacing = 20.0;
        let window_width = self.params.window_size[0];
        let window_height = self.params.window_size[1];
        
        let cols = (window_width / (grid_size + spacing)).floor().max(1.0) as u32;
        let rows = (self.grid_items.len() as f32 / cols as f32).ceil();
        let content_height = rows * (grid_size + spacing) + spacing;
        
        let max_scroll = (content_height - window_height).max(0.0);
        
        self.grid_scroll += dy;
        self.grid_scroll = self.grid_scroll.clamp(-max_scroll, 0.0);
    }

    pub fn zoom(&mut self, amount: f32) {
        if self.params.is_grid_item > 0.5 {
            self.scroll_grid(amount * 100.0);
            return;
        }
        let factor = 1.1f32;
        if amount > 0.0 { self.params.zoom *= factor; } else { self.params.zoom /= factor; }
        self.params.zoom = self.params.zoom.clamp(0.1, 50.0);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        if self.params.is_grid_item > 0.5 { return; }
        self.params.pan[0] += dx;
        self.params.pan[1] += dy;
    }

    pub fn get_zoom(&self) -> f32 {
        self.params.zoom
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.params.zoom = zoom;
    }

    pub fn get_image_size(&self) -> [f32; 2] {
        self.params.image_size
    }

    pub fn update_grid_item_texture(&mut self, index: usize, img: &RgbaImage) {
        if let Some(item) = self.grid_items.get_mut(index) {
            item.texture_bind_group = Self::create_texture_bind_group(&self.device, &self.queue, &self.texture_bind_group_layout, img, &self.sampler_linear);
            item.image_size = [img.width() as f32, img.height() as f32];
        }
    }

    pub fn set_view_mode(&mut self, is_grid: bool) {
        self.params.is_grid_item = if is_grid { 1.0 } else { 0.0 };
    }

    fn create_texture_bind_group(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, img: &RgbaImage, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
        let dimensions = img.dimensions();
        let texture_size = wgpu::Extent3d { width: dimensions.0, height: dimensions.1, depth_or_array_layers: 1 };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        })
    }

    pub fn render(&mut self, is_grid: bool, selected_index: Option<usize>) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if !is_grid {
            self.params.is_grid_item = 0.0;
            self.params.is_selected = 0.0;
            self.queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&self.params));
            {
                let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
                });
                rp.set_pipeline(&self.render_pipeline);
                rp.set_bind_group(0, &self.diffuse_bind_group, &[]);
                rp.set_bind_group(1, &self.params_bind_group, &[]);
                rp.draw(0..3, 0..1);
            }
        } else {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.01, g: 0.01, b: 0.012, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
            });
            rp.set_pipeline(&self.render_pipeline);

            let grid_size = 250.0;
            let spacing = 20.0;
            let cols = (self.params.window_size[0] / (grid_size + spacing)).floor().max(1.0) as u32;
            
            for (i, item) in self.grid_items.iter().enumerate() {
                let col = (i as u32) % cols;
                let row = (i as u32) / cols;
                
                let x = spacing + (col as f32) * (grid_size + spacing);
                let y = spacing + (row as f32) * (grid_size + spacing) + self.grid_scroll;

                if y + grid_size < 0.0 || y > self.params.window_size[1] { continue; }
                
                let p = Params {
                    image_size: item.image_size,
                    window_size: self.params.window_size,
                    pan: [x, y],
                    zoom: grid_size,
                    is_grid_item: 1.0,
                    is_selected: if Some(i) == selected_index { 1.0 } else { 0.0 },
                    _pad: 0.0,
                    _pad2: [0.0; 2],
                };
                
                self.queue.write_buffer(&item.params_buffer, 0, bytemuck::bytes_of(&p));
                
                rp.set_bind_group(0, &item.texture_bind_group, &[]);
                rp.set_bind_group(1, &item.params_bind_group, &[]);
                rp.draw(0..3, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn get_window_size(&self) -> [f32; 2] {
        self.params.window_size
    }

    pub fn scroll_to_item(&mut self, index: usize) {
        let grid_size = 250.0;
        let spacing = 20.0;
        let window_height = self.params.window_size[1];
        let cols = (self.params.window_size[0] / (grid_size + spacing)).floor().max(1.0) as u32;
        
        let row = index as u32 / cols;
        let item_top = row as f32 * (grid_size + spacing) + spacing;
        let item_bottom = item_top + grid_size;
        
        // If above current view
        if item_top < -self.grid_scroll {
            self.grid_scroll = -item_top + spacing;
        }
        // If below current view
        else if item_bottom > -self.grid_scroll + window_height {
            self.grid_scroll = -item_bottom + window_height - spacing;
        }
        
        // Clamp scroll
        let rows = (self.grid_items.len() as f32 / cols as f32).ceil();
        let content_height = rows * (grid_size + spacing) + spacing;
        let max_scroll = (content_height - window_height).max(0.0);
        self.grid_scroll = self.grid_scroll.clamp(-max_scroll, 0.0);
    }
}