// World-space reference-plane sunlight transmission through the Enhanced cloud field.
struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>, scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>, to_sun: vec4<f32>, surface_ambient: vec4<f32>, surface_diffuse: vec4<f32>,
};
struct Material { intensity: vec4<f32>, options: vec4<f32>, appearance: vec4<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(3) var<uniform> material: Material;
@group(1) @binding(4) var cloud_noise: texture_3d<f32>;
@group(1) @binding(5) var cloud_sampler: sampler;
@group(1) @binding(6) var cloud_detail: texture_3d<f32>;
@group(1) @binding(7) var cloud_weather: texture_3d<f32>;

@vertex fn shadow_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
    return vec4(p[index],0.0,1.0);
}
@fragment fn shadow_fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let sun = normalize(-globals.light_dir.xyz);
    if sun.z <= 0.05 || material.options.w <= 0.005 { return vec4(1.0); }
    let xy = globals.camera_pos.xy + (pixel.xy / 256.0 - vec2(0.5)) * 8000.0;
    let entry = CLOUD_BOTTOM / sun.z;
    let step_size = CLOUD_THICKNESS / sun.z / 8.0;
    var optical_depth = 0.0;
    for (var i = 0; i < 8; i++) {
        let point = vec3(xy * 0.001, 0.0) + sun * (entry + (f32(i) + 0.5) * step_size);
        optical_depth += density(point, material.options.w, max(step_size, 0.03125)) * step_size;
    }
    let transmission = mix(1.0, exp(-10.0 * optical_depth), 0.65);
    return vec4(transmission, transmission, transmission, 1.0);
}
