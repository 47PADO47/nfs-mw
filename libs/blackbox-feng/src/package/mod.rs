//! Reading an FEng package: header, resources, objects with their scripts and message responses, and the
//! package-level responses and message targets. Format: `docs/formats/frontend.md`.

mod object;
mod raw;
mod response;
mod script;
mod types;

pub use raw::{Chunk, Tag, chunks, tags};
pub use types::*;

use crate::error::{Error, Result};
use crate::hash::resource_handle;
use raw::read_u32;

/// A parsed package. Plain data; the runtime copies what it animates.
#[derive(Clone, Debug)]
pub struct Package {
    /// Short name, e.g. `HUD_SingleRace.fng`.
    pub name: String,
    /// The source path stored in the file.
    pub file_name: String,
    pub version: u32,
    pub resources: Vec<Resource>,
    /// All objects, parents before children.
    pub objects: Vec<ObjectDef>,
    pub button_count: u32,
    /// Package-level responses (`PkgR`).
    pub responses: Vec<ResponseList>,
    pub targets: Vec<MessageTargets>,
}

impl Package {
    /// Parses the payload of an `FEngPackage` chunk (`0x00030203`).
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let top = chunks(payload, "package")?;
        let root = top.iter().find(|c| c.is(b"FEng") && c.nested).ok_or(Error::NotAPackage("no FEng chunk"))?;
        let mut package = Package {
            name: String::new(),
            file_name: String::new(),
            version: 0,
            resources: Vec::new(),
            objects: Vec::new(),
            button_count: 0,
            responses: Vec::new(),
            targets: Vec::new(),
        };
        for chunk in chunks(root.data, "FEng")? {
            match &chunk.id {
                b"PkHd" => package.read_header(chunk.data)?,
                b"ResL" => package.read_resources(chunk.data)?,
                b"ObjL" => package.read_objects(chunk.data)?,
                b"PkgR" => package.responses = response::read_response_lists(chunk.data, "PkgR")?,
                b"Targ" => package.targets = response::read_targets(chunk.data)?,
                _ => {}
            }
        }
        if package.version == 0 {
            return Err(Error::NotAPackage("no header"));
        }
        package.link_resources();
        Ok(package)
    }

    /// Parses the payload of an `FEngCompressedPackage` chunk (`0x00030210`): the hash of the package name,
    /// then a compressed package. Returns the hash and the package.
    pub fn parse_compressed(payload: &[u8]) -> Result<(u32, Self)> {
        let hash = read_u32(payload, 0).ok_or(Error::Truncated("compressed package"))?;
        let inner = ea_compress::unwrap(&payload[4..]).map_err(|e| Error::Decompress(e.to_string()))?;
        Ok((hash, Self::parse(&inner)?))
    }

    fn read_header(&mut self, data: &[u8]) -> Result<()> {
        let word = |i: usize| read_u32(data, i * 4).ok_or(Error::Truncated("PkHd"));
        self.version = word(0)?;
        if self.version < 0x20000 {
            return Err(Error::OldVersion(self.version));
        }
        let (short, long) = (word(4)? as usize, word(5)? as usize);
        let names = data.get(24..).unwrap_or_default();
        let cstr = |range: Option<&[u8]>| {
            let bytes = range.unwrap_or_default();
            let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        };
        self.name = cstr(names.get(..short.min(names.len())));
        self.file_name = cstr(names.get(short.min(names.len())..(short + long).min(names.len())));
        Ok(())
    }

    fn read_resources(&mut self, data: &[u8]) -> Result<()> {
        let mut names: &[u8] = &[];
        let mut requests: &[u8] = &[];
        for c in chunks(data, "ResL")? {
            match &c.id {
                b"RsNm" => names = c.data,
                b"RsRq" => requests = c.data,
                _ => {}
            }
        }
        let count = read_u32(requests, 0).unwrap_or(0) as usize;
        for i in 0..count {
            let at = 4 + i * 24;
            let rec = requests.get(at..at + 24).ok_or(Error::Truncated("RsRq"))?;
            let word = |k: usize| u32::from_le_bytes(rec[k * 4..k * 4 + 4].try_into().unwrap_or([0; 4]));
            let offset = (word(1) as usize).min(names.len());
            let tail = &names[offset..];
            let end = tail.iter().position(|b| *b == 0).unwrap_or(tail.len());
            let name = String::from_utf8_lossy(&tail[..end]).into_owned();
            let kind = match word(2) {
                1 => ResourceKind::Image,
                2 => ResourceKind::Font,
                7 => ResourceKind::MultiImage,
                other => ResourceKind::Other(other),
            };
            self.resources.push(Resource { handle: resource_handle(&name), name, kind, flags: word(3) });
        }
        Ok(())
    }

    fn read_objects(&mut self, data: &[u8]) -> Result<()> {
        for c in chunks(data, "ObjL")? {
            match &c.id {
                b"Butn" => self.button_count = read_u32(c.data, 0).unwrap_or(0),
                b"FObj" => {
                    if let Some(obj) = object::read_object(c.data)? {
                        self.objects.push(obj);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Objects keep a resource index; make it `None` when it points outside the list.
    fn link_resources(&mut self) {
        let n = self.resources.len();
        for o in &mut self.objects {
            if o.resource.is_some_and(|i| i >= n) {
                o.resource = None;
            }
        }
    }

    /// Index of the first object with this name hash.
    pub fn find_by_hash(&self, name_hash: u32) -> Option<usize> {
        self.objects.iter().position(|o| o.name_hash == name_hash)
    }

    pub fn find_by_guid(&self, guid: u32) -> Option<usize> {
        self.objects.iter().position(|o| o.guid == guid)
    }

    /// Responses of the package to `message`.
    pub fn responses_to(&self, message: u32) -> Option<&ResponseList> {
        self.responses.iter().find(|r| r.message == message)
    }

    /// The objects that receive `message` when it is sent to every package.
    pub fn targets_of(&self, message: u32) -> &[u32] {
        self.targets.iter().find(|t| t.message == message).map_or(&[], |t| &t.guids)
    }
}

#[cfg(test)]
pub(crate) mod synth;
#[cfg(test)]
mod tests;
