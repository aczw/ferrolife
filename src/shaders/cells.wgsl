struct CameraUniform {
    view_proj: mat4x4f,
}

struct VertexInput {
    @location(0) position: vec3f,
}

struct InstanceInput {
    @location(1) model_mat_0: vec4f,
    @location(2) model_mat_1: vec4f,
    @location(3) model_mat_2: vec4f,
    @location(4) model_mat_3: vec4f,
    @location(5) color: vec3f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec3f,
}

@binding(0) @group(0) var<uniform> camera: CameraUniform;

@vertex
fn vs_main(vert_in: VertexInput, inst_in: InstanceInput) -> VertexOutput {
    let instance_model = mat4x4f(
        inst_in.model_mat_0,
        inst_in.model_mat_1,
        inst_in.model_mat_2,
        inst_in.model_mat_3,
    );

    var out: VertexOutput;
    out.color = inst_in.color;
    out.clip_position = camera.view_proj * instance_model * vec4f(vert_in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}
