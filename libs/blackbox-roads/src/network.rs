//! The whole road network: nodes, segments, profiles and roads of the `RNgp` group.
//! Spec: `docs/formats/road-network.md`.

use blackbox_chunk::{Chunk, ids};
use blackbox_collision::carp::{Carp, Group, tag4};

use crate::bytes::Reader;
use crate::error::malformed;
use crate::node::{NODE_LEN, RoadNode};
use crate::profile::{PROFILE_LEN, RoadProfile};
use crate::road::{ROAD_LEN, Road};
use crate::segment::{RoadSegment, SEGMENT_LEN, flags};
use crate::{Error, Result};

const PACK_ALIGN: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct RoadNetwork {
    pub nodes: Vec<RoadNode>,
    pub segments: Vec<RoadSegment>,
    pub profiles: Vec<RoadProfile>,
    pub roads: Vec<Road>,
}

impl RoadNetwork {
    /// Parses the payload of a `0x3B800` chunk (as returned by `Chunk::aligned_payload(16)`).
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let root = Carp::new(payload).root()?;
        let group = root.group(tag4(b"RNgp"))?.ok_or(Error::Missing("RNgp group"))?;
        Self::from_group(&group)
    }

    /// Parses a `0x3B800` chunk.
    pub fn from_chunk(chunk: &Chunk<'_>) -> Result<Self> {
        Self::parse(chunk.aligned_payload(PACK_ALIGN))
    }

    /// The road network in `data` (the track's world metadata file), if it has one.
    pub fn read(data: &[u8]) -> Result<Option<Self>> {
        blackbox_chunk::find(data, ids::CARP_WGRID).map(|c| Self::from_chunk(&c)).transpose()
    }

    fn from_group(group: &Group<'_>) -> Result<Self> {
        let head = group.require(tag4(b"RNhd"), "RNhd record")?;
        let h = Reader::new(head.data, "road network header");
        let counts = [h.u16(0)?, h.u16(2)?, h.u16(4)?, h.u16(8)?].map(usize::from);
        let [profiles, nodes, segments, roads] = counts;

        let table = |tag: &[u8; 4], what: &'static str, stride: usize, expected: usize| -> Result<Vec<&[u8]>> {
            let record = group.require(tag4(tag), what)?;
            let elements: Vec<&[u8]> = record.elements(stride, what)?.collect();
            if elements.len() != expected {
                return Err(malformed(what, format!("{} elements, header says {expected}", elements.len())));
            }
            Ok(elements)
        };
        let net = Self {
            profiles: table(b"RNpf", "RNpf record", PROFILE_LEN, profiles)?
                .into_iter()
                .map(|e| RoadProfile::parse(&Reader::new(e, "road profile"), 0))
                .collect::<Result<_>>()?,
            nodes: table(b"RNnd", "RNnd record", NODE_LEN, nodes)?
                .into_iter()
                .enumerate()
                .map(|(i, e)| RoadNode::parse(&Reader::new(e, "road node"), 0, i))
                .collect::<Result<_>>()?,
            segments: table(b"RNsg", "RNsg record", SEGMENT_LEN, segments)?
                .into_iter()
                .enumerate()
                .map(|(i, e)| RoadSegment::parse(&Reader::new(e, "road segment"), 0, i))
                .collect::<Result<_>>()?,
            roads: table(b"RNrd", "RNrd record", ROAD_LEN, roads)?
                .into_iter()
                .map(|e| Road::parse(&Reader::new(e, "road"), 0))
                .collect::<Result<_>>()?,
        };
        net.validate()?;
        Ok(net)
    }

    /// Every cross-reference must land inside its table, so the accessors can index freely.
    fn validate(&self) -> Result<()> {
        for (i, node) in self.nodes.iter().enumerate() {
            if usize::from(node.profile) >= self.profiles.len() {
                return Err(malformed("road node", format!("node {i}: profile {}", node.profile)));
            }
            if let Some(s) = node.segments.iter().find(|&&s| usize::from(s) >= self.segments.len()) {
                return Err(malformed("road node", format!("node {i}: segment {s}")));
            }
        }
        for (i, seg) in self.segments.iter().enumerate() {
            if let Some(n) = seg.nodes.iter().find(|&&n| usize::from(n) >= self.nodes.len()) {
                return Err(malformed("road segment", format!("segment {i}: node {n}")));
            }
            if seg.road.is_some_and(|r| usize::from(r) >= self.roads.len()) {
                return Err(malformed("road segment", format!("segment {i}: road {:?}", seg.road)));
            }
        }
        Ok(())
    }

    pub fn node(&self, index: u16) -> &RoadNode {
        &self.nodes[usize::from(index)]
    }

    pub fn segment(&self, index: u16) -> &RoadSegment {
        &self.segments[usize::from(index)]
    }

    /// The profile of `node` as read for `segment`: mirrored when the segment flags that end inverted.
    pub fn profile_at(&self, segment: u16, node: u16) -> RoadProfile {
        let seg = self.segment(segment);
        let profile = &self.profiles[usize::from(self.node(node).profile)];
        let inverted = match node == seg.nodes[0] {
            true => seg.has(flags::START_INVERTED),
            false => seg.has(flags::END_INVERTED),
        };
        match inverted {
            true => profile.inverted(),
            false => profile.clone(),
        }
    }

    /// The plain (non-decision) segment at `node` other than `not`, if any.
    pub fn plain_segment_at(&self, node: u16, not: u16) -> Option<u16> {
        self.node(node).segments.iter().copied().find(|&s| s != not && !self.segment(s).is_decision())
    }
}
