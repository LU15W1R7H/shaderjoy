use wgpu::util::DeviceExt;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::Window,
};

use std::borrow::Cow;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
  screen_resolution: [f32; 2],
}

async fn run(event_loop: EventLoop<()>, window: Window) {
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

  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
  });

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

  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[&uniform_bind_group_layout],
    push_constant_ranges: &[],
  });

  let swapchain_capabilities = surface.get_capabilities(&adapter);
  let swapchain_format = swapchain_capabilities.formats[0];

  let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
  });

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

  event_loop.run(move |event, _, control_flow| {
    let _ = (&instance, &adapter, &shader, &pipeline_layout);

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
        {
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
        }

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
