// Independent analytic atmosphere and shallow cloud volume. See docs/enhanced-sky.md.
// All distances in the cloud integration are kilometres; the scene is Z-up.
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
@group(2) @binding(0) var cloud_map: texture_2d<f32>;
@group(2) @binding(1) var cloud_map_sampler: sampler;

struct Input {
    @location(0) position: vec3<f32>,
    @location(4) model_0: vec4<f32>, @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>, @location(7) model_3: vec4<f32>,
};
struct Output {
    @builtin(position) clip: vec4<f32>,
    @location(0) direction: vec3<f32>,
};

@vertex fn vs_main(v: Input) -> Output {
    let model = mat4x4(v.model_0, v.model_1, v.model_2, v.model_3);
    let world = model * vec4(v.position, 1.0);
    var out: Output;
    out.clip = globals.view_proj * world;
    out.direction = world.xyz - globals.camera_pos.xyz;
    // Projected sky placement mirrors Z and can carry its own vertical offset.
    // Recover the unmirrored direction without turning that offset into parallax.
    if v.model_2.z < 0.0 {
        out.direction = vec3(world.xy-v.model_3.xy, v.model_3.z-world.z);
    }
    return out;
}

fn sun_direction() -> vec3<f32> {
    if length(globals.to_sun.xyz) > 0.01 { return normalize(globals.to_sun.xyz); }
    return normalize(-globals.light_dir.xyz);
}

fn sun_colour(sun: vec3<f32>) -> vec3<f32> {
    // Longer air path reddens low sunlight. Match the same sun used by scene lighting.
    let air = 1.0 / max(sun.z + 0.12, 0.12);
    let low_sun = 1.0 - smoothstep(0.13, 0.42, sun.z);
    return exp(-vec3(0.035, 0.075, 0.15) * air) * pow(vec3(1.12, 0.72, 0.43), vec3(low_sun * material.appearance.w));
}

fn sky_linear(colour: vec3<f32>) -> vec3<f32> {
    let c = max(colour, vec3(0.0));
    return select(pow((c+0.055)/1.055, vec3(2.4)), c/12.92, c <= vec3(0.04045));
}

fn sky_encoded(colour: vec3<f32>) -> vec3<f32> {
    let c = max(colour, vec3(0.0));
    return select(1.055*pow(c, vec3(1.0/2.4))-0.055, c*12.92, c <= vec3(0.0031308));
}

fn atmosphere(ray: vec3<f32>, sun: vec3<f32>, coverage: f32) -> vec3<f32> {
    let mu = clamp(dot(ray, sun), -1.0, 1.0);
    let horizon = pow(1.0 - max(ray.z, 0.0), 5.0);
    let daylight = smoothstep(-0.12, 0.18, sun.z);
    let dusk = 1.0 - smoothstep(0.13, 0.42, sun.z);
    let air = 1.0 / max(ray.z + 0.16, 0.16);
    let extinction = exp(-vec3(0.045, 0.10, 0.22) * air);
    let rayleigh = 0.75 * (1.0 + mu * mu);
    let sky = vec3(0.24, 0.42, 0.72) * (vec3(1.0) - extinction) * vec3(4.8, 3.6, 2.5) * rayleigh;
    let fog = sky_linear(globals.fog_color.rgb);
    let sunward = smoothstep(-0.35, 0.85, dot(normalize(vec3(ray.xy, 0.001)), normalize(vec3(sun.xy, 0.001))));
    let warm_horizon = mix(fog, vec3(0.82, 0.30, 0.09), clamp(dusk * mix(0.30, 0.85, sunward) * material.appearance.w, 0.0, 1.0));
    var colour = mix(sky, warm_horizon, horizon * 0.85);
    // A forward-scattering haze halo, separate from the solar disk.
    let mie = 0.035 * (1.0 - 0.76 * 0.76) / pow(max(1.0 + 0.76 * 0.76 - 1.52 * mu, 0.01), 1.5);
    colour += sun_colour(sun) * mie * (0.4 + 0.6 * horizon);
    colour = mix(colour, vec3(dot(colour, vec3(0.24, 0.56, 0.20))) * vec3(0.90, 0.97, 1.05), coverage * 0.55);
    colour = mix(vec3(0.008, 0.013, 0.027), colour, daylight);
    return mix(colour, fog, 1.0 - smoothstep(-0.12, 0.03, ray.z));
}

fn solar_disk(ray: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
    let mu = clamp(dot(ray, sun), -1.0, 1.0);
    // Baseline 0.53 degree disk; default size 2 gives a more readable 1.06 degrees.
    let radius = 0.00465 * material.appearance.x;
    let angle = acos(clamp(mu, -0.9999999, 0.9999999));
    let pixel = max(fwidth(angle), 0.00015);
    let disk = 1.0 - smoothstep(radius - pixel, radius + pixel, angle);
    return sun_colour(sun) * disk * 7.0 * material.appearance.y
        * smoothstep(-0.12, 0.18, sun.z) * smoothstep(-0.12, 0.03, ray.z);
}

