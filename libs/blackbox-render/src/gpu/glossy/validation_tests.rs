//! `glossy.wgsl` parses, validates, and declares the blocks the Rust side packs.

use super::SHADER;
use super::uniforms::{MaterialUniform, RigUniform};

fn module() -> naga::Module {
    let module = naga::front::wgsl::parse_str(SHADER).unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER)));
    module
}

fn struct_size(module: &naga::Module, name: &str) -> u32 {
    module
        .types
        .iter()
        .find_map(|(_, ty)| match (&ty.name, &ty.inner) {
            (Some(n), naga::TypeInner::Struct { span, .. }) if n == name => Some(*span),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no struct {name} in glossy.wgsl"))
}

#[test]
fn the_shader_validates() {
    let module = module();
    let entries: Vec<&str> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
    for expected in ["vs_main", "fs_opaque", "fs_alpha_test", "fs_blend"] {
        assert!(entries.contains(&expected), "missing entry point {expected}: {entries:?}");
    }
}

#[test]
fn uniform_blocks_have_the_size_rust_packs() {
    let module = module();
    assert_eq!(struct_size(&module, "Rig") as usize, std::mem::size_of::<RigUniform>());
    assert_eq!(struct_size(&module, "Material") as usize, std::mem::size_of::<MaterialUniform>());
}
