//! `CarpWCollisionPack` (`0x3B801`): the static collision of one streaming section.
//! Spec: `docs/formats/collision.md` ("Collision packs").

use blackbox_chunk::{Chunk, ids};

use crate::bytes::{Reader, malformed};
use crate::carp::{Carp, tag, tag4};
use crate::instance::{INSTANCE_LEN, Instance};
use crate::object::{OBJECT_LEN, Object};
use crate::{Article, Error, Result};

/// Size of the `bChunkCarpHeader` in front of the blob.
const CARP_HEADER_LEN: usize = 16;
/// Chunk payload alignment of the pack.
const PACK_ALIGN: usize = 16;
/// Runtime exclusion bits `SetExclusionFlags` puts on scenery-group geometry.
pub const GROUP_EXCLUSION: u8 = 0xC0;

#[derive(Debug, Clone, PartialEq)]
pub struct CollisionPack {
    /// The streaming section the pack belongs to.
    pub section: u32,
    pub instances: Vec<Instance>,
    /// Indexed by [`Instance::article`].
    pub articles: Vec<Article>,
    pub objects: Vec<Object>,
}

impl CollisionPack {
    /// Parses the payload of a `0x3B801` chunk (as returned by `Chunk::aligned_payload(16)`).
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let header = Reader::new(payload, "CARP chunk header");
        let size = usize::try_from(header.u32(0)?).unwrap_or(usize::MAX);
        let section = header.u32(4)?;
        let blob = header.slice(CARP_HEADER_LEN, size)?;

        let root = Carp::new(blob).root()?;
        let arti = root.group(tag4(b"Arti"))?.ok_or(Error::Missing("Arti group"))?;

        let instances = match arti.record(tag(*b"ci", 0))? {
            Some(rec) => rec
                .elements(INSTANCE_LEN, "instance record")?
                .map(|e| Instance::decode(&Reader::new(e, "collision instance"), 0))
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        let objects = match arti.record(tag(*b"co", 0))? {
            Some(rec) => rec
                .elements(OBJECT_LEN, "object record")?
                .map(|e| Object::decode(&Reader::new(e, "collision object"), 0))
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        let mut articles = Vec::new();
        for (i, rec) in arti.records().enumerate() {
            let rec = rec?;
            if rec.tag >> 16 == tag(*b"ca", 0) >> 16 {
                let index = usize::from((rec.tag & 0xFFFF) as u16);
                if index != articles.len() {
                    return Err(malformed("collision pack", format!("article record {i} has index {index}")));
                }
                articles.push(Article::parse(rec.data)?);
            }
        }
        for (i, inst) in instances.iter().enumerate() {
            if usize::from(inst.article) >= articles.len() {
                return Err(malformed("collision pack", format!("instance {i} uses article {}", inst.article)));
            }
        }
        Ok(Self { section, instances, articles, objects })
    }

    /// Parses a `0x3B801` chunk.
    pub fn from_chunk(chunk: &Chunk<'_>) -> Result<Self> {
        Self::parse(chunk.aligned_payload(PACK_ALIGN))
    }

    /// The article of instance `index`.
    pub fn article_of(&self, index: usize) -> Option<&Article> {
        self.articles.get(usize::from(self.instances.get(index)?.article))
    }

    /// What the game does after loading a pack: instances in a scenery group (`group != 0`) that
    /// have no strips get [`GROUP_EXCLUSION`] in their flags, and every barrier of a grouped
    /// instance's article gets it in its flags, so queries with that exclusion bit skip them.
    pub fn apply_group_exclusion(&mut self) {
        for inst in &mut self.instances {
            if inst.group == 0 {
                continue;
            }
            let Some(article) = self.articles.get_mut(usize::from(inst.article)) else { continue };
            if article.strips.is_empty() {
                inst.flags |= u16::from(GROUP_EXCLUSION);
            }
            for b in &mut article.barriers {
                b.flags |= GROUP_EXCLUSION;
            }
        }
    }
}

/// Every collision pack in `data` (a streaming file), in file order.
pub fn read_collision_packs(data: &[u8]) -> Result<Vec<CollisionPack>> {
    blackbox_chunk::find_all(data, ids::CARP_WCOLLISION_PACK).iter().map(CollisionPack::from_chunk).collect()
}
