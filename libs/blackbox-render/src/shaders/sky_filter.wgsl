// Spatial box filtering of periodic density fields, including the one-slice weather map.
@group(0) @binding(0) var source: texture_3d<f32>;
@group(0) @binding(1) var destination: texture_storage_3d<rgba8unorm, write>;
@compute @workgroup_size(4,4,1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id >= textureDimensions(destination)) { return; }
    let extent = vec3<i32>(textureDimensions(source));
    var sum = vec4(0.0);
    for (var z=0; z<2; z++) {
        for (var y=0; y<2; y++) {
            for (var x=0; x<2; x++) {
                let p = min(vec3<i32>(id)*2+vec3(x,y,z), extent-vec3(1));
                sum += textureLoad(source, p, 0);
            }
        }
    }
    textureStore(destination, id, sum/8.0);
}
