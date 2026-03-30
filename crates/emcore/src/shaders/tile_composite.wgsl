// Tile compositing shader: renders textured quads for each tile.

struct TileUniforms {
    // NDC offset (x, y) and scale (z, w) for positioning the quad.
    offset_scale: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: TileUniforms;
@group(0) @binding(1) var tile_texture: texture_2d<f32>;
@group(0) @binding(2) var tile_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Full-screen quad vertices (two triangles), indexed by vertex_index 0..5.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Quad corners: (0,0), (1,0), (0,1), (1,1)
    let uv = vec2<f32>(
        f32(vi & 1u),
        f32((vi >> 1u) & 1u),
    );

    // Triangle strip order for 6 vertices: 0,1,2, 2,1,3
    let idx = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);
    let corner = idx[vi];
    let corner_uv = vec2<f32>(
        f32(corner & 1u),
        f32((corner >> 1u) & 1u),
    );

    // Convert to NDC: offset + corner * scale
    let ndc = uniforms.offset_scale.xy + corner_uv * uniforms.offset_scale.zw;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = corner_uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tile_texture, tile_sampler, in.uv);
}
