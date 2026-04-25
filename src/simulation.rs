use cgmath::Vector3;
use wgpu::util::DeviceExt;

use crate::instance::Instance;

const GRID_WIDTH: u32 = 70;
const GRID_HEIGHT: u32 = 50;

/// Recenters the instances at the world space origin.
const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(
    (GRID_WIDTH - 1) as f32 * 0.5,
    (GRID_HEIGHT - 1) as f32 * 0.5,
    0.0,
);

/// Has to match with the `@workgroup_size` used in the compute shader.
const WORKGROUP_SIZE: u32 = 16;
const NUM_WORKGROUPS_X: u32 = GRID_WIDTH.div_ceil(WORKGROUP_SIZE);
const NUM_WORKGROUPS_Y: u32 = GRID_HEIGHT.div_ceil(WORKGROUP_SIZE);

enum CurrentInstanceBuffer {
    A,
    B,
}

pub struct Simulation {
    instances: Vec<Instance>,

    current_instance_buf: CurrentInstanceBuffer,
    state_buf_a: wgpu::Buffer,
    state_buf_b: wgpu::Buffer,

    ping_pong_bufs_bgl: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl Simulation {
    pub fn new(device: &wgpu::Device) -> Self {
        let instances = (0..GRID_HEIGHT)
            .flat_map(|y| {
                (0..GRID_WIDTH).map(move |x| {
                    let x_flt = x as f32;
                    let x_upper_bound = (GRID_WIDTH - 1) as f32;
                    let y_flt = y as f32;
                    let y_upper_bound = (GRID_HEIGHT - 1) as f32;

                    let translation = Vector3 {
                        x: x_flt,
                        y: y_flt,
                        z: 0.0,
                    } - INSTANCE_DISPLACEMENT;
                    let color = Vector3 {
                        x: x_flt / x_upper_bound,
                        y: y_flt / y_upper_bound,
                        z: 0.0,
                    };

                    Instance { translation, color }
                })
            })
            .collect::<Vec<_>>();
        let instance_raw_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        let state_buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simulation-state-buf-a"),
            contents: bytemuck::cast_slice(&instance_raw_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        });
        let state_buf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simulation-state-buf-a"),
            contents: bytemuck::cast_slice(&instance_raw_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        });

        let ping_pong_bufs_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("simulation-ping-pong-bufs-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("simulation-compute-pipeline-layout"),
                bind_group_layouts: &[Some(&ping_pong_bufs_bgl)],
                immediate_size: 0,
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("simulation-compute-pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &device.create_shader_module(wgpu::include_wgsl!("shaders/simulation.wgsl")),
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            instances,
            current_instance_buf: CurrentInstanceBuffer::A,
            state_buf_a,
            state_buf_b,
            ping_pong_bufs_bgl,
            pipeline: compute_pipeline,
        }
    }

    pub fn record(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
    ) -> &wgpu::Buffer {
        let (buf_written_to, buf_read_from) = match self.current_instance_buf {
            CurrentInstanceBuffer::A => {
                // Read from A, write to B
                self.current_instance_buf = CurrentInstanceBuffer::B;
                (&self.state_buf_b, &self.state_buf_a)
            }
            CurrentInstanceBuffer::B => {
                // Read from B, write to A
                self.current_instance_buf = CurrentInstanceBuffer::A;
                (&self.state_buf_a, &self.state_buf_b)
            }
        };

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("simulation-compute-pass"),
            timestamp_writes: None,
        });

        let ping_pong_bufs_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simulation-ping-pong-bufs-bg"),
            layout: &self.ping_pong_bufs_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf_read_from.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buf_written_to.as_entire_binding(),
                },
            ],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &ping_pong_bufs_bg, &[]);

        pass.dispatch_workgroups(NUM_WORKGROUPS_X, NUM_WORKGROUPS_Y, 1);

        buf_written_to
    }

    pub fn num_instances(&self) -> usize {
        self.instances.len()
    }
}
