use pollster::block_on;
use std::{num::NonZeroU32, sync::Arc};
use wgpu::{
    util::{backend_bits_from_env, initialize_adapter_from_env_or_default},
    Adapter, Backends, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendState, BufferUsages,
    Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompositeAlphaMode,
    ComputePassDescriptor, ComputePipelineDescriptor, Device, DeviceDescriptor, Extent3d, Features,
    FragmentState, FrontFace, ImageCopyTexture, ImageCopyTextureBase, Instance, Limits, LoadOp,
    MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor, PolygonMode, PresentMode,
    PrimitiveState, PrimitiveTopology, PushConstantRange, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, SamplerBindingType, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StorageTextureAccess, Surface, SurfaceConfiguration, SurfaceError,
    SurfaceTexture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexState,
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
            .with_transparent(true)
            .build(&events)
            .expect("Failed to open a window"),
    );

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
            features: Features::PUSH_CONSTANTS,
            limits: Limits {
                max_push_constant_size: 64,
                ..Default::default()
            },
            //features: Features::TEXTURE_BINDING_ARRAY | Features::STORAGE_RESOURCE_BINDING_ARRAY,
            ..Default::default()
        },
        None,
    ))
    .expect("Failed to create a device");

    let format = s_config(&surface, &window, &device, &adapter, None);

    /* let module = device.create_shader_module(ShaderModuleDescriptor {
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
    }); */

    let blit_module = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
    });
    let blit_group = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            /* BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: false },
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            }, */
        ],
    });
    let blit_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..8,
        }],
    });
    let blit_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&blit_layout),
        vertex: VertexState {
            module: &blit_module,
            entry_point: "vs_main",
            buffers: &[],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &blit_module,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format,
                blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

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

                let view = tex.texture.create_view(&TextureViewDescriptor {
                    ..Default::default()
                });

                let mut ce =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });

                let mut rp = ce.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                let size = window.inner_size().cast::<f32>();
                #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
                #[repr(C)]
                struct Size {
                    w: f32,
                    h: f32,
                }
                let size = Size {
                    w: size.width,
                    h: size.height,
                };
                //rp.set_viewport(0.0, 0.0, size.w, size.h, 0.0, 1.0);
                rp.set_pipeline(&blit_pipeline);
                rp.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&size));
                rp.draw(0..4, 0..1);
                drop(rp);

                let cb = ce.finish();
                queue.submit([cb]);
                tex.present();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => _ = s_config(&surface, &window, &device, &adapter, Some(size)),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *ctrl = ControlFlow::Exit,
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
) -> TextureFormat {
    let format = *surface
        .get_supported_formats(adapter)
        .first()
        .unwrap_or(&TextureFormat::Rgba8Unorm);
    let size = size.unwrap_or_else(|| window.inner_size());
    let modes = surface.get_supported_alpha_modes(adapter);
    let alpha_mode = if modes.contains(&CompositeAlphaMode::Inherit) {
        CompositeAlphaMode::Inherit
    } else if modes.contains(&CompositeAlphaMode::PostMultiplied) {
        CompositeAlphaMode::PostMultiplied
    } else if modes.contains(&CompositeAlphaMode::PreMultiplied) {
        CompositeAlphaMode::PreMultiplied
    } else {
        CompositeAlphaMode::Auto
    };

    surface.configure(
        &device,
        &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT, /* | TextureUsages::COPY_DST */
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode,
        },
    );

    format
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
                s_config(&surface, &window, &device, adapter, None);
            }
        }
    }
}
