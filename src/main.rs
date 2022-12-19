use pollster::block_on;
use std::{num::NonZeroU32, sync::Arc};
use wgpu::{
    util::{backend_bits_from_env, initialize_adapter_from_env_or_default},
    Adapter, Backends, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, CommandEncoderDescriptor,
    CompositeAlphaMode, ComputePassDescriptor, ComputePipelineDescriptor, Device, DeviceDescriptor,
    Extent3d, Features, ImageCopyTexture, ImageCopyTextureBase, Instance, LoadOp, Operations,
    Origin3d, PipelineLayoutDescriptor, PresentMode, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StorageTextureAccess,
    Surface, SurfaceConfiguration, SurfaceError, SurfaceTexture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

//

fn main() {
    tracing_subscriber::fmt::init();

    let events = EventLoop::new();
    let window = Arc::new(
        WindowBuilder::new()
            .with_always_on_top(true)
            .with_decorations(false)
            .with_resizable(false)
            .build(&events)
            .expect("Failed to open a window"),
    );
    window.set_cursor_hittest(true).unwrap();

    let backends = backend_bits_from_env().unwrap_or(Backends::all());
    let instance = Instance::new(backends);

    let surface = unsafe { instance.create_surface(&*window) };

    let adapter = block_on(initialize_adapter_from_env_or_default(
        &instance,
        backends,
        Some(&surface),
    ))
    .expect("No suitable GPUs");

    let (device, queue) = block_on(adapter.request_device(
        &DeviceDescriptor {
            features: Features::TEXTURE_BINDING_ARRAY | Features::STORAGE_RESOURCE_BINDING_ARRAY,
            ..Default::default()
        },
        None,
    ))
    .expect("Failed to create a device");

    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(include_str!("gui.wgsl").into()),
    });
    let group = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba8Uint,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        }],
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&group],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&layout),
        module: &module,
        entry_point: "main",
    });

    s_config(&surface, &window, &device, &adapter, None);

    events.run(move |event, _, ctrl| {
        *ctrl = ControlFlow::Wait;
        _ = &window;

        if let Some(monitor) = window.current_monitor() {
            let m_size = monitor.size();
            let w_size = window.inner_size();
            window.set_outer_position(LogicalPosition::new(
                m_size.width - w_size.width / 2,
                m_size.height - w_size.width / 2,
            ));
        }

        match event {
            Event::RedrawEventsCleared => {
                let tex = acquire(&surface, &window, &device, &adapter);
                let storage = device.create_texture(&TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: 800,
                        height: 600,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Uint,
                    usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
                });
                let storage_view = storage.create_view(&Default::default());

                let view = tex.texture.create_view(&TextureViewDescriptor {
                    ..Default::default()
                });

                let mut ce =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });

                let rp = ce.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                drop(rp);

                let bindings = device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &group,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&storage_view),
                    }],
                });

                let mut cp = ce.begin_compute_pass(&ComputePassDescriptor { label: None });
                cp.set_pipeline(&pipeline);
                cp.set_bind_group(0, &bindings, &[]);
                cp.dispatch_workgroups(100, 100, 1);
                drop(cp);

                ce.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &storage,
                        mip_level: 0,
                        origin: Origin3d { x: 0, y: 0, z: 0 },
                        aspect: TextureAspect::All,
                    },
                    ImageCopyTexture {
                        texture: &tex.texture,
                        mip_level: 0,
                        origin: Origin3d { x: 0, y: 0, z: 0 },
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: 800,
                        height: 600,
                        depth_or_array_layers: 1,
                    },
                );

                let cb = ce.finish();
                queue.submit([cb]);
                tex.present();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => s_config(&surface, &window, &device, &adapter, Some(size)),
            _ => {}
        }
    });
}

fn s_config(
    surface: &Surface,
    window: &Window,
    device: &Device,
    adapter: &Adapter,
    size: Option<PhysicalSize<u32>>,
) {
    let format = *surface
        .get_supported_formats(adapter)
        .first()
        .unwrap_or(&TextureFormat::Rgba8Unorm);
    let size = size.unwrap_or_else(|| window.inner_size());
    surface.configure(
        &device,
        &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::PostMultiplied,
        },
    );
}

fn acquire(
    surface: &Surface,
    window: &Window,
    device: &Device,
    adapter: &Adapter,
) -> SurfaceTexture {
    loop {
        match surface.get_current_texture() {
            Ok(tex) => {
                if tex.suboptimal {
                    drop(tex);
                    s_config(surface, window, device, adapter, None);
                    continue;
                }
                return tex;
            }
            Err(SurfaceError::Timeout) => {
                continue;
            }
            Err(err) => {
                tracing::debug!("{err}");
                s_config(&surface, &window, &device, adapter, None)
            }
        }
    }
}
