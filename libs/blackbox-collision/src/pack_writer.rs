//! The write side of the world collision pack (`0x3B801`) for a triangle soup: a converter's ground, split into small
//! instances the way the game's own packs are (docs/formats/collision.md, "Collision packs"). What this writes reads back
//! through [`CollisionPack::parse`] to the same triangles, to the quantisation of the strips (1/128 m).
//!
//! Every instance is axis aligned (a translation only), holds one article, and the article holds one strip of three vertices
//! per triangle. A strip's vertices are 16-bit offsets from the strip's own centre, so a triangle may not span more than
//! about 250 m: [`write_soup_pack`] leaves such triangles out and says how many.

use blackbox_carp::{tag, tag4};
use blackbox_chunk::{ChunkWriter, ids};

use crate::Result;
use crate::article::{RADIUS_SCALE, STRIP_UP_FACING, VERT_SCALE};
use crate::bytes::malformed;
use crate::instance::INSTANCE_LEN;
use crate::math::Vec3;

/// Chunk payload alignment of a pack.
const PACK_ALIGN: usize = 16;
/// Bytes of the strip data of one triangle (three packed vertices) and of its sphere record.
const TRIANGLE_BYTES: usize = 3 * 8 + 16;
/// Most triangles one article can hold: its strip area size and its strip offsets are 16-bit.
pub const MAX_TRIANGLES_PER_ARTICLE: usize = (u16::MAX as usize - 16) / TRIANGLE_BYTES;
/// The largest distance from a strip's centre to one of its vertices that a packed vertex can hold, metres.
const MAX_REACH: f32 = 250.0;
/// Most surfaces an article can name (the strip's surface index is a byte).
const MAX_SURFACES: usize = 255;

/// One triangle of the ground in the game's physics space (x right, y up, z forward, metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoupTriangle {
    pub points: [Vec3; 3],
    /// The `simsurface` key of the face's surface (`docs/formats/collision.md`, "Surface types").
    pub surface: u32,
}

/// How the soup is cut into instances.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoupOptions {
    /// Most triangles an instance may hold, at most [`MAX_TRIANGLES_PER_ARTICLE`].
    pub max_triangles: usize,
    /// The widest an instance's box may be in x and z before it is split, metres: the broad phase tests the circle that
    /// holds the box, so small instances keep a query from visiting triangles far away.
    pub max_extent: f32,
}

impl Default for SoupOptions {
    fn default() -> Self {
        Self { max_triangles: 1200, max_extent: 48.0 }
    }
}

/// What [`write_soup_pack`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SoupReport {
    pub instances: usize,
    pub triangles: usize,
    /// Triangles too wide for a strip, or not finite, which are not in the pack.
    pub skipped: usize,
}

/// Writes a `0x3B801` chunk for section `section` holding `triangles` into `out`, and says what went in.
pub fn write_soup_pack(
    out: &mut ChunkWriter,
    section: u32,
    triangles: &[SoupTriangle],
    options: &SoupOptions,
) -> Result<SoupReport> {
    if options.max_triangles == 0 || options.max_triangles > MAX_TRIANGLES_PER_ARTICLE {
        return Err(malformed("pack writer", format!("{} triangles per instance", options.max_triangles)));
    }
    let (usable, skipped): (Vec<&SoupTriangle>, Vec<&SoupTriangle>) = triangles.iter().partition(|t| fits(t));
    let all: Vec<usize> = (0..usable.len()).collect();
    let mut groups = Vec::new();
    split(&usable, all, options, &mut groups);
    if groups.len() > usize::from(u16::MAX) {
        return Err(malformed("pack writer", format!("{} instances", groups.len())));
    }
    let articles: Vec<Vec<u8>> = groups.iter().map(|group| article_of(&usable, group)).collect();
    let boxes: Vec<([f32; 3], [f32; 3])> = groups.iter().map(|group| bounds_of(&usable, group)).collect();
    let blob = blob(&articles, &boxes);
    let mut payload = Vec::with_capacity(blob.len() + 16);
    for word in [blob.len() as u32, section, 0, 0] {
        payload.extend_from_slice(&word.to_le_bytes());
    }
    payload.extend_from_slice(&blob);
    out.leaf_aligned(ids::CARP_WCOLLISION_PACK, PACK_ALIGN, &payload);
    Ok(SoupReport { instances: groups.len(), triangles: usable.len(), skipped: skipped.len() })
}

