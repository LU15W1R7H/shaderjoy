struct Uniform {
  screen_resolution: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniform: Uniform;

struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
  @builtin(vertex_index) ivertex: u32
) -> VertexOutput {
  var positions = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, 1.0)
  );
  let pos = positions[ivertex];

  var out: VertexOutput;
  out.pos = vec4<f32>(pos, 0.0, 1.0);
  out.uv = pos * 0.5 + 0.5;
  return out;
}

@fragment
fn fs_main(
  in: VertexOutput,
) -> @location(0) vec4<f32> {
  var uv = in.pos.xy / uniform.screen_resolution;
  return vec4<f32>(uv.x, uv.y, 0.0, 1.0);
}
