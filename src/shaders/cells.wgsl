struct CameraUniform {
    view_proj: mat4x4f,
}

struct VertexInput {
    @location(0) position: vec3f,
}

struct InstanceInput {
    @location(1) color: vec4f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec4f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

const GRID_WIDTH: f32 = 400.0;
const GRID_HEIGHT: f32 = 300.0;

@vertex
fn vs_main(
    vert_in: VertexInput,
    inst_in: InstanceInput,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let x = f32(instance_index % 400u);
    let y = f32(instance_index / 400u);
    let translation = vec2f(x - (GRID_WIDTH - 1.0) * 0.5, y - (GRID_HEIGHT - 1.0) * 0.5);

    var out: VertexOutput;
    out.color = inst_in.color;
    out.clip_position = camera.view_proj * vec4f(vert_in.position.xy + translation, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}
