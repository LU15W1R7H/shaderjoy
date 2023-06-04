struct Uniform {
  screen_resolution: vec2<f32>,
  time: f32,
  _pad0: f32,
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


fn palette(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
  return a + b * cos(6.28318 * (c * t + d));
}

fn my_palette(t: f32) -> vec3<f32> {
  let a = vec3(0.5, 0.5, 0.5);
  let b = vec3(0.5, 0.5, 0.5);
  let c = vec3(1.0, 1.0, 1.0);
  let d = vec3(0.263, 0.416, 0.557);
  return palette(t, a, b, c, d);
}

@fragment
fn fs_main(
  in: VertexOutput,
) -> @location(0) vec4<f32> {
  var pos = in.pos.xy;
  let res = uniform.screen_resolution;
  let time = uniform.time;

  pos = (pos * 2.0 - res) / res.y;
  let pos0 = pos;
  var final_color = vec3(0.0);

  for (var i: f32 = 0.0; i < 4.0; i += 1.0) {
    pos = fract(1.5 * pos) - 0.5;

    var d = length(pos) * exp(-length(pos0));

    var col = my_palette(length(pos0) + i * 0.4 + time * 0.2);

    d = sin(d * 8.0 + time) / 8.0;
    d = abs(d);
    d = pow(0.01 / d, 2.0);

    final_color += col * d;
  }

  return vec4<f32>(final_color, 1.0);
}
