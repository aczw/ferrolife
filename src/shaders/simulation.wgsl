struct CellState {
    model: mat4x4f,
    color: vec4f,
}

@group(0) @binding(0) var<storage, read> input: array<CellState>;
@group(0) @binding(1) var<storage, read_write> output: array<CellState>;

@group(1) @binding(0) var<uniform> grid_dims: vec2u;

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

struct NeighborState {
    num_alive: vec3u, // number of alive neighbor cells per channel
    avg_color: vec3f,
}

fn get_neighbor_state(id: vec3u) -> NeighborState {
    var state = NeighborState(vec3u(0u), vec3f(0.0));
    var total_color = vec3f(0.0);
    for (var i = 0u; i < 8u; i++) {
        let neighbor = vec3i(id) + vec3i(neighbor_deltas[i], 0);
        if neighbor.x >= 0 && neighbor.x < i32(grid_dims.x) &&
            neighbor.y >= 0 && neighbor.y < i32(grid_dims.y) {
            let cell = input[get_index(vec3u(neighbor))];
            total_color += cell.color.rgb;
            let alive_mask = cell.color.rgb > vec3f(0.0);
            state.num_alive += select(vec3u(0u), vec3u(1u), alive_mask);
        }
    }

    let has_alive = state.num_alive > vec3u(0u);
    let denom = vec3f(max(state.num_alive, vec3u(1u)));
    state.avg_color = select(vec3f(0.0), total_color / denom, has_alive);

    return state;
}

fn update_channel(prev_channel: f32, alive_neighbors: u32, avg_channel: f32) -> f32 {
    if prev_channel > 0.0 {
        if alive_neighbors == 2u || alive_neighbors == 3u {
            return prev_channel;
        }
        return 0.0;
    }

    if alive_neighbors == 3u {
        return avg_channel;
    }
    return 0.0;
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

    prev.color.r = update_channel(prev.color.r, neighbor_state.num_alive.r, neighbor_state.avg_color.r);
    prev.color.g = update_channel(prev.color.g, neighbor_state.num_alive.g, neighbor_state.avg_color.g);
    prev.color.b = update_channel(prev.color.b, neighbor_state.num_alive.b, neighbor_state.avg_color.b);

    output[index] = prev;
}
