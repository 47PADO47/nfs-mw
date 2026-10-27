use chameleon_bundle::Import;

use crate::error::Result;

/// Samplers the engine binds for every draw; they are not part of a material's own texture list.
const GLOBAL_SAMPLERS: [&str; 5] = [
    "EnvironmentSpecularTextureSampler",
    "cubeReflectionSampler",
    "TonemapSampler",
    "AmbientSpecSampler",
    "cubeIrradianceSampler",
];

/// What a material needs to know about its shader: the vertex layout it draws with, the order its samplers take the
/// material's textures in, and the names the engine gives its technique and parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shader {
    /// The `VertexDescriptor` resource the shader's input layout imports, when it has one.
    pub vertex_descriptor: Option<u64>,
    /// Every NUL-terminated identifier in the block, in file order.
    pub names: Vec<String>,
    /// The samplers that take the material's textures, in the order the material lists them.
    pub material_samplers: Vec<String>,
}

impl Shader {
    pub fn parse(main: &[u8], imports: &[Import], is_descriptor: impl Fn(u64) -> bool) -> Result<Self> {
        let vertex_descriptor = imports.iter().map(|import| import.id).find(|&id| is_descriptor(id));
        let found = identifiers(main);
        let material_samplers = sampler_table(main, &found);
        let names = found.into_iter().map(|(_, name)| name).collect();
        Ok(Self { vertex_descriptor, names, material_samplers })
    }

    /// The technique name, the last name that contains an underscore and starts with a capital letter.
    pub fn technique(&self) -> Option<&str> {
        self.names
            .iter()
            .rev()
            .find(|name| name.contains('_') && name.starts_with(|c: char| c.is_ascii_uppercase()))
            .map(String::as_str)
    }

    pub fn has_name(&self, name: &str) -> bool {
        self.names.iter().any(|candidate| candidate == name)
    }

    /// Which of the material's textures the sampler `name` takes.
    pub fn sampler_slot(&self, name: &str) -> Option<usize> {
        self.material_samplers.iter().position(|sampler| sampler == name)
    }
}

/// The identifiers of the block with their offsets: runs of letters, digits and underscores of five or more
/// characters that start with a letter and end at a NUL.
fn identifiers(data: &[u8]) -> Vec<(usize, String)> {
    let is_ident = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let mut out = Vec::new();
    let mut start = None;
    for (at, &byte) in data.iter().enumerate() {
        match (is_ident(byte), start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                start = None;
                let word = &data[from..at];
                if at - from >= 5 && word[0].is_ascii_alphabetic() && byte == 0 {
                    out.push((from, String::from_utf8_lossy(word).into_owned()));
                }
            }
            _ => {}
        }
    }
    out
}

/// The first run of consecutive 32-bit words that all point at sampler names and are not the engine's own samplers.
fn sampler_table(data: &[u8], names: &[(usize, String)]) -> Vec<String> {
    let sampler_at = |offset: u32| names.iter().find(|(at, name)| *at == offset as usize && name.ends_with("Sampler"));
    let mut runs: Vec<Vec<String>> = vec![Vec::new()];
    for &word in data.as_chunks::<4>().0 {
        let offset = u32::from_le_bytes(word);
        match sampler_at(offset) {
            Some((_, name)) => runs.last_mut().expect("runs is never empty").push(name.clone()),
            None if runs.last().is_some_and(|run| !run.is_empty()) => runs.push(Vec::new()),
            None => {}
        }
    }
    let own = |run: &&Vec<String>| !run.is_empty() && run.iter().all(|name| !GLOBAL_SAMPLERS.contains(&name.as_str()));
    runs.iter().find(own).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> Vec<u8> {
        // names at 0x10, 0x28; a pointer table at 0x40 of [normal, diffuse] then a gap, then the global sampler.
        let mut data = vec![0u8; 0x80];
        data[0x10..0x10 + 21].copy_from_slice(b"NormalTextureSampler\0");
        data[0x28..0x28 + 22].copy_from_slice(b"DiffuseTextureSampler\0");
        data[0x40..0x44].copy_from_slice(&0x10u32.to_le_bytes());
        data[0x44..0x48].copy_from_slice(&0x28u32.to_le_bytes());
        data[0x60..0x68].copy_from_slice(b"TonemapSampler\0"[..8].try_into().unwrap());
        data
    }

    #[test]
    fn material_samplers_follow_the_pointer_table_order() {
        let shader = Shader::parse(&block(), &[], |_| false).unwrap();
        assert_eq!(shader.material_samplers, ["NormalTextureSampler", "DiffuseTextureSampler"]);
        assert_eq!(shader.sampler_slot("DiffuseTextureSampler"), Some(1));
        assert_eq!(shader.sampler_slot("SpecularTextureSampler"), None);
    }

    #[test]
    fn finds_nul_terminated_names() {
        let found = identifiers(b"\x01\x02PbrDiffuse\0xx\0DiffuseTextureSampler\0VehicleNFS_Body\0");
        let names: Vec<_> = found.iter().map(|(_, name)| name.as_str()).collect();
        assert_eq!(names, ["PbrDiffuse", "DiffuseTextureSampler", "VehicleNFS_Body"]);
    }
}