/// Whether a triangle can be a strip: finite, and no corner farther than [`MAX_REACH`] from the centroid.
fn fits(triangle: &SoupTriangle) -> bool {
    if triangle.points.iter().flatten().any(|v| !v.is_finite()) {
        return false;
    }
    let centre = centroid(&triangle.points);
    triangle.points.iter().all(|p| distance(*p, centre) < MAX_REACH)
}

fn centroid(points: &[Vec3; 3]) -> Vec3 {
    let sum = |axis: usize| (points[0][axis] + points[1][axis] + points[2][axis]) / 3.0;
    [sum(0), sum(1), sum(2)]
}

fn distance(a: Vec3, b: Vec3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Cuts `ids` at the median of the longer side of their centroids until each part is small enough.
fn split(triangles: &[&SoupTriangle], mut ids: Vec<usize>, options: &SoupOptions, out: &mut Vec<Vec<usize>>) {
    if ids.is_empty() {
        return;
    }
    let (low, high) = bounds_of_ids(triangles, &ids);
    let width = [high[0] - low[0], high[2] - low[2]];
    let small = ids.len() <= options.max_triangles && width[0].max(width[1]) <= options.max_extent;
    if small || ids.len() == 1 {
        out.push(ids);
        return;
    }
    let axis = if width[0] >= width[1] { 0 } else { 2 };
    ids.sort_by(|&a, &b| centroid(&triangles[a].points)[axis].total_cmp(&centroid(&triangles[b].points)[axis]));
    let upper = ids.split_off(ids.len() / 2);
    split(triangles, ids, options, out);
    split(triangles, upper, options, out);
}

fn bounds_of_ids(triangles: &[&SoupTriangle], ids: &[usize]) -> (Vec3, Vec3) {
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for &id in ids {
        for point in triangles[id].points {
            for axis in 0..3 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
    }
    (low, high)
}

/// The centre and half extents of the box of an instance, a little larger than its triangles.
fn bounds_of(triangles: &[&SoupTriangle], ids: &[usize]) -> ([f32; 3], [f32; 3]) {
    let (low, high) = bounds_of_ids(triangles, ids);
    let centre = [0, 1, 2].map(|axis| (low[axis] + high[axis]) / 2.0);
    let half = [0, 1, 2].map(|axis| (high[axis] - low[axis]) / 2.0 + 0.1);
    (centre, half)
}

/// The bytes of one article for the triangles `ids`, in the instance's local space (centred on its box).
fn article_of(triangles: &[&SoupTriangle], ids: &[usize]) -> Vec<u8> {
    let (centre, _half) = bounds_of(triangles, ids);
    let mut surfaces: Vec<u32> = Vec::new();
    let mut spheres = Vec::with_capacity(ids.len() * 16);
    let mut strips = Vec::with_capacity(ids.len() * 24);
    for &id in ids {
        let triangle = triangles[id];
        let local = triangle.points.map(|p| [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]]);
        let middle = centroid(&local);
        let radius = local.iter().map(|p| distance(*p, middle)).fold(0.0, f32::max);
        let surface = surface_index(&mut surfaces, triangle.surface);
        let offset = ids.len() * 16 + strips.len();
        for value in middle {
            spheres.extend_from_slice(&value.to_le_bytes());
        }
        let stored = ((radius * RADIUS_SCALE).ceil() as u16).saturating_add(1);
        spheres.extend_from_slice(&stored.to_le_bytes());
        spheres.extend_from_slice(&(offset as u16).to_le_bytes());
        let flags = if faces_up(&local) { STRIP_UP_FACING } else { 0 };
        for (i, point) in local.iter().enumerate() {
            for axis in 0..3 {
                let steps = ((point[axis] - middle[axis]) * VERT_SCALE).round() as i16;
                strips.extend_from_slice(&steps.to_le_bytes());
            }
            // The first vertex carries the vertex count, the second the strip flags, the third its surface and flags.
            let tail: [u8; 2] = match i {
                0 => 3u16.to_le_bytes(),
                1 => flags.to_le_bytes(),
                _ => [surface, 0],
            };
            strips.extend_from_slice(&tail);
        }
    }
    let strips_size = (spheres.len() + strips.len()).next_multiple_of(16);
    let mut article = Vec::new();
    article.extend_from_slice(&(ids.len() as u16).to_le_bytes());
    article.extend_from_slice(&(strips_size as u16).to_le_bytes());
    article.extend_from_slice(&0u16.to_le_bytes());
    article.extend_from_slice(&0u16.to_le_bytes());
    article.extend_from_slice(&[0, surfaces.len() as u8]);
    article.extend_from_slice(&((surfaces.len() * 4) as u16).to_le_bytes());
    article.extend_from_slice(&[0; 4]);
    article.extend_from_slice(&spheres);
    article.extend_from_slice(&strips);
    article.resize(16 + strips_size, 0);
    for hash in &surfaces {
        article.extend_from_slice(&hash.to_le_bytes());
    }
    article.resize(article.len().next_multiple_of(16), 0);
    article
}

/// The index of `hash` in the article's surface table, adding it when it is new (the last slot takes the overflow).
fn surface_index(surfaces: &mut Vec<u32>, hash: u32) -> u8 {
    if let Some(at) = surfaces.iter().position(|known| *known == hash) {
        return at as u8;
    }
    if surfaces.len() >= MAX_SURFACES {
        return (MAX_SURFACES - 1) as u8;
    }
    surfaces.push(hash);
    (surfaces.len() - 1) as u8
}

/// Whether the ground query's winding test takes the triangle as seen from above: the corners go one way round in x and z.
fn faces_up(points: &[Vec3; 3]) -> bool {
    let side = |a: Vec3, b: Vec3, p: Vec3| (a[0] - b[0]) * (p[2] - b[2]) - (a[2] - b[2]) * (p[0] - b[0]);
    let middle = centroid(points);
    (0..3).all(|i| side(points[i], points[(i + 1) % 3], middle) <= 0.0)
}

/// The CARP blob: root `CARP`, one `Arti` group with `Name`, the articles `ca0..`, and the instances `ci0`.
fn blob(articles: &[Vec<u8>], boxes: &[([f32; 3], [f32; 3])]) -> Vec<u8> {
    let mut instances = Vec::with_capacity(boxes.len() * INSTANCE_LEN);
    for (index, (centre, half)) in boxes.iter().enumerate() {
        let radius = (half[0] * half[0] + half[2] * half[2]).sqrt();
        let article = (index as u16).to_le_bytes();
        let words: [[u8; 4]; 16] = [
            1.0f32.to_le_bytes(),
            0.0f32.to_le_bytes(),
            0.0f32.to_le_bytes(),
            half[0].to_le_bytes(),
            [0; 4],
            half[1].to_le_bytes(),
            [0, 0, article[0], article[1]],
            [0; 4],
            0.0f32.to_le_bytes(),
            0.0f32.to_le_bytes(),
            1.0f32.to_le_bytes(),
            half[2].to_le_bytes(),
            (-centre[0]).to_le_bytes(),
            (-centre[1]).to_le_bytes(),
            (-centre[2]).to_le_bytes(),
            radius.to_le_bytes(),
        ];
        instances.extend(words.iter().flatten());
    }
    let mut records: Vec<(u32, u32, Vec<u8>)> = vec![(tag4(b"Name"), 0, vec![0; 4])];
    for (index, article) in articles.iter().enumerate() {
        records.push((tag(*b"ca", index as u16), 0, article.clone()));
    }
    records.push((tag(*b"ci", 0), boxes.len() as u32, instances));
    carp(tag4(b"Arti"), records)
}

/// A CARP blob whose root group has one child group holding `records` (tag, element count, bytes), sorted by tag.
fn carp(child: u32, mut records: Vec<(u32, u32, Vec<u8>)>) -> Vec<u8> {
    records.sort_by_key(|record| record.0);
    let mut blob = Vec::new();
    for word in [tag4(b"CARP"), 0b10 | (1 << 5), 0, 1] {
        blob.extend_from_slice(&word.to_le_bytes());
    }
    for word in [child, 0b10, records.len() as u32, 1] {
        blob.extend_from_slice(&word.to_le_bytes());
    }
    let headers_end = blob.len() + records.len() * 16;
    let mut data = Vec::new();
    for (i, (record_tag, count, bytes)) in records.iter().enumerate() {
        let header_at = 0x20 + i * 16;
        let at = (headers_end + data.len()).next_multiple_of(16);
        data.resize(at - headers_end, 0);
        data.extend_from_slice(bytes);
        for word in [*record_tag, 0b10 | (bytes.len() as u32) << 8, *count, (at - header_at) as u32] {
            blob.extend_from_slice(&word.to_le_bytes());
        }
    }
    blob.extend_from_slice(&data);
    blob.resize(blob.len().next_multiple_of(16), 0);
    blob
}
