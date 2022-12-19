struct PushConstants {
    res: vec2<f32>,
};
var<push_constant> pc: PushConstants;

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
    let pos: vec2<f32> = vec2<f32>(f32((id << u32(1)) & u32(2)), f32(id & u32(2)));
    return vec4<f32>(pos - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    if pos.x > 20.0 && pos.x < pc.res.x - 20.0 && pos.y > 20.0 && pos.y < pc.res.y - 20.0 {
       discard;
    }
    return vec4<f32>(pos.xy / pc.res, 0.0, 1.0);
}
