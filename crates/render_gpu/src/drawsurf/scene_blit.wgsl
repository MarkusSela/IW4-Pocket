@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOut;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}
@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(scene, scene_sampler, in.uv);
}
