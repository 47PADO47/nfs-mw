//! SDR source saturation and material alpha agree across HDR and visibility passes.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    AoQuality, AoSettings, BlendMode, DrawRange, FrameParams, GraphicsSettings, Instance, MeshDesc, NormalMaterialDesc,
    NormalModel, PixelFormat, PostSettings, Projection, RenderBackend, RenderExperiments, Shading, SurfaceLight,
    TextureDesc, TextureHandle, Tonemap, Vertex, WindowMaterialDesc,
};
use glam::{Mat4, Vec3, Vec4};

fn half(bits: u16) -> f32 {
    let sign = match bits & 0x8000 {
        0 => 1.0,
        _ => -1.0,
    };
    let mantissa = f32::from(bits & 0x03ff);
    let exponent = i32::from((bits >> 10) & 31);
    match exponent {
        0 => sign * mantissa * 2.0_f32.powi(-24),
        31 => match bits & 0x03ff {
            0 => sign * f32::INFINITY,
            _ => f32::NAN,
        },
        _ => sign * (1.0 + mantissa / 1024.0) * 2.0_f32.powi(exponent - 15),
    }
}

/// The scene texture deliberately has no COPY_SRC usage. Sample its exact texel into
/// a one-pixel float target, then copy eight bytes, without changing production usage/masks.
fn pixel(renderer: &Renderer, input: &wgpu::TextureView, depth: bool) -> [f32; 4] {
    let device = &renderer.device;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("HDR fixture texel readback"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
@group(0) @binding(0) var scene: texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let points = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(points[index],0.0,1.0);
}

@fragment fn fragment() -> @location(0) vec4<f32> {
    return textureLoad(scene,vec2<i32>(32,32),0);
}
"#
            .replace(
                "texture_2d<f32>",
                match depth {
                    true => "texture_depth_2d",
                    false => "texture_2d<f32>",
                },
            )
            .replace(
                "return textureLoad(scene,vec2<i32>(32,32),0);",
                match depth {
                    true => "return vec4(textureLoad(scene,vec2<i32>(32,32),0),0.0,0.0,1.0);",
                    false => "return textureLoad(scene,vec2<i32>(32,32),0);",
                },
            )
            .into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("HDR fixture texel readback"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("HDR fixture scene input"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input) }],
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("HDR fixture single texel"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR fixture readback"),
        size: u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HDR fixture readback"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
    );
    renderer.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let bytes = slice.get_mapped_range().unwrap();
    let pixel = std::array::from_fn(|i| half(u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]])));
    drop(bytes);
    buffer.unmap();
    pixel
}

#[derive(Clone, Copy, Debug)]
enum Family {
    Reflective,
    Window,
}

fn texture(renderer: &mut Renderer, rgba: [u8; 4]) -> TextureHandle {
    renderer.create_texture(&TextureDesc {
        label: "synthetic surface boundary texel",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&rgba],
    })
}

