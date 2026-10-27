//! Two-colour paint packs as liveries.
//!
//! A 2012 vehicle with two-colour paint packs (`docs/formats/chameleon-customization.md`) is painted in a pair of colours:
//! the pack names a colour group of the car, whose first paint the body takes and whose second paint the stripes, roofs and
//! lower panels take, either on whole meshes (`TwoPaint`) or where a mask texture says (`TwoPaintMask`). The 2005 engine
//! paints a car with one flat colour or one picture over all its paint surfaces, so each such pack becomes a livery whose
//! picture is that look baked (`chameleon_vehicle::Sheet`), and the paint surfaces get the texture coordinates that pick
//! their colours out of it. The picture is opaque, so the livery replaces the paint the player picks while it is on; the
//! packs that are plain colours stay paints.

use std::collections::{BTreeSet, HashMap};

use chameleon_gfx::Texture;
use chameleon_vehicle::{Lod, Mask, PaintBlend, PaintSurface, PartMesh, Sheet, SurfaceKind, TwoColourPack};

use crate::liveries::{self, LiveryChoice};
use crate::source::{CarFile, Source, SourceCar};

/// Offer the two-colour paint packs of the car in `file` (loaded as `car`, in the folder `folder`) as its liveries: one
/// picture per pack, in the shop's order, named by the game's name of the pack. The car's paint surfaces take the texture
/// coordinates of the pictures' layout. `None`, with the car left as it was, when the car has no such pack or its paint
/// surfaces cannot be told apart (the log says why).
pub fn apply(
    source: &Source,
    file: &CarFile,
    car: &mut SourceCar,
    folder: &str,
) -> Option<Vec<(LiveryChoice, Texture)>> {
    let packs = packs_of(source, file, car)?;
    let surfaces = match source.paint_surfaces_of(file) {
        Ok(surfaces) => surfaces,
        Err(error) => {
            log::warn!("{folder}: the paint surfaces do not read, no two-colour liveries: {error:#}");
            return None;
        }
    };
    if !car.vehicle.body.iter().zip(&surfaces).all(|(level, found)| lines_up(level, found))
        || surfaces.len() != car.vehicle.body.len()
    {
        log::warn!("{folder}: the paint surfaces do not line up with the meshes, no two-colour liveries");
        return None;
    }
    let Some(sheet) = sheet_of(&surfaces, &car.textures) else {
        log::warn!("{folder}: the paint surfaces have no two-colour layout that can be carried over");
        return None;
    };
    place(&mut car.vehicle.body, &surfaces, &sheet);
    let raw: Vec<Option<String>> = packs.iter().map(|pack| source.text(pack.name)).collect();
    let names = liveries::distinct_names(&raw);
    let pictures = packs
        .iter()
        .zip(&names)
        .enumerate()
        .map(|(position, (pack, name))| {
            (liveries::named_choice(folder, position, name.as_deref()), sheet.bake(pack.first, pack.second))
        })
        .collect();
    Some(pictures)
}

/// The two-colour packs of the car's vehicle, in the shop's order; `None` when it has none.
fn packs_of(source: &Source, file: &CarFile, car: &SourceCar) -> Option<Vec<TwoColourPack>> {
    let group = source.pack_group(file)?;
    let bytes = source.bundle_bytes(file).ok()?;
    let bundle = chameleon_bundle::Bundle::parse(&bytes).ok()?;
    let pairs = chameleon_vehicle::colour_groups(&bundle).ok()?;
    let packs: Vec<TwoColourPack> = group.two_colour_packs(&pairs, &car.paints).into_iter().flatten().collect();
    (!packs.is_empty()).then_some(packs)
}

/// Whether the paint meshes of `level` are the paint surfaces `surfaces`, one by one in order (the same technique and
/// triangle count), which is how the surfaces are matched to the meshes.
fn lines_up(level: &[PartMesh], surfaces: &[PaintSurface]) -> bool {
    let meshes: Vec<&PartMesh> = level.iter().filter(|mesh| mesh.appearance.kind == SurfaceKind::Paint).collect();
    meshes.len() == surfaces.len()
        && meshes.iter().zip(surfaces).all(|(mesh, surface)| {
            mesh.appearance.technique == surface.technique && mesh.triangles.len() == surface.triangles
        })
}

