use cgmath::Vector4;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub color: u32,
}

pub fn float_to_u8(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u32
}

pub fn pack_color(color: Vector4<f32>) -> u32 {
    let r = float_to_u8(color.x) as u32;
    let g = float_to_u8(color.y) as u32;
    let b = float_to_u8(color.z) as u32;
    let a = float_to_u8(color.w) as u32;

    r | (g << 8) | (b << 16) | (a << 24)
}

impl Instance {
    pub fn buf_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Unorm8x4,
                offset: 0,
                shader_location: 1,
            }],
        }
    }
}
