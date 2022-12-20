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
    var uv: vec2<f32> = pos.xy - pc.res * 0.5;
    var color: vec4<f32> = vec4<f32>(pos.xy / pc.res, 0.0, 1.0);

    let border_radius: f32 = 8.0;

    var corner: vec2<f32> = abs(uv) - vec2<f32>(pc.res * 0.5 - border_radius);
    if all(corner > vec2<f32>(0.0)) {
        var edge = corner.x * corner.x + corner.y * corner.y - border_radius * border_radius;
        color *= smoothstep(0.0, -border_radius * 2.0, edge);
        if edge > 0.0 {
            discard;
        }
    }

    return color;
}
