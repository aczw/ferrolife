pub struct Simulation {
    state_buf_a: wgpu::Buffer,
    state_buf_b: wgpu::Buffer,

    ping_pong_bufs_bgl: wgpu::BindGroupLayout,

    pipeline: wgpu::ComputePipeline,
}

impl Simulation {
    pub fn new(device: &wgpu::Device) -> Self {
        let state_buf_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("state-buf-a"),
            size: 1,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let state_buf_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("state-buf-b"),
            size: 1,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let ping_pong_bufs_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ping-pong-bufs-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute-pipeline-layout"),
                bind_group_layouts: &[Some(&ping_pong_bufs_bgl)],
                immediate_size: 0,
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute-pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &device.create_shader_module(wgpu::include_wgsl!("shaders/simulation.wgsl")),
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            state_buf_a,
            state_buf_b,
            ping_pong_bufs_bgl,
            pipeline: compute_pipeline,
        }
    }
}
