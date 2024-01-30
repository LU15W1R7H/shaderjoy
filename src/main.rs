use std::{
  fs,
  sync::{Arc, Mutex, OnceLock},
  time::{Duration, Instant},
};

use notify::Watcher;
use wgpu::util::DeviceExt;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

static SHADER_SRC_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
  screen_resolution: [f32; 2],
  time: f32,
  _pad0: f32,
}

#[pollster::main]
async fn main() {
  env_logger::init();

  let event_loop = EventLoop::new().unwrap();
  event_loop.set_control_flow(ControlFlow::Poll);

  let window = WindowBuilder::new().build(&event_loop).unwrap();

  SHADER_SRC_PATH
    .set(std::path::PathBuf::from("./src/shader.wgsl"))
    .unwrap();

  let size = window.inner_size();

  let instance = wgpu::Instance::default();

  let surface = instance.create_surface(&window).unwrap();

  let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::LowPower,
      //power_preference: wgpu::PowerPreference::HighPerformance,
      force_fallback_adapter: false,
      compatible_surface: Some(&surface),
    })
    .await
    .expect("Failed to find an appropriate adapter");

  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
      },
      None,
    )
    .await
    .expect("Failed to create device");

  let swapchain_capabilities = surface.get_capabilities(&adapter);
  let swapchain_format = swapchain_capabilities.formats[0];
  let mut config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: swapchain_format,
    width: size.width,
    height: size.height,
    present_mode: wgpu::PresentMode::Fifo,
    alpha_mode: swapchain_capabilities.alpha_modes[0],
    view_formats: vec![],
    desired_maximum_frame_latency: 1,
  };
  surface.configure(&device, &config);

  let mut uniform = Uniform {
    screen_resolution: [size.width as f32, size.height as f32],
    time: 0.0,
    _pad0: 0.0,
  };

  let mut current_buffer = 0;
  let uniform_buffers = [
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("uniform-buffer"),
      contents: bytemuck::cast_slice(&[uniform]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    }),
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("uniform-buffer"),
      contents: bytemuck::cast_slice(&[uniform]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    }),
  ];

  let uniform_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("uniform-bind-group-layout"),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
    });

  let uniform_bind_groups = [
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("uniform-bind-group"),
      layout: &uniform_bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform_buffers[0].as_entire_binding(),
      }],
    }),
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("uniform-bind-group"),
      layout: &uniform_bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform_buffers[1].as_entire_binding(),
      }],
    }),
  ];

  let device = Arc::new(device);
  let device_clone = Arc::clone(&device);

  let create_pipeline = move || -> wgpu::RenderPipeline {
    let pipeline_layout = device_clone.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[&uniform_bind_group_layout],
      push_constant_ranges: &[],
    });

    let shader_source =
      fs::read_to_string(SHADER_SRC_PATH.get().unwrap()).expect("failed to load shader source");

    let shader = device_clone.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    device_clone.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: "vs_main",
        buffers: &[],
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: &[Some(swapchain_format.into())],
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
    })
  };

  let render_pipeline = create_pipeline();
  let render_pipeline = Arc::new(Mutex::new(render_pipeline));

  let render_pipeline_clone = Arc::clone(&render_pipeline);
  let mut watcher = notify::recommended_watcher(move |_res| {
    let mut render_pipeline = render_pipeline_clone.lock().unwrap();
    *render_pipeline = create_pipeline();
  })
  .unwrap();

  watcher
    .watch(
      SHADER_SRC_PATH.get().unwrap(),
      notify::RecursiveMode::NonRecursive,
    )
    .unwrap();

  let mut last_instant = Instant::now();

  let window = &window;
  event_loop
    .run(move |event, elwt| match event {
      Event::AboutToWait => {
        let new_instant = Instant::now();
        let tick = (new_instant - last_instant).as_secs_f32();
        uniform.time += tick;
        last_instant = new_instant;

        queue.write_buffer(
          &uniform_buffers[current_buffer],
          0,
          bytemuck::cast_slice(&[uniform]),
        );
        current_buffer = (current_buffer + 1) % 2;

        window.request_redraw();

        std::thread::sleep(Duration::from_millis(30));
      }
      Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => {
        config.width = size.width;
        config.height = size.height;
        uniform.screen_resolution = [size.width as f32, size.height as f32];
        surface.configure(&device, &config);
      }
      Event::WindowEvent {
        event: WindowEvent::RedrawRequested,
        ..
      } => {
        let frame = surface
          .get_current_texture()
          .expect("Failed to acquire next swap chain texture");
        let view = frame
          .texture
          .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
          device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let render_pipeline = render_pipeline.lock().unwrap();

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: None,
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
              store: wgpu::StoreOp::Store,
            },
          })],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
        });

        rpass.set_pipeline(&render_pipeline);
        rpass.set_bind_group(0, &uniform_bind_groups[current_buffer], &[]);
        rpass.draw(0..4, 0..1);

        drop(rpass);
        queue.submit(Some(encoder.finish()));
        frame.present();
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => elwt.exit(),
      _ => {}
    })
    .unwrap();
}
