//! The compute pipelines and their bind group layouts.
//!
//! The layouts are written out here (not derived from the shaders) because textures that cannot be
//! filtered, such as `R32Float` and depth, must be declared as unfilterable.

use std::num::NonZeroU64;

use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, ComputePipeline,
    ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType,
    TextureViewDimension,
};

use crate::{
    config::Fsr3Config,
    constants::CONSTANTS_SIZE,
    shaders::{OVERRIDE_NAMES, Pass, module_source},
};

/// What a binding of a pass holds.
#[derive(Clone, Copy)]
enum Slot {
    /// A float texture read with `textureLoad`.
    Texture,
    /// A read-only storage buffer.
    BufferRead,
    /// A read-write storage buffer.
    BufferWrite,
    /// A write-only storage texture of a format.
    StoreTexture(TextureFormat),
    /// A read-write storage texture of a format.
    StoreTextureReadWrite(TextureFormat),
}

use Slot::{BufferRead, BufferWrite, StoreTexture, StoreTextureReadWrite, Texture};

/// One compute pipeline with the layout of its single bind group.
pub(crate) struct Step {
    pub layout: BindGroupLayout,
    pub pipeline: ComputePipeline,
}

/// Every pipeline of the upscaler.
pub(crate) struct Pipelines {
    pub prepare_inputs: Step,
    pub luma_reduce: Step,
    pub luma_final: Step,
    pub shading_level0: Step,
    pub shading_down: Step,
    pub shading_change: Step,
    pub prepare_reactivity: Step,
    pub luma_instability: Step,
    pub accumulate: Step,
    pub accumulate_sharpen: Step,
    pub rcas: Step,
}

fn entry(binding: u32, slot: Slot) -> BindGroupLayoutEntry {
    let ty = match slot {
        Texture => BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: false },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        },
        BufferRead => BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        BufferWrite => BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        StoreTexture(format) => BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format,
            view_dimension: TextureViewDimension::D2,
        },
        StoreTextureReadWrite(format) => BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format,
            view_dimension: TextureViewDimension::D2,
        },
    };
    BindGroupLayoutEntry { binding, visibility: ShaderStages::COMPUTE, ty, count: None }
}

fn uniform_entry() -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(CONSTANTS_SIZE as u64),
        },
        count: None,
    }
}

fn step(
    device: &Device,
    module: &ShaderModule,
    entry_point: &str,
    slots: &[(u32, Slot)],
    constants: &[(&str, f64)],
) -> Step {
    let mut entries = vec![uniform_entry()];
    entries.extend(slots.iter().map(|&(binding, slot)| entry(binding, slot)));
    let layout =
        device.create_bind_group_layout(&BindGroupLayoutDescriptor { label: Some(entry_point), entries: &entries });
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(entry_point),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(&pipeline_layout),
        module,
        entry_point: Some(entry_point),
        compilation_options: PipelineCompilationOptions { constants, ..Default::default() },
        cache: None,
    });
    Step { layout, pipeline }
}

fn module(device: &Device, pass: Pass, output_format: TextureFormat) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some(pass.file_name()),
        source: ShaderSource::Wgsl(module_source(pass, output_format).into()),
    })
}

impl Pipelines {
    pub fn new(device: &Device, config: &Fsr3Config) -> Self {
        let output = config.output_format;
        let flag = |on: bool| if on { 1.0 } else { 0.0 };
        // The values in the order of OVERRIDE_NAMES.
        let constants = |sharpen: bool| {
            let values = [
                flag(config.depth.inverted),
                flag(config.hdr),
                flag(!config.motion_vectors.display_resolution),
                flag(config.motion_vectors.jittered),
                flag(config.auto_exposure),
                flag(sharpen),
            ];
            std::array::from_fn::<_, 6, _>(|i| (OVERRIDE_NAMES[i], values[i]))
        };
        let plain = constants(false);
        let sharpen = constants(true);
        let r32 = TextureFormat::R32Float;
        let rg32 = TextureFormat::Rg32Float;
        let rgba16 = TextureFormat::Rgba16Float;

        let m = module(device, Pass::PrepareInputs, output);
        let prepare_inputs = step(
            device,
            &m,
            "cs_main",
            &[
                (1, Texture),
                (2, Texture),
                (3, Texture),
                (4, StoreTexture(rg32)),
                (5, StoreTexture(r32)),
                (6, StoreTexture(r32)),
                (7, StoreTexture(r32)),
                (8, BufferWrite),
            ],
            &plain,
        );

        let m = module(device, Pass::LumaPyramid, output);
        let luma_reduce = step(
            device,
            &m,
            "cs_reduce",
            &[(1, Texture), (2, Texture), (3, StoreTexture(r32)), (4, BufferWrite)],
            &plain,
        );
        let luma_final = step(device, &m, "cs_final", &[(4, BufferWrite), (5, BufferWrite), (6, Texture)], &plain);

        let m = module(device, Pass::ShadingChangePyramid, output);
        let shading_level0 = step(
            device,
            &m,
            "cs_level0",
            &[(1, Texture), (2, Texture), (3, Texture), (4, BufferRead), (5, StoreTexture(rg32))],
            &plain,
        );
        let shading_down = step(device, &m, "cs_down", &[(5, StoreTexture(rg32)), (6, Texture)], &plain);

        let m = module(device, Pass::ShadingChange, output);
        let shading_change =
            step(device, &m, "cs_main", &[(1, Texture), (2, Texture), (3, Texture), (4, StoreTexture(r32))], &plain);

        let m = module(device, Pass::PrepareReactivity, output);
        let prepare_reactivity = step(
            device,
            &m,
            "cs_main",
            &[
                (1, Texture),
                (2, Texture),
                (3, BufferRead),
                (4, Texture),
                (5, Texture),
                (6, Texture),
                (7, Texture),
                (8, Texture),
                (9, BufferRead),
                (10, StoreTexture(rgba16)),
                (11, StoreTexture(r32)),
                (12, StoreTextureReadWrite(r32)),
            ],
            &plain,
        );

        let m = module(device, Pass::LumaInstability, output);
        let luma_instability = step(
            device,
            &m,
            "cs_main",
            &[
                (1, Texture),
                (2, Texture),
                (3, Texture),
                (4, Texture),
                (5, BufferRead),
                (6, StoreTexture(rgba16)),
                (7, StoreTexture(r32)),
            ],
            &plain,
        );

        let m = module(device, Pass::Accumulate, output);
        let accumulate_slots = [
            (1, Texture),
            (2, Texture),
            (3, Texture),
            (4, Texture),
            (5, Texture),
            (6, Texture),
            (7, Texture),
            (8, BufferRead),
            (9, StoreTexture(rgba16)),
            (10, StoreTexture(output)),
            (11, StoreTextureReadWrite(r32)),
        ];
        let accumulate = step(device, &m, "cs_main", &accumulate_slots, &plain);
        let accumulate_sharpen = step(device, &m, "cs_main", &accumulate_slots, &sharpen);

        let m = module(device, Pass::Rcas, output);
        let rcas = step(device, &m, "cs_main", &[(1, Texture), (2, BufferRead), (3, StoreTexture(output))], &sharpen);

        Self {
            prepare_inputs,
            luma_reduce,
            luma_final,
            shading_level0,
            shading_down,
            shading_change,
            prepare_reactivity,
            luma_instability,
            accumulate,
            accumulate_sharpen,
            rcas,
        }
    }
}