/// Keep independent surface depth separate from the main scene depth. Sampling the
/// latter would let the color pass repair a broken prepass and conceal the regression.
fn render_surface(
    renderer: &mut Renderer,
    family: Family,
    blend: BlendMode,
    texture_alpha: u8,
    vertex_alpha: u8,
    perspective: bool,
) {
    let diffuse = texture(renderer, [255, 255, 255, texture_alpha]);
    let normal = texture(renderer, [128, 128, 255, 255]);
    let black = texture(renderer, [0, 0, 0, 255]);
    let normal_material = match family {
        Family::Reflective => Some(
            renderer
                .create_normal_material(&NormalMaterialDesc {
                    model: NormalModel::Reflective,
                    diffuse,
                    normal,
                    specular: black,
                    reflection: Some(black),
                    uv_offset: [0.0; 2],
                })
                .expect("native reflective material"),
        ),
        Family::Window => None,
    };
    let window_material = match family {
        Family::Window => Some(
            renderer
                .create_window_material(&WindowMaterialDesc { diffuse, reflection: black })
                .expect("native window material"),
        ),
        Family::Reflective => None,
    };
    let shading = match family {
        Family::Reflective => Shading::WorldNormal(normal_material.unwrap()),
        Family::Window => Shading::WorldWindow(window_material.unwrap()),
    };
    let vertices = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]].map(|position| Vertex {
        position,
        normal: [0.0, 0.0, 1.0],
        color_bgra: [255, 255, 255, vertex_alpha],
        uv: [0.5; 2],
    });
    let tangents = [[1.0, 0.0, 0.0, 1.0]; 4];
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "synthetic opaque/glass boundary quad",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: Some(&tangents),
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(diffuse),
            blend,
            shading,
        }],
    });
    let (view, projection) = match perspective {
        true => (
            glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y),
            Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        ),
        false => (Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(0.0, 0.0, 0.5, 0.0), Vec4::W), Projection::Identity),
    };
    let frame = FrameParams {
        view,
        projection,
        camera_position: Vec3::new(0.0, 0.0, 3.0),
        light_dir: -Vec3::Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: Some(SurfaceLight {
            shadow_strength: 1.0,
            ambient: [1.0; 3],
            ambient_alpha: 1.0,
            diffuse: [1.0; 3],
            specular: [0.0; 3],
            specular_power: 1.0,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
        }),
        camera_cut: true,

        shadow_tuning: None,
    };
    renderer.render(&frame, &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    renderer.destroy_mesh(mesh);
    if let Some(material) = normal_material {
        renderer.destroy_normal_material(material);
    }
    if let Some(material) = window_material {
        renderer.destroy_window_material(material);
    }
    for handle in [diffuse, normal, black] {
        renderer.destroy_texture(handle);
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_surface_sdr_saturation_precedes_source_blending_in_hdr_intermediate() {
    let _gpu = test_support::serial();
    let mut renderer = Renderer::headless((64, 64), test_support::options()).expect("selected GPU API");
    let resolved = renderer.apply_graphics(&GraphicsSettings {
        post: PostSettings { tonemap: Tonemap::Aces, ..Default::default() },
        ..Default::default()
    });
    assert!(resolved.is_exact() && renderer.is_hdr(), "HDR fixture must be supported");
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    for family in [Family::Reflective, Family::Window] {
        for blend in [BlendMode::Opaque, BlendMode::AlphaBlend] {
            // White direct + ambient terms exceed one independently of highlights.
            // The source must saturate before blending into the floating point target.
            render_surface(&mut renderer, family, blend, 128, 0, false);
            let target = renderer.targets.offscreen().expect("HDR scene");
            assert_eq!(target.color_format, wgpu::TextureFormat::Rgba16Float);
            let actual = pixel(&renderer, &target.color_view, false);
            let alpha = 128.0 / 255.0;
            let expected_rgb = match (family, blend) {
                (Family::Window, BlendMode::AlphaBlend) => alpha,
                _ => 1.0,
            };
            for &channel in &actual[..3] {
                assert!((channel - expected_rgb).abs() < 0.004, "{family:?}/{blend:?}: {actual:?}");
            }
            if blend == BlendMode::Opaque {
                let expected_alpha = match family {
                    Family::Reflective => 1.0,
                    Family::Window => alpha,
                };
                assert!((actual[3] - expected_alpha).abs() < 0.004, "{family:?} scene alpha: {actual:?}");
            }
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn surface_visibility_prepass_obeys_original_material_alpha_with_ao_and_contacts() {
    let _gpu = test_support::serial();
    let mut renderer = Renderer::headless((64, 64), test_support::options()).expect("selected GPU API");
    renderer.apply_graphics(&GraphicsSettings {
        ao: AoSettings { quality: AoQuality::Low, ..Default::default() },
        ..Default::default()
    });
    renderer.set_prepass_depth_reuse(false);
    for (family, texture_alpha, original, visible) in [
        (Family::Reflective, 0, true, true),
        (Family::Reflective, 255, true, true),
        (Family::Reflective, 255, false, false),
        (Family::Window, 255, true, true),
        (Family::Window, 0, true, false),
        (Family::Window, 255, false, false),
    ] {
        renderer.set_render_experiments(RenderExperiments {
            original_world_materials: original,
            contact_shadows: true,
            ..Default::default()
        });
        render_surface(&mut renderer, family, BlendMode::AlphaTest, texture_alpha, 0, true);
        let surfaces = renderer.surfaces.as_ref().expect("AO/contact prepass must execute");
        let depth = pixel(&renderer, &surfaces.targets.depth, true)[0];
        assert_eq!(
            depth > 0.01,
            visible,
            "{family:?}, texture alpha={texture_alpha}, original={original}, depth={depth}"
        );
    }
}
