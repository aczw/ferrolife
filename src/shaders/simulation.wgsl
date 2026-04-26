struct CellState {
    model: mat4x4f,
    color: vec4f,
}

@group(0) @binding(0) var<storage, read> input: array<CellState>;
@group(0) @binding(1) var<storage, read_write> output: array<CellState>;

@group(1) @binding(0) var<uniform> grid_dims: vec2u;

const alive_threshold = 0.3f;
const neighbor_blend = 0.2f;

const neighbor_deltas = array<vec2i, 8>(
    vec2i(-1, -1),
    vec2i(-1, 0),
    vec2i(-1, 1),
    vec2i(0, 1),
    vec2i(1, 1),
    vec2i(1, 0),
    vec2i(1, -1),
    vec2i(0, -1)
);

fn get_index(id: vec3u) -> u32 {
    return id.y * grid_dims.x + id.x;
}

fn is_alive(color: vec3f) -> bool {
    return max(color.r, max(color.g, color.b)) > alive_threshold;
}

struct NeighborState {
    num_alive: u32,
    avg_color: vec3f,
}

fn get_neighbor_state(id: vec3u) -> NeighborState {
    var state = NeighborState(0u, vec3f(0.0));

    var total_color = vec3f(0.0);
    for (var i = 0u; i < 8u; i++) {
        let neighbor = vec3i(id) + vec3i(neighbor_deltas[i], 0);
        if neighbor.x >= 0 && neighbor.x < i32(grid_dims.x) &&
            neighbor.y >= 0 && neighbor.y < i32(grid_dims.y) {
            let cell = input[get_index(vec3u(neighbor))];
            if is_alive(cell.color.rgb) {
                state.num_alive += 1u;
                total_color += cell.color.rgb;
            }
        }
    }

    if state.num_alive > 0u {
        state.avg_color = total_color / f32(state.num_alive);
    }

    return state;
}

fn update_color(prev_color: vec3f, state: NeighborState) -> vec3f {
    if is_alive(prev_color) {
        if state.num_alive == 2u || state.num_alive == 3u {
            return prev_color * (1.0 - neighbor_blend) + state.avg_color * neighbor_blend;
        }
        return vec3f(0.0);
    }

    if state.num_alive == 3u {
        return state.avg_color;
    }
    return vec3f(0.0);
}

@compute
@workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= grid_dims.x || id.y >= grid_dims.y {
        return;
    }

    let index = get_index(id);
    var prev = input[index];

    let neighbor_state = get_neighbor_state(id);
    prev.color = vec4(update_color(prev.color.rgb, neighbor_state), 1.0);

    output[index] = prev;
}
