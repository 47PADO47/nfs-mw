# RIDE 6 tracks

What a track of RIDE 6 (Milestone, Unreal Engine 5.6.1, build 1697004) is made of in the install, found on `D:\RIDE 6`
with the console tool of `tools/ride6-exporter` of the branch `feat/ride6-parts-liveries` (CUE4Parse, the game's `.usmap`; see
that exporter's README) and with the viewers `tools/glb_merge.py` and `tools/glb_merge_preview.py`.
Materials of bikes: [ride6-materials.md](ride6-materials.md). Plan: [plans/tracks-and-fairhaven-feasibility.md](../plans/tracks-and-fairhaven-feasibility.md).
Evidence tags: [README](../README.md#evidence-tags). Everything below was measured on Okayama unless it says otherwise; no
game data is in the repository (the exports are in `C:\r6work\tracks\`).

## The install **[verified]**

The IoStore containers (`ride6/Content/Paks/*.utoc` and `.ucas`, 53 GB in 179 files, the 45 `.pak` files have an encrypted index and are not needed for tracks; the audio banks are in them)
open without a key. 134,905 files; 1,075 `.umap` levels, of which 1,002 are under `maps/Tracks/`. Reading a package needs
the mapping file (`.usmap`): the assets use unversioned properties and nothing parses without it. The data is cooked UE5:
meshes are serialized render data (Nanite or distance-field data may be present: not checked), textures are BC1 to BC7 inside Oodle blocks. No
Rust crate reads this; CUE4Parse (C#, Apache-2.0) does.

## The tracks **[verified]**

41 folders under `maps/Tracks/` (two are not circuits: `Common` with 2 levels and `Dummies` with 45 test levels), so **39 map
folders**. The number is the levels of each:

Autopolis 25, BlueWaveArena 31, BlueWaveCircuit 30, BlueWaveEnduro 36, BrandsHatch 25, CadwellPark 23, Canadian 19,
CappadociaTrails 31, Daytona 27, Donington 26, FrenchRiviera 25, Fuji 22, GunmaMountainCircuit 29, Imola 22, Kanto 28,
Kyalami 21, Lagunaseca 19, LeMans 20, Magnycours 21, Monza 20, Mugello 30, Nevada 25, Nordschleife 20, NorthWest200 19,
NurburgringGP 25, Okayama 26, Portimao 23, RedBullRing 26, RoadAmerica 22, RoadAtlanta 28, Southern100 19, SpainCircuits
20, Stelvio 25, StelvioPark 31, Sugo 22, Suzuka 25, UlsterGP 18, Valencia 25, Vallelunga 25.

Circuits (Imola, Monza, Suzuka...), road courses (Nordschleife, Isle of Man's Southern 100 and NorthWest 200, Ulster GP),
mountain and open maps (Stelvio, Kanto, Nevada, French Riviera, Cappadocia Trails) and the Blue Wave arenas. The track art
lives in `assets/graphics/Env/<Track>/` (Imola 1,607 files, Suzuka 1,564, Monza 1,518, Okayama 843, StelvioPark 323...) and in
the shared sets `ObjShared` (7,267 files), `VegShared` (2,023) and `EnvShared` (1,151). The cars' own packages are separate
([ride6-bikes.md](../modding/ride6-bikes.md)). Menu scenes (`maps/Menu/*`, the dealership, motorhome, podium) are levels too.

## A track is a persistent level and about 25 sub-levels **[verified]**

`maps/Tracks/<Track>/<Track>.umap` holds almost nothing (46 objects in Okayama: a `World`, 30 `LevelStreamingDynamic`
entries); the content is in `Game/*.umap`, streamed by name. Okayama's 26 levels and what is in them
(`StaticMeshActor` count, instances of instanced and foliage meshes, distinct meshes):

| Sub-level | What | Actors, instances |
|---|---|---|
| `Terrain` | ground slices, kerbs, painted lines, the "runnable" mesh, far terrain | 42 actors, 42 distinct meshes |
| `Buildings` | grandstands, bridges, the paddock | 124 actors, 51 meshes |
| `Props`, `Props_Barriers`, `Pitlane_barriers`, `Sponsors` | cones, tyres, fences and barriers; barriers are placed along splines (225 `IgnitionSplineActor`, 426 instanced-mesh components) | 755 actors; 18,218 + 28 + 282 instances |
| `Vegetation`, `Grass`, `Crowd` | trees, grass (one mesh, 17,922 instances), spectators (13 meshes, 9,227 instances) | 2,296 + 17,922 + 9,227 instances |
| `Splines`, `SplinesA`, `SplinesB`, `ForkA`, `ForkB` | the racing and track splines, borders, pit lane, respawn splines, markers, route variants (forks) | see below |
| `Cameras`, `CamerasForkA/B`, `Lighting`, `Sounds*`, `VFX`, `Decals`, `Characters` | broadcast cameras, lights, audio emitters, effects | |
| `RS_Level_1` | the riding-school course (cones, bridges, portal markers) | 7 actors |

Okayama in all: 1,061 static-mesh actors and about 48,000 instances of instanced meshes, in 26 levels; the largest level file
(`Props_Barriers`) is 35 MB of JSON. The other tracks use the same level names (the counts per track above are 17 to 36 levels).

### Static meshes and materials

An actor is a `StaticMeshActor` whose `StaticMeshComponent` names a mesh (`Env/Okayama/Grounds/Sat/SM_OKYM_SatNear_001`) and
carries `RelativeLocation`, `RelativeRotation`, `RelativeScale3D` when it is not at the origin (the terrain pieces are at the
origin: their vertices are in world space). Units are **centimetres**, z up, left-handed; CUE4Parse's glTF export is in metres.
A mesh has material slots (`MI_OKYM_Grass_Sat_Near`, `MI_OKYM_Tarmac_Sat_Near_01`, `SM_OKYM_Gravel`), each a material instance
whose parent is a layered shader (`BASE_Terrain3_4L`, `BASE_Tarmac3_Circuit`, `BASE_Kerb3_1L_ET`): parameters (tiling,
colours) and textures (colour, normal, packed `PBR`), no baked per-track diffuse. The **ground** of Okayama is:

- `Grounds/Sat/SM_OKYM_SatNear_001..012` (twelve slices, 12 meshes of 0.7 to 4.6 MB as glb) with the "water" and `_SC`
  (shadow caster) variants, `SM_OKYM_FarSat001..003` (far terrain);
- `Grounds/Terrain/SM_OKYM_Runnable` (6.5 MB) and `SM_OKYM_RunnablePaddock`: the tarmac the bikes run on, `SM_OKYM_Kerbs_001..007`
  (kerbs), `SM_OKYM_Overlay_Line*` (the painted lines and the starting grid), `SM_OKYM_Terrain` (14.7 MB).

Exported as glb at full detail: the twelve slices, the seven kerbs and the runnable mesh make **444,437 triangles** in 20
meshes (34 MB as glTF, geometry only), extent 1,541 by 2,012 m, and show the whole circuit (a 3.7 km lap, the pit lane, the
infield) in a top view (`tools/glb_merge_preview.py`). Exporting them took 1.5 s with CUE4Parse once the package index was
open. The glb is `C:\r6work\tracks\okayama_ground.glb`.

### Surface types **[verified]**

The bikes' physics read the **surface under each wheel** (`SurfaceType1` ... `SurfaceType10` in the tyre and chassis data:
[ride6-tyre.md](../specs/ride6-tyre.md), [ride6-chassis-round6.md](../specs/ride6-chassis-round6.md)). The mapping from a
mesh to a surface is the **material instance's `PhysMaterial`**: `MI_OKYM_RunnableTarmac_004` has `PM_T_Tarmac`
(`SurfaceType1`), `MI_OKYM_Kerbs_Drain` has `PM_T_MetalGrid`. The meshes' `BodySetup` is empty, so there is no collision
material on the mesh itself. A material without the override (the grass, sand and gravel instances of the ground slices)
inherits from its layered parent, where the mask-based physical material of the terrain shader is **[unconfirmed]**.
The physical materials are `assets/data/PhysicalMaterials/PM_T_*` (terrain: Tarmac, TarmacDirty, Concrete, ConcreteGrid,
Grass, GrassSynth, Gravel, LooseGravel, Sand, Kerb, MetalGrid) and `PM_P_*` (props: Bush, Concrete, Cone, Hay, Metal, Plastic,
Rock, Tyres, Wood), plus `PM_Vehicle` and the rider ragdoll ones. Each is an asset with `Restitution` and an enum
`EPhysicalSurface::SurfaceTypeN`.

## Splines **[verified]**

`IgnitionTrackSpline` actors (`Splines`, `SplinesA`, `SplinesB`, the cameras' levels) own an `IgnitionSplineComponent`
named `MainSpline` and two `IgnitionSplineComponentBase` named `LeftBorder` and `RightBorder`. A spline component holds:

| Property | Content |
|---|---|
| `SplineCurves.position.Points` | UE spline keys: `InVal` (the index), `OutVal` (x, y, z in cm), `ArriveTangent`, `LeaveTangent` |
| `SplineLength` | cm. Okayama's `TrackSpline` is 374,236 cm = 3.74 km with 44 points (closed loop); two `IdealTrajectory_1000cc_*` racing lines are 3.72 and 3.67 km with 38 and 124 points |
| `bClosedLoop`, `RelativeLocation`, `RelativeRotation` | the component's transform |
| `CurvatureR` | per-sample radius of curvature (huge values where straight) |
| `LeftBorderDistance`, `RightBorderDistance` | the width to each border per sample (cm), and `LeftDistanceOnBorder`, `RightDistanceOnBorder`, the matching distance along the border |
| `TerrainCollisionAlignment` | true: the spline is projected onto the terrain collision |
| `CollisionChannels` | which channels the spline traces against |

`IgnitionSplineMarker` (`DistanceAlongSpline`, tags such as `ForkSplineBox_1`) marks positions on a spline (start line, route
forks, sectors), `PitLaneBoxTrigger` and `PitHumanEnterTriggerBox_BP_C` the pit lane, `SlowDownRewardMarker`, `TargetPoint`
(grid and respawn points), `IgnitionSplineActor` the respawn splines (11 in Okayama) and the barrier placement
(`IgnitionSplineDecorationPlacementComponent` with `IgnitionSplineDecorationSourceData`: a pattern of meshes repeated along a
spline). The pit lane is its own track spline (`PitLaneSpline`, 951 m, with borders, 19 to 24 points). A route fork (`ForkA`,
`ForkB`) is a second set of splines, barriers and cameras switched by tags.

The splines are what a lap, an AI racing line, the respawn and the minimap are made from: a position along the track spline
(a distance) and a lateral offset with the border widths are the game's own track coordinates.

## Collision **[unconfirmed]**

The visible static meshes have default collision (`BodySetup` empty); the "runnable" meshes of the ground are the likely
contact surface, and the `TerrainCollisionAlignment` flag says splines are traced to the terrain. Whether the cooked
packages keep simplified collision data, and which meshes bikes collide with, was not read (the mesh JSON shows
`CollisionEnabled: NoCollision` only for the shadow casters). For a first version the visible `Runnable`, kerbs and slices are
the ground; barriers (`Props_Barriers`, `PM_P_*`) need their instanced meshes.

## What is not read

Textures and materials (shader graphs) of the track, lighting (`Lighting_BuiltData`), the sky and atmosphere, the audio
emitters, the crowd and grass wind, the Nanite data, the cameras' choreography, the career and event setup (which track
layout, which fork). Grass and foliage are instanced meshes (`PerInstanceSMData`, 17,922 grass instances in Okayama) that a
converter can turn into placement lists.
