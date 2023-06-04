use std::{
  fs,
  sync::{Arc, Mutex, OnceLock},
};

use notify::Watcher;
use wgpu::util::DeviceExt;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::Window,
};

static SHADER_SRC_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
  screen_resolution: [f32; 2],
}

async fn run(event_loop: EventLoop<()>, window: Window) {
  SHADER_SRC_PATH
    .set(std::path::PathBuf::from("./src/shader.wgsl"))
    .unwrap();

  let size = window.inner_size();

  let instance = wgpu::Instance::default();

  let surface = unsafe { instance.create_surface(&window) }.unwrap();

  let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::default(),
      force_fallback_adapter: false,
      compatible_surface: Some(&surface),
    })
    .await
    .expect("Failed to find an appropriate adapter");

  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        label: None,
        features: wgpu::Features::empty(),
        limits: wgpu::Limits::default().using_resolution(adapter.limits()),
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
  };
  surface.configure(&device, &config);

  let mut uniform = Uniform {
    screen_resolution: [size.width as f32, size.height as f32],
  };

  let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("uniform-buffer"),
    contents: bytemuck::cast_slice(&[uniform]),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
  });

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

  let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("uniform-bind-group"),
    layout: &uniform_bind_group_layout,
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: uniform_buffer.as_entire_binding(),
    }],
  });

  let device = Arc::new(device);

  let device_clone = Arc::clone(&device);

  let recreate_pipeline = move || -> wgpu::RenderPipeline {
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

  let render_pipeline = recreate_pipeline();
  let render_pipeline = Arc::new(Mutex::new(render_pipeline));

  let window = Arc::new(window);
  let window_clone = Arc::clone(&window);

  let render_pipeline_clone = Arc::clone(&render_pipeline);
  let mut watcher = notify::recommended_watcher(move |_res| {
    let mut render_pipeline = render_pipeline_clone.lock().unwrap();
    *render_pipeline = recreate_pipeline();
    window_clone.request_redraw();
  })
  .unwrap();

  watcher
    .watch(
      SHADER_SRC_PATH.get().unwrap(),
      notify::RecursiveMode::NonRecursive,
    )
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;
    match event {
      Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => {
        // Reconfigure the surface with the new size
        config.width = size.width;
        config.height = size.height;
        surface.configure(&device, &config);

        uniform.screen_resolution = [size.width as f32, size.height as f32];
        queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        // On macos the window needs to be redrawn manually after resizing
        window.request_redraw();
      }
      Event::RedrawRequested(_) => {
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
              store: true,
            },
          })],
          depth_stencil_attachment: None,
        });

        rpass.set_pipeline(&render_pipeline);
        rpass.set_bind_group(0, &uniform_bind_group, &[]);
        rpass.draw(0..4, 0..1);

        drop(rpass);
        queue.submit(Some(encoder.finish()));
        frame.present();
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,
      _ => {}
    }
  });
}

fn main() {
  let event_loop = EventLoop::new();
  let window = winit::window::Window::new(&event_loop).unwrap();
  env_logger::init();
  pollster::block_on(run(event_loop, window));
}
