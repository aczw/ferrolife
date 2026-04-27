use cgmath::{Vector3, Vector4};
use wgpu::util::DeviceExt;

use crate::instance::{Instance, pack_color};

pub const GRID_WIDTH: u32 = 400;
pub const GRID_HEIGHT: u32 = 300;

/// Has to match with the `@workgroup_size` used in the compute shader.
const WORKGROUP_SIZE: u32 = 16;
const NUM_WORKGROUPS_X: u32 = GRID_WIDTH.div_ceil(WORKGROUP_SIZE);
const NUM_WORKGROUPS_Y: u32 = GRID_HEIGHT.div_ceil(WORKGROUP_SIZE);
const INITIAL_LIVE_DENSITY: f32 = 0.20;
const DEFAULT_ALIVE_THRESHOLD: f32 = 0.30;
const DEFAULT_BORN_RULES: u16 = 0b0000_1000;
const DEFAULT_SURVIVE_RULES: u16 = 0b0000_1100;

fn hash01(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B9) ^ y.wrapping_mul(0x85EB_CA6B) ^ seed;
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vector3<f32> {
    let h6 = (h.fract() * 6.0).abs();
    let c = v * s;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h6 < 1.0 {
        (c, x, 0.0)
    } else if h6 < 2.0 {
        (x, c, 0.0)
    } else if h6 < 3.0 {
        (0.0, c, x)
    } else if h6 < 4.0 {
        (0.0, x, c)
    } else if h6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vector3::new(r1 + m, g1 + m, b1 + m)
}

fn initial_color(x: u32, y: u32) -> Vector4<f32> {
    let is_alive = hash01(x, y, 0xA511_E9B3) < INITIAL_LIVE_DENSITY;
    if !is_alive {
        return Vector4::new(0.0, 0.0, 0.0, 1.0);
    }

    let hue = hash01(x, y, 0x8DA6_B343);
    let saturation = 0.82 + 0.16 * hash01(x, y, 0xD816_3841);
    let value = 0.78 + 0.20 * hash01(x, y, 0xCB1A_B31F);
    let color = hsv_to_rgb(hue, saturation, value);

    Vector4::new(color.x, color.y, color.z, 1.0)
}

enum CurrentInstanceBuffer {
    A,
    B,
}

pub struct Simulation {
    current_instance_buf: CurrentInstanceBuffer,
    state_buf_a: wgpu::Buffer,
    state_buf_b: wgpu::Buffer,

    ping_pong_bufs_bgl: wgpu::BindGroupLayout,
    grid_dims_bg: wgpu::BindGroup,
    simulation_params_buf: wgpu::Buffer,
    alive_threshold: f32,
    born_rules: u16,
    survive_rules: u16,
    pipeline: wgpu::ComputePipeline,
}

