struct CellState {
    model: mat4x4f,
    color: vec4f,
}

@group(0) @binding(0) var<storage, read> input: array<CellState>;
@group(0) @binding(1) var<storage, read_write> output: array<CellState>;

@compute
@workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= 70 || id.y >= 50 {
        return;
    }

    let index = id.y * 70 + id.x;
    var prev = input[index];
    prev.color += vec4f(0.01, 0.01, 0.01, 0.0);

    output[index] = prev;
}
