@group(0)
@binding(0)
var texture: texture_storage_2d<rgba8uint, write>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dim = textureDimensions(texture);
    textureStore(texture, vec2(50, 50), vec4(u32(20.0)));
}