/// The layout of the pictures of a car whose paint surfaces are `surfaces` (per body level): a sheet the size of the mask
/// when its surfaces blend by one, else the flat sheet. `None` when the car has no second colour on its surfaces, has a
/// livery picture on one (nothing to bake it into), blends by several masks, or the mask cannot be read.
fn sheet_of(surfaces: &[Vec<PaintSurface>], textures: &HashMap<u64, Texture>) -> Option<Sheet> {
    let all = || surfaces.iter().flatten().map(|surface| surface.blend);
    let masks: BTreeSet<u64> = all()
        .filter_map(|blend| match blend {
            PaintBlend::Mask(id) => Some(id),
            _ => None,
        })
        .collect();
    let second = surfaces.first()?.iter().any(|s| matches!(s.blend, PaintBlend::Second | PaintBlend::Mask(_)));
    if !second || masks.len() > 1 || all().any(|blend| blend == PaintBlend::Picture) {
        return None;
    }
    let Some(&id) = masks.first() else { return Some(Sheet::flat()) };
    Some(Sheet::with_mask(Mask::read(textures.get(&id)?)?))
}

/// Give the paint meshes of every level of `body` the texture coordinates of `sheet` for the colour their surface takes.
fn place(body: &mut [Lod], surfaces: &[Vec<PaintSurface>], sheet: &Sheet) {
    for (level, found) in body.iter_mut().zip(surfaces) {
        let paint = level.iter_mut().filter(|mesh| mesh.appearance.kind == SurfaceKind::Paint);
        for (mesh, surface) in paint.zip(found) {
            for vertex in &mut mesh.vertices {
                vertex.uv = sheet.uv_of(surface.blend, vertex.uv);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chameleon_gfx::{PixelFormat, Vertex};
    use chameleon_vehicle::Appearance;

    use super::*;

    fn mesh(kind: SurfaceKind, technique: &str, triangles: usize) -> PartMesh {
        PartMesh {
            vertices: vec![Vertex { position: [0.0; 3], color: [255; 4], uv: [5.0, 6.0] }; 3],
            triangles: vec![[0, 1, 2]; triangles],
            appearance: Appearance {
                kind,
                technique: technique.to_owned(),
                diffuse_texture: None,
                diffuse_colour: None,
                alpha_test: false,
            },
        }
    }

    fn surface(technique: &str, triangles: usize, blend: PaintBlend) -> PaintSurface {
        PaintSurface { technique: technique.to_owned(), triangles, blend }
    }

    /// A 16 x 16 BC1 mask of one block row repeated: white (both end points) everywhere.
    fn white_mask() -> Texture {
        let block: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0];
        Texture {
            width: 16,
            height: 16,
            depth: 1,
            array_size: 1,
            mip_levels: 1,
            dxgi_format: 71,
            format: PixelFormat::Bc1,
            data: block.repeat(16),
        }
    }

    #[test]
    fn surfaces_line_up_with_the_paint_meshes_in_order() {
        let level = vec![
            mesh(SurfaceKind::Paint, "BodyPaint_TwoPaint", 4),
            mesh(SurfaceKind::Body, "Body", 9),
            mesh(SurfaceKind::Paint, "BodyPaint", 2),
        ];
        let found = [surface("BodyPaint_TwoPaint", 4, PaintBlend::Second), surface("BodyPaint", 2, PaintBlend::First)];
        assert!(lines_up(&level, &found));
        assert!(!lines_up(&level, &found[..1]), "a surface missing");
        assert!(!lines_up(&level, &[found[1].clone(), found[0].clone()]), "in the wrong order");
        assert!(!lines_up(&level, &[found[0].clone(), surface("BodyPaint", 3, PaintBlend::First)]), "another size");
    }

    #[test]
    fn a_car_with_a_second_colour_and_no_mask_gets_the_flat_sheet() {
        let surfaces = vec![vec![surface("A", 1, PaintBlend::First), surface("B", 1, PaintBlend::Second)]];
        assert_eq!(sheet_of(&surfaces, &HashMap::new()), Some(Sheet::flat()));
    }

    #[test]
    fn a_car_with_a_mask_gets_a_sheet_of_its_size() {
        let surfaces = vec![vec![surface("A", 1, PaintBlend::First), surface("B", 1, PaintBlend::Mask(7))]];
        let sheet = sheet_of(&surfaces, &HashMap::from([(7, white_mask())])).unwrap();
        assert_eq!((sheet.width(), sheet.height()), (16, 16));
        assert_eq!(sheet_of(&surfaces, &HashMap::new()), None, "the mask is not among the textures");
    }

    #[test]
    fn a_car_with_nothing_to_tell_apart_or_a_livery_picture_has_no_sheet() {
        let plain = vec![vec![surface("A", 1, PaintBlend::First)]];
        assert_eq!(sheet_of(&plain, &HashMap::new()), None, "one colour only");
        let picture = vec![vec![surface("A", 1, PaintBlend::Second), surface("B", 1, PaintBlend::Picture)]];
        assert_eq!(sheet_of(&picture, &HashMap::new()), None, "a picture on a surface");
        let two_masks = vec![vec![surface("A", 1, PaintBlend::Mask(1)), surface("B", 1, PaintBlend::Mask(2))]];
        let textures = HashMap::from([(1, white_mask()), (2, white_mask())]);
        assert_eq!(sheet_of(&two_masks, &textures), None, "two masks");
        assert_eq!(sheet_of(&[], &HashMap::new()), None, "no surface at all");
    }

    /// Every vehicle whose packs name a pair of colours becomes a car with one livery per pack (set `MW2012_GAME_DIR`).
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_twelve_two_colour_vehicles_become_liveries() {
        let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let source = Source::open(std::path::Path::new(&root)).unwrap();
        let (mut cars, mut liveries) = (0, 0);
        for file in source.cars() {
            let named = source.pack_group(file).is_some_and(|group| group.packs.iter().any(|pack| pack.two_paint != 0));
            if !named {
                continue;
            }
            let mut car = source.load(file).unwrap();
            let Some(pictures) = apply(&source, file, &mut car, "TEST") else { continue };
            let names: BTreeSet<&str> = pictures.iter().map(|(choice, _)| choice.name.as_str()).collect();
            assert_eq!(names.len(), pictures.len(), "{}: livery names are unique", file.path.display());
            for (_, picture) in &pictures {
                assert_eq!(Some(picture.data.len()), picture.slice_size(), "{}", file.path.display());
            }
            println!("{}: {} liveries, first {:?}", file.path.display(), pictures.len(), pictures[0].0);
            cars += 1;
            liveries += pictures.len();
        }
        assert_eq!((cars, liveries), (12, 62));
    }

    #[test]
    fn the_paint_meshes_take_the_places_of_their_colours() {
        let sheet = Sheet::flat();
        let mut body = vec![vec![
            mesh(SurfaceKind::Paint, "A", 1),
            mesh(SurfaceKind::Body, "Body", 1),
            mesh(SurfaceKind::Paint, "B", 1),
            mesh(SurfaceKind::Paint, "C", 1),
        ]];
        let found = vec![vec![
            surface("A", 1, PaintBlend::First),
            surface("B", 1, PaintBlend::Second),
            surface("C", 1, PaintBlend::Mask(3)),
        ]];
        place(&mut body, &found, &sheet);
        let uvs: Vec<[f32; 2]> = body[0].iter().map(|mesh| mesh.vertices[2].uv).collect();
        assert_eq!(uvs, vec![[0.25, 0.5], [5.0, 6.0], [0.75, 0.5], [5.0, 6.0]]);
    }
}
