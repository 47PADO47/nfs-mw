// Hemisphere-kernel SSAO reference. Shares the GTAO geometry, static sampling,
// range limits, spatial filter and ambient-only composition for a controlled comparison.
// This is a separate estimator, not an implementation of HBAO+ or CACAO.
@compute @workgroup_size(8, 8)
fn generate_ssao(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(params.sizes.zw)) { return; }
    let pixel = ao_pixel(vec2<i32>(id.xy));
    let center = position_at(pixel);
    if center.z == 0.0 || -center.z >= params.effect.w {
        textureStore(output_ao,id.xy,vec4<f32>(1.0,0.0,0.0,0.0));
        return;
    }
    let normal = normal_at(pixel,center);
    let helper = select(vec3<f32>(0.0,0.0,1.0),vec3<f32>(1.0,0.0,0.0),abs(normal.z)>0.9);
    let tangent = normalize(cross(helper,normal));
    let bitangent = cross(normal,tangent);
    let phase = sample_phase(id.xy);
    let count = params.sampling.y * params.sampling.z * 2u;
    let radius = min(params.effect.x,-center.z * 2.0 * params.projection.y * (96.0 / 1080.0));
    var occlusion = 0.0;
    for (var i = 0u; i < count; i++) {
        // Cosine-weighted hemisphere directions, with a low-discrepancy radial sequence.
        let u = (f32(i)+0.5) / f32(count);
        let phi = 2.0 * PI * fract(phase.x + f32(i) * 0.61803398875);
        let direction = tangent * (sqrt(u)*cos(phi)) + bitangent * (sqrt(u)*sin(phi)) + normal*sqrt(1.0-u);
        let t = fract(phase.y + f32(i) * 0.56984029100);
        let probe = center + direction * radius * (0.1 + 0.9 * t * t);
        if probe.z >= -0.001 { continue; }
        let ndc = probe.xy / (-probe.z * params.projection.xy);
        let uv = vec2<f32>(ndc.x,-ndc.y) * 0.5 + 0.5;
        let coordinate = vec2<i32>(floor(uv * params.sizes.xy));
        let sample = horizon_position(coordinate,coordinate-pixel);
        if sample.z == 0.0 || sample.z <= probe.z + 0.02 { continue; }
        let delta = sample-center;
        if dot(delta,normal) <= 0.015 { continue; }
        occlusion += 1.0 - smoothstep(radius*0.35,radius,length(vec3<f32>(delta.xy,delta.z*1.3)));
    }
    let visibility = clamp(1.0-occlusion/f32(count),0.0,1.0);
    let fade = smoothstep(params.effect.z,params.effect.w,-center.z);
    textureStore(output_ao,id.xy,vec4<f32>(mix(visibility,1.0,fade),-center.z,encode_normal(normal)));
}