impl Simulation {
    pub fn new(device: &wgpu::Device) -> Self {
        let instance_raw_data: Vec<Instance> = (0..GRID_HEIGHT)
            .flat_map(|y| {
                (0..GRID_WIDTH).map(move |x| {
                    let color = initial_color(x, y);
                    Instance {
                        color: pack_color(color),
                    }
                })
            })
            .collect();

        let state_buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simulation-state-buf-a"),
            contents: bytemuck::cast_slice(&instance_raw_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });
        let state_buf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simulation-state-buf-b"),
            contents: bytemuck::cast_slice(&instance_raw_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let grid_dims_unif_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simulation-grid-dims-unif-buf"),
            contents: bytemuck::cast_slice(&[GRID_WIDTH, GRID_HEIGHT]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let simulation_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simulation-params-unif-buf"),
            contents: bytemuck::cast_slice(&[[
                DEFAULT_ALIVE_THRESHOLD,
                DEFAULT_BORN_RULES as f32,
                DEFAULT_SURVIVE_RULES as f32,
                0.0,
            ]]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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

        let grid_dims_and_params_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("simulation-grid-dims-and-params-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let grid_dims_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simulation-grid-dims-and-params-bg"),
            layout: &grid_dims_and_params_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_dims_unif_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: simulation_params_buf.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("simulation-compute-pipeline-layout"),
                bind_group_layouts: &[Some(&ping_pong_bufs_bgl), Some(&grid_dims_and_params_bgl)],
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
            current_instance_buf: CurrentInstanceBuffer::A,
            state_buf_a,
            state_buf_b,
            ping_pong_bufs_bgl,
            grid_dims_bg,
            simulation_params_buf,
            alive_threshold: DEFAULT_ALIVE_THRESHOLD,
            born_rules: DEFAULT_BORN_RULES,
            survive_rules: DEFAULT_SURVIVE_RULES,
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
        pass.set_bind_group(1, &self.grid_dims_bg, &[]);

        pass.dispatch_workgroups(NUM_WORKGROUPS_X, NUM_WORKGROUPS_Y, 1);

        buf_written_to
    }

    pub fn num_instances(&self) -> usize {
        (GRID_WIDTH * GRID_HEIGHT) as usize
    }

    pub fn current_instance_buf_to_use(&self) -> &wgpu::Buffer {
        match self.current_instance_buf {
            CurrentInstanceBuffer::A => &self.state_buf_a,
            CurrentInstanceBuffer::B => &self.state_buf_b,
        }
    }

    pub fn set_cell_color(&mut self, queue: &wgpu::Queue, x: u32, y: u32, color: Vector4<f32>) {
        if x >= GRID_WIDTH || y >= GRID_HEIGHT {
            return;
        }

        let index = (y * GRID_WIDTH + x) as wgpu::BufferAddress;
        let offset = index * std::mem::size_of::<Instance>() as wgpu::BufferAddress;
        let instance = [Instance {
            color: pack_color(color),
        }];
        let bytes = bytemuck::cast_slice(&instance);

        queue.write_buffer(&self.state_buf_a, offset, bytes);
        queue.write_buffer(&self.state_buf_b, offset, bytes);
    }

    pub fn clear_board(&mut self, queue: &wgpu::Queue) {
        let black = Instance {
            color: pack_color(Vector4::new(0.0, 0.0, 0.0, 1.0)),
        };
        let clear_data = vec![black; self.num_instances()];
        let bytes = bytemuck::cast_slice(&clear_data);

        queue.write_buffer(&self.state_buf_a, 0, bytes);
        queue.write_buffer(&self.state_buf_b, 0, bytes);
        self.current_instance_buf = CurrentInstanceBuffer::A;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn alive_threshold(&self) -> f32 {
        self.alive_threshold
    }

    pub fn set_alive_threshold(&mut self, queue: &wgpu::Queue, alive_threshold: f32) {
        self.alive_threshold = alive_threshold.clamp(0.0, 1.0);
        self.write_simulation_params(queue);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn born_rules(&self) -> u16 {
        self.born_rules
    }

    pub fn set_born_rules(&mut self, queue: &wgpu::Queue, born_rules: u16) {
        self.born_rules = born_rules;
        self.write_simulation_params(queue);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn survive_rules(&self) -> u16 {
        self.survive_rules
    }

    pub fn set_survive_rules(&mut self, queue: &wgpu::Queue, survive_rules: u16) {
        self.survive_rules = survive_rules;
        self.write_simulation_params(queue);
    }

    fn write_simulation_params(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.simulation_params_buf,
            0,
            bytemuck::cast_slice(&[[
                self.alive_threshold,
                self.born_rules as f32,
                self.survive_rules as f32,
                0.0,
            ]]),
        );
    }

    pub fn set_state_from_rgba_image(
        &mut self,
        queue: &wgpu::Queue,
        image_width: u32,
        image_height: u32,
        rgba_bytes: &[u8],
    ) -> anyhow::Result<()> {
        let expected_len = (image_width as usize)
            .saturating_mul(image_height as usize)
            .saturating_mul(4);
        if rgba_bytes.len() != expected_len {
            anyhow::bail!(
                "invalid image data length: expected {expected_len}, got {}",
                rgba_bytes.len()
            );
        }

        let mut state_data = Vec::with_capacity(self.num_instances());

        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let src_x = x.saturating_mul(image_width) / GRID_WIDTH;
                // Flip Y to keep uploaded images visually upright.
                let src_y = (GRID_HEIGHT - 1 - y).saturating_mul(image_height) / GRID_HEIGHT;
                let src_idx = ((src_y * image_width + src_x) * 4) as usize;
                let alpha = rgba_bytes[src_idx + 3] as f32 / 255.0;

                // See: https://en.wikipedia.org/wiki/SRGB#Transfer_function_(%22gamma%22)
                let srgb_to_linear = |srgb: f32| {
                    if srgb <= 0.04045 {
                        srgb / 12.92
                    } else {
                        ((srgb + 0.055) / 1.055).powf(2.4)
                    }
                };

                let to_channel = |byte: u8| {
                    let srgb = byte as f32 / 255.0;
                    srgb_to_linear(srgb) * alpha
                };

                let color = Vector4::new(
                    to_channel(rgba_bytes[src_idx]),
                    to_channel(rgba_bytes[src_idx + 1]),
                    to_channel(rgba_bytes[src_idx + 2]),
                    1.0,
                );
                state_data.push(Instance {
                    color: pack_color(color),
                });
            }
        }

        queue.write_buffer(&self.state_buf_a, 0, bytemuck::cast_slice(&state_data));
        queue.write_buffer(&self.state_buf_b, 0, bytemuck::cast_slice(&state_data));
        self.current_instance_buf = CurrentInstanceBuffer::A;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_current_instances(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<Vec<Instance>> {
        let instance_size = std::mem::size_of::<Instance>() as u64;
        let byte_len = self.num_instances() as u64 * instance_size;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simulation-readback-staging-buffer"),
            size: byte_len,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("simulation-readback-encoder"),
        });
        encoder.copy_buffer_to_buffer(
            self.current_instance_buf_to_use(),
            0,
            &staging_buffer,
            0,
            byte_len,
        );
        queue.submit(std::iter::once(encoder.finish()));

        let (tx, rx) = std::sync::mpsc::channel();
        staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });

        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()
            .map_err(|err| anyhow::anyhow!("failed to receive map_async callback: {err}"))??;

        let mapped = staging_buffer.slice(..).get_mapped_range();
        let mut instances = Vec::with_capacity(self.num_instances());
        for chunk in mapped.chunks_exact(instance_size as usize) {
            let color = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            instances.push(Instance { color });
        }
        drop(mapped);
        staging_buffer.unmap();

        Ok(instances)
    }
}
