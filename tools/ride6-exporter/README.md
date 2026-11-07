# RIDE 6 exporter

Our own tools that read the user's RIDE 6 install and write what `mods/ride6-bikes` imports
([docs/modding/ride6-bikes.md](../../docs/modding/ride6-bikes.md), [batch](../../docs/modding/ride6-bikes-batch.md)). The
repository holds the source only: nothing of the game (meshes, textures, JSON dumps) is ever committed, and the output goes
outside the repository (`C:\r6work`).

| File | Does |
|---|---|
| `R6ToolParts/` | the console tool (C#): opens the install with [CUE4Parse](https://github.com/FabianFG/CUE4Parse) (FabianFG, Apache-2.0, cloned outside the repository), modes `list`, `jsonmany`, `tex`, `export`, `raw`, `serve` |
| `r6lib.py` | starts the tool (`serve`), caches the JSON dump of every package on disk |
| `bindings.py` | the bindings tables `graphics/Bindings/Bike/*` (data tables: one row per bike and part) |
| `partsel.py` | which stock parts a bike has (rows, data assets, tyre choice), and why a part is missing |
| `materials.py` | material instances resolved through their parents into effective parameters |
| `liveries.py` | the `CH_<id>_Livery-NN` rows: the materials each livery changes |
| `glbtools.py`, `bake.py` | the node tree and skinning of the exported `.glb`, the anchor of a part on the chassis skeleton |
| `bikeparts.py` | one bike: `parts.json`, `glb/` (levels of detail 0 to 3), `tex/` |
| `export_all.py` | every bike, several at a time, resumable (`status.json` per bike) |
| `report.py` | the completeness report (which bike lacks which part, and the reason) |
| `gear.py`, `gear_colors.py` | the rider gear catalogue (helmets, suits, gloves, boots) with its meshes and pictures, written once into `C:6workider-gear` ([format](../../docs/formats/ride6-rider-gear.md)); needs the tool's `gmesh` and `gtex` modes |
| `rider_anims.py` | the rider animations the standstill and the hands need (the finger clips, the stand, start and stop clips of every bike type): `skeleton.json`, `bike_skeleton.json`, an `R6RA` file and `acl/`, for `ride6-rider-import --merge` |
| `physdump.py` | the physics dump `ride6-bike-import --physics` reads (inheritance resolved) |

## Setup (once)

Needs the .NET 10 SDK (a newer SDK builds it), python 3.14 with numpy and Pillow, the RIDE 6 install (`D:\RIDE 6`), and the
`.usmap` of the game (`C:\Dumper-7\5.6.1-1697004+++ride6+rel-submission-ride6\Mappings\*.usmap`, made by running the game
with a dumper; the packages use unversioned properties and cannot be read without it).

```
git clone --depth 1 https://github.com/FabianFG/CUE4Parse.git C:\r6work\tools\CUE4Parse
cd tools\ride6-exporter
set CUE4PARSE_SKIP_NATIVE=true
dotnet build R6ToolParts\R6ToolParts.csproj -c Release -o C:\r6work\tools\bin
```

The IoStore containers (`D:\RIDE 6\ride6\Content\Paks\*.utoc`) open without a key. The first run downloads the Oodle and zlib
decompressors (GitHub) next to the executable. Environment: `R6_PAKS` (the `Paks` folder), `R6_USMAP` (the mapping file),
`R6_TOOL` (the executable, default `C:\r6work\tools\bin\R6ToolParts.exe`), `R6_CACHE` (default `C:\r6work\cache`: the
JSON dumps of the packages, shared by every run). Run python with `-I`; the scripts add the user site-packages and their own
folder themselves.

## Run

```
python -I tools\ride6-exporter\export_all.py --out C:\r6work\bikes [--only <regex>] [--jobs 3] [--force] [--normals]
python -I tools\ride6-exporter\report.py --out C:\r6work\bikes
python -I tools\ride6-exporter\physdump.py --out C:\r6work\physdump-all
python -I tools\ride6-exporter\bikeparts.py <vehicle id> C:\r6work\bikes\<vehicle id>      (one bike)
```

`export_all.py` writes `C:\r6work\bikes\<vehicle id>\{parts.json, glb/, tex/, status.json}` for each of the 319 vehicles
(about 85 MB each without the normal maps, 125 MB with them: `--normals`; 10 s a bike the first time, 1.7 s with the JSON cache warm, so about 17 minutes for all 293 with 3 jobs the first time: [measured](../../docs/modding/ride6-bikes-disk.md)). A bike that has a `status.json` is
skipped unless `--force`. A vehicle whose `CHASSIS` data asset is not in the install is refused with that reason. `report.py`
writes `export-report.md` into the output folder. `physdump.py` writes `vehicles/<Brand>/<Vehicle>.json`,
`resolved/plugin/Physics/**` and `resolved/content/assets/**` (the rider's correction table and the camera containers) in about
a minute.

## Import (the lead, once cargo builds are allowed)

```
ride6-bike-import <bike folder> <mod folder> --physics C:\r6work\physdump-all --tyre-tables C:\r6work\physdump-all\resolved\plugin\Physics\Tire
```

or the native mod with `source = 'C:\r6work\bikes'` and `physics = 'C:\r6work\physdump-all'` in its `config.toml`: the
mod imports every bike folder in the background. The folder name is the vehicle id, as before.

## Tool modes

`R6ToolParts <mode> ...`; in `serve` mode each line on stdin is one request (tab separated) and the answer ends with a line
`\x01DONE`.

| Mode | Does |
|---|---|
| `list <regex> [outfile]` | the file paths of the install matching the regex |
| `jsonmany <listfile> <outdir>` | the exports of each listed package as `<outdir>/<lower-case package path>.json` |
| `tex <listfile> <outdir> <max>` | each texture as `<stem>.png` (largest mip not above `<max>` px) and `<stem>.txt` (format, size, sRGB) |
| `export "<pkg;pkg>" <outdir> Gltf2 All` | every level of detail of each skeletal or static mesh as `.glb` (`<name>.glb`, `<name>_LOD<n>.glb`) |
| `gmesh <listfile> <outdir>`, `gtex <listfile> <outdir> <max>` | skeletal meshes as `R6RM` and pictures as `R6TX` (block-compressed mips as the game stores them), for `gear.py` |
| `raw <listfile> <outdir>` | the bytes of each listed file (any extension) |
| `skeleton <package> <outfile>` | the skeleton of an animation sequence's `USkeleton` as `skeleton.json` (the rider's, [format](../../docs/formats/ride6-rider.md)) |
| `anims <listfile> <outfile.r6ra>` | each listed animation sequence that Unreal's own codecs store, sampled at every source frame, as one `R6RA` file (a sequence stored with ACL is listed as `ACL` and left out) |
| `acl <listfile> <outdir>` | each listed ACL animation sequence as `<name>.acl` (the compressed buffer) and `<name>.meta.json`; `ride6-rider-import` decodes them |
| `bp <listfile> <outdir>` | each listed Blueprint class as pseudo-C++ (`<class>.cpp`) and its functions' bytecode as `<class>.functions.txt` (the rider's animation blueprint; sets `ReadScriptData`) |

Package paths are Unreal object paths (`/Game/assets/...`, `/PhysicsRide6/Physics/...`) or the file paths `list` prints.
