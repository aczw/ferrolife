use wgpu::util::DeviceExt;

use crate::{instance::Instance, texture, vertex::Vertex};

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

pub struct Cells {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    num_indices: u32,

    depth_texture: texture::Texture,
    pipeline: wgpu::RenderPipeline,
}

impl Cells {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        camera_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells-vertex-buf"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cells-index-buf"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let depth_texture = texture::Texture::create_depth_texture(
            &device,
            &surface_config,
            texture::Texture::DEPTH_TEXTURE_LABEL,
        );

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/cells.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cells-render-pipeline-layout"),
                bind_group_layouts: &[Some(&camera_bgl)],
                immediate_size: 0,
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cells-render-pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::buf_layout(), Instance::buf_layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            vertex_buf,
            index_buf,
            num_indices: INDICES.len() as u32,
            depth_texture,
            pipeline: render_pipeline,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) {
        self.depth_texture = texture::Texture::create_depth_texture(
            device,
            surface_config,
            texture::Texture::DEPTH_TEXTURE_LABEL,
        );
    }

    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        camera_bg: &wgpu::BindGroup,
        instance_buf_to_use: &wgpu::Buffer,
        num_instances: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cells-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bg, &[]);

        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_vertex_buffer(1, instance_buf_to_use.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);

        pass.draw_indexed(0..self.num_indices, 0, 0..num_instances);
    }
}
