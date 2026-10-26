// Authored surface families; contract: docs/specs/world-material-routing.md.
// Light/scattering are evaluated at vertices, with D3D9 COLOR saturation before interpolation.
struct WorldTerms { ambient: vec3<f32>, diffuse: vec3<f32>, scatter: vec3<f32>, };
fn world_vertex(world: vec3<f32>, normal: vec3<f32>, color: vec3<f32>) -> WorldTerms {
    var out: WorldTerms;
    out.ambient = color;
    out.diffuse = vec3<f32>(0.0);
    out.scatter = vec3<f32>(0.0);
    if WORLD_KIND == 0u || WORLD_KIND >= 3u || globals.light_dir.w < 0.5 { return out; }
    let n = normal;
    let lambert = clamp(dot(n, -globals.light_dir.xyz), 0.0, 1.0);
    var ambient = color * 0.35;
    var direct = vec3<f32>(0.65 * lambert);
    if globals.surface_ambient.w > 0.5 {
        ambient = globals.surface_ambient.xyz * globals.fog_color.w * color;
        direct = globals.surface_diffuse.xyz * lambert;
    }
    var extinction = vec3<f32>(1.0);
    if globals.scatter_rayleigh.w > 0.5 {
        let delta = globals.camera_pos.xyz - world;
        let distance = length(delta);
        let d = clamp(distance * globals.to_sun.w, 0.0, 2500.0);
        let c = dot(delta / max(distance, 1e-6), globals.to_sun.xyz);
        let br = globals.scatter_rayleigh.xyz;
        let bm = globals.scatter_mie.xyz;
        let g = globals.scatter_mie.w;
        let sum = br + bm;
        extinction = min(exp(-sum * d), vec3<f32>(1.0));
        let phase_r = 3.0 / (16.0 * PI_F) * (1.0 + c * c);
        let phase_m = (1.0 - g) * (1.0 - g) / (4.0 * PI_F * pow(max(1.0 + g*g - 2.0*g*c, 1e-6), 1.5));
        let s = (br * phase_r + bm * phase_m) * (vec3<f32>(1.0) - extinction) / max(sum, vec3<f32>(1e-9));
        out.scatter = clamp(s * globals.surface_diffuse.w, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    if WORLD_KIND == 1u { ambient *= extinction; }
    if WORLD_KIND == 2u { direct *= extinction; }
    out.ambient = clamp(ambient, vec3<f32>(0.0), vec3<f32>(1.0));
    out.diffuse = clamp(direct, vec3<f32>(0.0), vec3<f32>(1.0));
    return out;
}
fn world_pixel(in: VsOut, texel: vec4<f32>, receive_ao: bool) -> vec4<f32> {
    if WORLD_KIND >= 3u {
        let alpha = texel.a * in.color.a * select(1.0, 2.0, WORLD_KIND == 3u);
        return extended_output(vec4<f32>(2.0 * texel.rgb * in.color.rgb, alpha));
    }
    let receiver = receiver_normal(in.world, in.normal);
    if globals.fog_range.w < 0.5 && shadow_data.debug.x > 0.5 {
        return vec4<f32>(lighting_debug_colour(in.world, receiver, in.clip.xy), 1.0);
    }
    var ao = 1.0;
    if receive_ao && globals.fog_range.w < 0.5 { ao = ambient_visibility(in.clip.xy); }
    var visibility = 1.0;
    if WORLD_KIND != 2u || shadow_data.projected.x < 0.5 {
        visibility = scene_visibility(in.world, receiver, in.clip.xy, in.receives_shadow);
    }
    let light = in.world_ambient * ao + in.world_diffuse * visibility * cloud_visibility(in.world);
    let alpha = select(texel.a * in.color.a, texel.a, WORLD_KIND == 2u);
    var rgb = 2.0 * texel.rgb * light + in.world_scatter;
    if globals.scatter_rayleigh.w < 0.5 {
        let d = length(in.world - globals.camera_pos.xyz);
        let fog = clamp((d - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
        rgb = mix(rgb, globals.fog_color.rgb, fog);
    }
    return extended_output(vec4<f32>(rgb, alpha));
}

// HDR-only extension of final RGB; intermediate authored arithmetic remains intact.
fn extended_output(c:vec4<f32>)->vec4<f32> {
    if globals.camera_pos.w>1.5 { return vec4(max(c.rgb,vec3(0.0)),clamp(c.a,0.0,1.0)); }
    return clamp(c,vec4(0.0),vec4(1.0));
}
