#import bevy_shader_utils::perlin_noise_2d
#import bevy_pbr::mesh_view_bindings

@fragment
fn fragment(
    @builtin(position) coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) normals: vec3<f32>,
    @location(2) uv: vec2<f32>
) -> @location(0) vec4<f32> {
    var input1: vec2<f32> = vec2<f32>(world_position.x, globals.time);
    var input2: vec2<f32> = vec2<f32>(world_position.y, globals.time);
    var input3: vec2<f32> = vec2<f32>(world_position.z, globals.time);

    var noise1 = perlinNoise2(input1);
    var noise2 = perlinNoise2(input2);
    var noise3 = perlinNoise2(input3);

    var value1 = (noise1 + 1.0) / 2.0;
    var value2 = (noise2 + 1.0) / 2.0;
    var value3 = (noise3 + 1.0) / 2.0;

    // var input_u1: vec2<f32> = vec2<f32>(uv.x  / 50.0, material.time);
    // var input_u2: vec2<f32> = vec2<f32>(uv.y  / 50.0, material.time);

    // var noise_u1 = perlinNoise2(input_u1);
    // var noise_u2 = perlinNoise2(input_u2);

    // var value_u1 = (noise_u1 + 1.0) / 2.0;
    // var value_u2 = (noise_u2 + 1.0) / 2.0;

    // return vec4<f32>(normals.x, normals.y, normals.z, 1.0);
    // return world_position;
    if (normals.z == 1.0) {
        return vec4<f32>(value1 - 0.1, value2 - 0.1, value3 - 0.1, 1.0);
    } else if (normals.y == 1.0) {
        return vec4<f32>(value1 - 0.2, value2 - 0.2, value3 - 0.2, 1.0);
    } else {
        return vec4<f32>(value1, value2, value3, 1.0);
    }
    // return vec4<f32>(uv.x,uv.y,value3,1.0);
}