fn sky_output(background: vec3<f32>, disk: vec3<f32>) -> vec3<f32> {
    if globals.camera_pos.w > 1.5 { return sky_encoded(background + disk); }
    // Reserve SDR highlight contrast for the solar disk instead of clipping both
    // bright clouds/haze and the disk to the same white. Preserve low/mid tones.
    let peak = max(background.r, max(background.g, background.b));
    var mapped = background;
    if peak > 0.6 {
        let shoulder = 0.6 + 0.3 * (1.0 - exp(-(peak-0.6)/0.3));
        mapped *= shoulder / peak;
    }
    return clamp(sky_encoded(mapped + disk), vec3(0.0), vec3(1.0));
}

fn clouds(ray: vec3<f32>, sun: vec3<f32>, coverage: f32, pixel: vec2<f32>) -> vec4<f32> {
    if coverage < 0.005 || ray.z < 0.015 { return vec4(0.0, 0.0, 0.0, 1.0); }
    let origin = vec3(globals.camera_pos.xy * 0.001, clamp(globals.camera_pos.z * 0.001, -0.2, 0.8));
    let entry = (CLOUD_BOTTOM - origin.z) / ray.z;
    let exit = min((CLOUD_BOTTOM + CLOUD_THICKNESS - origin.z) / ray.z, 128.0);
    if exit <= entry { return vec4(0.0, 0.0, 0.0, 1.0); }
    let step_size = (exit - entry) / 96.0;
    // Fixed screen-space phase breaks slab bands without frame-varying seeds or history.
    let phase = fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
    var transmittance = 1.0;
    var radiance = vec3(0.0);
    let daylight = smoothstep(-0.12, 0.18, sun.z);
    let sky_fill = mix(vec3(0.48, 0.56, 0.68), vec3(0.48, 0.51, 0.55), coverage);
    let alignment = dot(ray, sun);
    for (var i = 0u; i < 96u; i++) {
        let p = origin + ray * (entry + (f32(i) + phase) * step_size);
        let d = density(p, coverage);
        if d > 0.001 {
            var optical_depth = 0.0;
            for (var light_step = 0u; light_step < 6u; light_step++) {
                optical_depth += density(p + sun * ((f32(light_step)+0.5)*(CLOUD_THICKNESS/6.0)), coverage) * (CLOUD_THICKNESS/6.0);
            }
            let direct = cloud_scattering(optical_depth, alignment);
            let height = clamp((p.z-CLOUD_BOTTOM)/CLOUD_THICKNESS, 0.0, 1.0);
            let light = sky_fill * mix(0.3, 0.6, height) + sun_colour(sun)*direct*5.0;
            let opacity = 1.0-exp(-d*step_size*10.0);
            radiance += transmittance * opacity * light * daylight;
            transmittance *= 1.0 - opacity;
        }
        if transmittance < 0.015 { break; }
    }
    // Distant clouds dissolve into the same horizon haze instead of forming a hard ring.
    let visibility = 1.0 - smoothstep(32.0, 128.0, entry);
    return vec4(radiance * visibility, mix(1.0, transmittance, visibility));
}

@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
    let ray = normalize(in.direction);
    let sun = sun_direction();
    let coverage = material.options.w;
    let background = atmosphere(ray, sun, coverage);
    // The native scene and reflection contracts are display-encoded, including float targets.
    // Integrate radiance in linear light, then encode exactly once at this boundary.
    let uv = vec2(atan2(ray.y, ray.x)/6.2831853+0.5, asin(clamp(ray.z, 0.0, 1.0))/1.5707963);
    var cloud = textureSampleLevel(cloud_map, cloud_map_sampler, uv, 0.0);
    if ray.z < 0.015 { cloud = vec4(0.0, 0.0, 0.0, 1.0); }
    let brightness = material.appearance.z;
    let colour = sky_output((background * cloud.a + cloud.rgb) * brightness, solar_disk(ray, sun) * cloud.a * brightness);
    return vec4(colour, 1.0);
}

// One shared upper hemisphere, independent of camera direction and output resolution.
// Keep the disk and atmosphere out of this map so they remain sharp in every view.
@vertex fn cache_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4(p*2.0-1.0, 0.0, 1.0);
}

@fragment fn cache_fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pixel.xy / vec2(1536.0, 384.0);
    let azimuth = (uv.x-0.5)*6.2831853;
    let elevation = uv.y*1.5707963;
    let ray = vec3(cos(azimuth)*cos(elevation), sin(azimuth)*cos(elevation), sin(elevation));
    return clouds(ray, sun_direction(), material.options.w, pixel.xy);
}
